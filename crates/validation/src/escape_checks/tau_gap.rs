//! `cargo xtask gate tau-gap` (TASK-M3-34): the window chord `|Δn̂|` of escapers and bound trajectories, the gap
//! between them, and the `tau` set inside it (pitfalls §2.2: "`tau` is not a tuned constant").
//!
//! **Written before the measurement.** Every quantity is read from the harness marches ([`super::march`]) replayed at
//! the register's window ([`super::replay`]), each trajectory's truth check 2's ([`super::check2::truth`]), censored
//! marches left out.
//!
//! - **The claim's statistic**, pitfalls §2.2's "7.04e-05 against 2.70e-02, stable across t = 25–30": per trajectory,
//!   the median chord over its marks in `t ∈ [25, 30]`; per class, the median of those. The ratio, bound over
//!   escapers, is reported against the claimed 383×. The statistic behind the claim is written nowhere the
//!   repository holds (prin-rs's `escape_criterion.py` computes no gap); the plan is unclear here, chose the medians
//!   because both quoted numbers read as one typical value per class.
//! - **The gap the rule decides on.** A trajectory fires at the first mark where the chord is below `tau` and the
//!   farthest body is unbound (armed). So each escaper fires by the judged time `T_h/2` exactly where `tau` exceeds its
//!   least armed chord by then ([`RunGap::fires_above`]), and each bound trajectory, or escaper whose firing would name
//!   another body, fires wrongly exactly where `tau` exceeds the least armed chord at which a wrong mark leads
//!   ([`RunGap::wrong_above`]). The gap's edges:
//!   - `U`, the false-positive edge: the least `wrong_above` over the population. Any `tau` below `U` fires on no bound
//!     trajectory (late rather than wrong, pitfalls §2.3).
//!   - `L`, the escapers' edge: the median of the escapers' `fires_above` (never-armed escapers counting as never
//!     firing). Above it, at least half the escapers fire by `T_h/2`.
//!
//!   The plan is unclear here (it names "the gap between escapers' and bound runs' |Δn̂|" without its statistic); chose
//!   these edges because `U` is the one value precision depends on, exactly, and `L` is the smallest `tau` that
//!   detects most escapes. Over several populations, `L` is the largest of theirs and `U` the smallest.
//! - **The `tau` it sets** ([`Gap::chosen`]): the geometric middle `√(L·U)` to one significant figure, required inside
//!   the middle half of the gap in `log tau`, `[L^{3/4}U^{1/4}, L^{1/4}U^{3/4}]` ([`Gap::middle`]).
//! - **Outcomes across the middle** ([`across_middle`]): each trajectory's label by its production horizon (fired or
//!   not, escaper and `detail`) at both ends of the middle half. Pass: no false positive at either end, and every label
//!   that differs is a truth escaper fired at the upper end and not yet at the lower (late rather than wrong). The
//!   count of differing labels is reported: where it is not zero, the strict reading, "outcomes unchanged", does not
//!   hold, and the report says so.
//!
//! **The gate** ([`TauGapGate`]): an input names populations and a `tau` (null for the register's); it passes where the
//! gap exists (`L < U`), the `tau` lies in its middle half, and the outcomes across the middle pass. The control is the
//! provisional `10⁻³`, below the gap.

use std::path::Path;

use serde::Deserialize;

use crate::gate::{Gate, GateConfig, GateReport, Minimum, RegionCount, Regions, Verdict};

use super::check2::{self, Truth};
use super::gate::{marched, Marched};
use super::replay::{self, Rule};
use super::spread;

/// The gate's name.
pub const NAME: &str = "tau-gap";

/// The claim's interval, canonical time (pitfalls §2.2).
pub const CLAIM_T: [f64; 2] = [25.0, 30.0];

/// The claimed separation, `2.70·10⁻² / 7.04·10⁻⁵` (pitfalls §2.2).
pub const CLAIMED_RATIO: f64 = 383.0;

/// One trajectory's place in the gap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunGap {
    /// Its truth.
    pub truth: Truth,
    /// The median chord over its marks in [`CLAIM_T`]; `None` where it has none.
    pub claim: Option<f64>,
    /// The `tau` above which it fires by the judged time, its least armed chord by then; infinite where never armed.
    pub fires_above: f64,
    /// The `tau` above which its first firing by the judged time is wrong; infinite where none can be.
    pub wrong_above: f64,
    /// The time of the mark that sets `wrong_above`.
    pub wrong_t: f64,
}

/// Whether a firing at a mark naming `body` with `detail` is wrong under `truth`.
fn wrong(truth: Truth, body: u32, detail: u32) -> bool {
    match truth {
        Truth::Bound => true,
        Truth::Escapes(k) => {
            !(detail == kernel::payload::DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION || body == k)
        }
        Truth::Censored => false,
    }
}

/// The median of `xs`, `None` where empty.
fn median(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    let mut v = xs.to_vec();
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

/// Each trajectory of `m` at `window` (the module's note).
pub fn runs(m: &Marched, window: f64) -> Vec<RunGap> {
    m.runs
        .iter()
        .zip(&m.pop.ics)
        .enumerate()
        .map(|(i, (run, ic))| {
            let truth = check2::truth(run);
            let chords = replay::chords(run, ic, window);
            let claim: Vec<f64> = chords
                .iter()
                .filter(|c| (CLAIM_T[0]..=CLAIM_T[1]).contains(&c.t))
                .map(|c| c.chord)
                .collect();
            let judged = m.pop.judged(i);
            // A mark fires first at `tau` just above its chord exactly where its chord is below every earlier armed
            // mark's: a running minimum's record.
            let (mut least, mut wrong_above, mut wrong_t) =
                (f64::INFINITY, f64::INFINITY, f64::NAN);
            for c in chords.iter().filter(|c| c.armed && c.t <= judged) {
                if c.chord < least {
                    if wrong(truth, c.body, c.detail) && c.chord < wrong_above {
                        wrong_above = c.chord;
                        wrong_t = c.t;
                    }
                    least = c.chord;
                }
            }
            RunGap {
                truth,
                claim: median(&claim),
                fires_above: least,
                wrong_above,
                wrong_t,
            }
        })
        .collect()
}

/// A population's, or several populations', gap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gap {
    /// The escapers' edge.
    pub l: f64,
    /// The false-positive edge.
    pub u: f64,
}

impl Gap {
    /// The gap of one population's trajectories; `l` infinite where it has no escaper.
    pub fn of(runs: &[RunGap]) -> Self {
        let fires: Vec<f64> = runs
            .iter()
            .filter(|r| matches!(r.truth, Truth::Escapes(_)))
            .map(|r| r.fires_above)
            .collect();
        Gap {
            l: median(&fires).unwrap_or(f64::INFINITY),
            u: runs
                .iter()
                .map(|r| r.wrong_above)
                .fold(f64::INFINITY, f64::min),
        }
    }

    /// The gap over several populations: the largest escapers' edge of those that have escapers, the least
    /// false-positive edge.
    pub fn over(gaps: &[Gap]) -> Self {
        Gap {
            l: gaps
                .iter()
                .map(|g| g.l)
                .filter(|l| l.is_finite())
                .fold(0.0, f64::max),
            u: gaps.iter().map(|g| g.u).fold(f64::INFINITY, f64::min),
        }
    }

    /// Whether there is a gap: `0 < L < U < ∞`.
    pub fn exists(&self) -> bool {
        self.l > 0.0 && self.l < self.u && self.u.is_finite()
    }

    /// `U / L`.
    pub fn ratio(&self) -> f64 {
        self.u / self.l
    }

    /// The middle half of the gap in `log tau`.
    pub fn middle(&self) -> [f64; 2] {
        [
            self.l.powf(0.75) * self.u.powf(0.25),
            self.l.powf(0.25) * self.u.powf(0.75),
        ]
    }

    /// The `tau` the gap sets: `√(L·U)` to one significant figure.
    pub fn chosen(&self) -> f64 {
        one_figure((self.l * self.u).sqrt())
    }

    /// Whether `tau` lies in the middle half.
    pub fn in_middle(&self, tau: f64) -> bool {
        let [a, b] = self.middle();
        a <= tau && tau <= b
    }
}

/// `x` to one significant figure.
pub fn one_figure(x: f64) -> f64 {
    let e = x.log10().floor();
    let m = (x / 10f64.powf(e)).round();
    // Written as the decimal it reads, so the value is the literal's nearest double.
    format!("{m}e{e}").parse().unwrap_or(f64::NAN)
}

/// One trajectory's label by its production horizon: its firing's escaper and `detail`, or none.
fn label(run: &crate::escape_checks::march::PostEscape, horizon: f64) -> Option<(u32, u32)> {
    run.firing
        .filter(|f| f.t <= horizon)
        .map(|f| (f.escaper, f.detail))
}

/// The outcomes across the middle half of `gap` on `m` (the module's note): the labels that differ between its ends,
/// those that are a truth escaper gained at the upper end, and the false positives at each end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Across {
    /// Trajectories whose label differs.
    pub changed: usize,
    /// Of those, a truth escaper fired at the upper end and not at the lower, naming its escaper.
    pub gained: usize,
    /// False positives by the judged time at the lower end.
    pub fp_low: usize,
    /// False positives by the judged time at the upper end.
    pub fp_high: usize,
}

impl Across {
    /// The pass criterion (the module's note).
    pub fn passes(&self) -> bool {
        self.fp_low == 0 && self.fp_high == 0 && self.changed == self.gained
    }
}

/// [`Across`] on `m` between `lo` and `hi`, at `window`.
pub fn across_middle(m: &Marched, lo: f64, hi: f64, window: f64) -> Across {
    let under = |tau| replay::all_under(&m.runs, &m.pop.ics, Rule { tau, window });
    let (a, b) = (under(lo), under(hi));
    let fp = |runs: &[_]| check2::tally(&check2::score(&m.pop, runs), false).fp;
    let mut x = Across {
        fp_low: fp(&a),
        fp_high: fp(&b),
        ..Across::default()
    };
    for (i, (ra, rb)) in a.iter().zip(&b).enumerate() {
        let h = m.pop.horizon(i);
        let (la, lb) = (label(ra, h), label(rb, h));
        if la != lb {
            x.changed += 1;
            let truth = check2::truth(ra);
            if la.is_none()
                && lb
                    .is_some_and(|(e, d)| matches!(truth, Truth::Escapes(_)) && !wrong(truth, e, d))
            {
                x.gained += 1;
            }
        }
    }
    x
}

/// One population's report lines at `window`: the claim's statistic and the gap's edges with their distributions.
pub fn report(m: &Marched, rg: &[RunGap]) -> String {
    let pick = |esc: bool, f: &dyn Fn(&RunGap) -> Option<f64>| -> Vec<f64> {
        rg.iter()
            .filter(|r| {
                if esc {
                    matches!(r.truth, Truth::Escapes(_))
                } else {
                    r.truth == Truth::Bound
                }
            })
            .filter_map(f)
            .collect()
    };
    let (ce, cb) = (pick(true, &|r| r.claim), pick(false, &|r| r.claim));
    let ratio = match (median(&ce), median(&cb)) {
        (Some(e), Some(b)) => format!("{:.3e} against {:.3e}: {:.2}×", e, b, b / e),
        _ => "none (a class is empty)".into(),
    };
    let finite = |v: Vec<f64>| v.into_iter().filter(|x| x.is_finite()).collect::<Vec<_>>();
    let g = Gap::of(rg);
    let censored = rg.iter().filter(|r| r.truth == Truth::Censored).count();
    let edge = edge_line(m, rg);
    format!(
        "{} ({} trajectories, {censored} censored):\n\
         - the claim's statistic, per-trajectory median |Δn̂| over t ∈ [25, 30]: escapers {}; bound {}\n\
         - its medians, escapers against bound (claimed 7.04e-05 against 2.70e-02, 383×): {ratio}\n\
         - escapers' least armed chord by T_h/2 (the tau above which each fires; never armed left out): {}\n\
         - bound trajectories' least armed chord by T_h/2 (never armed left out): {}\n\
         - wrong-firing edges (the tau above which each fires wrongly): {}\n\
         - L (escapers' edge) {:.4e}, U (false-positive edge) {:.4e}, U/L {:.3}\n\
         - {edge}",
        m.pop.name,
        rg.len(),
        spread(&ce),
        spread(&cb),
        spread(&finite(pick(true, &|r| Some(r.fires_above)))),
        spread(&finite(pick(false, &|r| Some(r.fires_above)))),
        spread(&finite(rg.iter().map(|r| r.wrong_above).collect())),
        g.l,
        g.u,
        g.ratio()
    )
}

/// The trajectory that sets the false-positive edge, described: its truth and why, the mark, and its farthest body's
/// distance and `E_rel` over the ground truth's segment.
fn edge_line(m: &Marched, rg: &[RunGap]) -> String {
    let Some((i, r)) = rg
        .iter()
        .enumerate()
        .filter(|(_, r)| r.wrong_above.is_finite())
        .min_by(|a, b| a.1.wrong_above.total_cmp(&b.1.wrong_above))
    else {
        return "the false-positive edge: no trajectory fires wrongly at any tau".into();
    };
    let run = &m.runs[i];
    let why = match check2::growth(run) {
        Some(g) => format!(
            "farthest body {} at T_h, farthest throughout the segment {}, increasing {}, growth ratio {:.4} (ρ {:.4})",
            g.body,
            g.farthest,
            g.increasing,
            g.ratio,
            check2::RHO
        ),
        None => "too few points".into(),
    };
    let at = |t: f64| {
        run.points
            .iter()
            .find(|p| p.t >= t)
            .or(run.points.last())
            .map(|p| {
                let k = p.candidate.body as usize;
                format!(
                    "t {:.2}: body {k} d {:.3} E_rel {:+.3e}",
                    p.t, p.d[k], p.e_rel[k]
                )
            })
            .unwrap_or_default()
    };
    let t_h = run.t_end;
    format!(
        "the false-positive edge is set by pixel {i}: truth {:?} ({why}); its wrong mark at t {:.3}, chord {:.4e}; \
         {}; {}; {}; {}",
        r.truth,
        r.wrong_t,
        r.wrong_above,
        at(r.wrong_t),
        at(t_h / 2.0),
        at(0.75 * t_h),
        at(t_h)
    )
}

/// One population an input names.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slice {
    /// Its name (`config_stability`, `config_basin`, `deep_interior`).
    pub slice: String,
    /// Pixels per side.
    pub n: usize,
    /// Its collision radius, in its units; null for its own.
    #[serde(default)]
    pub r_coll: Option<f64>,
}

impl Slice {
    /// Its marches.
    pub fn marched(&self) -> Result<std::sync::Arc<Marched>, String> {
        marched(&self.slice, self.n, self.r_coll)
    }
}

/// An input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    description: String,
    populations: Vec<Slice>,
    /// The `tau` judged; null for the register's.
    tau: Option<f64>,
}

/// The measured gap over `pops` at `window`, with each population's report and its own gap.
pub fn measure(pops: &[std::sync::Arc<Marched>], window: f64) -> (Gap, Vec<(Gap, String)>) {
    let each: Vec<(Gap, String)> = pops
        .iter()
        .map(|m| {
            let rg = runs(m, window);
            (Gap::of(&rg), report(m, &rg))
        })
        .collect();
    let gaps: Vec<Gap> = each.iter().map(|(g, _)| *g).collect();
    (Gap::over(&gaps), each)
}

/// `tau-gap` (the module's note).
pub struct TauGapGate;

impl Gate for TauGapGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = Input::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
        if input.populations.is_empty() {
            return Err(format!("refused: {name}: no population"));
        }
        let pops = input
            .populations
            .iter()
            .map(Slice::marched)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("refused: {name}: {e}"))?;
        let window = Rule::register().window;
        let tau = input.tau.unwrap_or(Rule::register().tau);
        let (gap, each) = measure(&pops, window);
        let [lo, hi] = gap.middle();
        let mut scatter: Vec<String> = each.iter().map(|(_, r)| r.clone()).collect();
        let mut negative = Vec::new();
        let mut pass = gap.exists();
        negative.push(format!(
            "the gap over the populations: L {:.4e}, U {:.4e}, U/L {:.3}; it {} (L < U)",
            gap.l,
            gap.u,
            gap.ratio(),
            if gap.exists() {
                "exists"
            } else {
                "DOES NOT exist"
            }
        ));
        let inside = gap.exists() && gap.in_middle(tau);
        pass &= inside;
        negative.push(format!(
            "tau {tau:e} {} the middle half [{lo:.4e}, {hi:.4e}]; the gap's own choice √(L·U) to one figure: {:e}",
            if inside { "lies in" } else { "is OUTSIDE" },
            gap.chosen()
        ));
        if gap.exists() {
            for m in &pops {
                let x = across_middle(m, lo, hi, window);
                pass &= x.passes();
                scatter.push(format!(
                    "{}: across the middle half, {} label(s) by the production horizon differ, {} of them a truth \
                     escaper fired at the upper end only; false positives {} at the lower end, {} at the upper; \
                     strictly unchanged: {}",
                    m.pop.name,
                    x.changed,
                    x.gained,
                    x.fp_low,
                    x.fp_high,
                    x.changed == 0
                ));
                negative.push(format!(
                    "{}: across the middle, no false positive and only escapes gained: {}",
                    m.pop.name,
                    if x.passes() { "met" } else { "NOT met" }
                ));
            }
        }
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(pops.len() as u32),
                minimum: Minimum::NotCalibrated {
                    requirement: config.threshold.requirement.clone(),
                },
            }),
            negative_results: negative,
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => {
                println!("{}", super::revalidation::propose_tau()?);
                Ok(())
            }
            _ => Err(format!(
                "tau-gap: unrecognised flags `{}`; use `--propose`",
                args.join(" ")
            )),
        }
    }
}
