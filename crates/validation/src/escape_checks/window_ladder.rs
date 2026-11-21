//! `cargo xtask gate escape-window-ladder` (TASK-M3-34): the escape window over a ladder holding 0.4, on the
//! re-validation set (REQ-EVT-033).
//!
//! **Written before the measurement.** At each rung `w` the rule is replayed at the register's `tau` and window `w` on
//! the same marches ([`super::replay`]; one march for every rung), and recorded: precision and recall against check 2,
//! the tau gap at `w` ([`super::tau_gap`]), how many trajectories' labels by their production horizon differ from the
//! 0.4 rung's, and the Burrau anchor's firing.
//!
//! **The pass criterion of a rung**: no firing whose check-2 truth is bound or another body's escape (at most the
//! gate's threshold, 0, `escape-precision`'s criterion: late rather than wrong, pitfalls §2.3), over at least
//! [`check2::MIN_DECIDED_FIRINGS`] decided firings: the one property a window must not change. Recall and the label
//! changes are recorded, not judged. The criterion was first written as precision ≥ 0.99; at that the control passed
//! (one false positive over 748 firings, 0.9987), so it could not fail, and it became `escape-precision`'s, set before
//! the ladder ran; no rung's verdict changed. **The control**: a rung far below the ladder, one sync quantum (0.125,
//! every sampling point a mark), at which the chord of a turning inner binary is a fraction of the 0.4 window's, so
//! transients settle in it and it must fail.
//!
//! **The range a carried window may take** ([`propose`]): the rungs about 0.4 at which the criterion holds, contiguous
//! with it.

use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;

use crate::gate::{Gate, GateConfig, GateReport, RegionCount, Regions, Verdict};

use super::check2::{self, Tally};
use super::gate::Marched;
use super::replay::Rule;
use super::revalidation::{self, burrau_firing, firings, revalidation_slices, tally_line, Scored};
use super::tau_gap::{self, Slice};

/// The gate's name.
pub const NAME: &str = "escape-window-ladder";

/// The ladder, physical time: 0.4 held, a factor of two each way (the rungs are the task's to propose, R-698).
pub const LADDER: [f64; 5] = [0.2, 0.4, 0.8, 1.6, 3.2];

/// One rung measured.
#[derive(Clone, Debug)]
pub struct Rung {
    /// The window.
    pub window: f64,
    /// Check 2's confusion, pooled over the set.
    pub tally: Tally,
    /// The pooled gap at this window.
    pub gap: tau_gap::Gap,
    /// Trajectories whose label by the production horizon differs from the 0.4 rung's.
    pub changed: usize,
    /// Trajectories in the set.
    pub n: usize,
    /// The Burrau firing, Szebehely & Peters' units, and its escaper.
    pub burrau: Option<(f64, u32)>,
    /// Each population's report lines.
    pub lines: Vec<String>,
}

impl Rung {
    /// The pass criterion (the module's note): at most `wrong` wrong firings over enough decided ones.
    pub fn verdict(&self, wrong: f64) -> Result<String, String> {
        let decided = self.tally.tp + self.tally.fp;
        let line = format!(
            "{} wrong firing(s) (criterion ≤ {wrong}) over {decided} decided firings (≥ {}), precision {}",
            self.tally.fp,
            check2::MIN_DECIDED_FIRINGS,
            self.tally
                .precision()
                .map_or("none".into(), |p| format!("{p:.4}"))
        );
        if decided >= check2::MIN_DECIDED_FIRINGS && self.tally.fp as f64 <= wrong {
            Ok(line)
        } else {
            Err(line)
        }
    }
}

/// The label of each trajectory of `m` by its production horizon under `rule`.
fn labels(m: &Marched, rule: Rule) -> Vec<Option<(u32, u32)>> {
    firings(m, Scored::Replay(rule))
        .iter()
        .enumerate()
        .map(|(i, r)| {
            r.firing
                .filter(|f| f.t <= m.pop.horizon(i))
                .map(|f| (f.escaper, f.detail))
        })
        .collect()
}

/// The rung at `window` on `pops`.
pub fn rung(pops: &[Arc<Marched>], window: f64) -> Rung {
    let tau = Rule::register().tau;
    let rule = Rule { tau, window };
    let base = Rule::register();
    let (mut tallies, mut lines, mut changed, mut n) = (Vec::new(), Vec::new(), 0, 0);
    for m in pops {
        let runs = firings(m, Scored::Replay(rule));
        let t = check2::tally(&check2::score(&m.pop, &runs), false);
        lines.push(format!("{}: {}", m.pop.name, tally_line(&t)));
        tallies.push(t);
        let (a, b) = (labels(m, rule), labels(m, base));
        changed += a.iter().zip(&b).filter(|(x, y)| x != y).count();
        n += a.len();
    }
    let (gap, _) = tau_gap::measure(pops, window);
    Rung {
        window,
        tally: revalidation::sum(&tallies),
        gap,
        changed,
        n,
        burrau: burrau_firing(rule),
        lines,
    }
}

/// A rung's report line.
pub fn line(r: &Rung) -> String {
    format!(
        "window {}: {}; gap L {:.4e} U {:.4e} (U/L {:.3}, {}); labels by the production horizon differing from the 0.4 \
         rung's: {} of {}; Burrau fires at {:?} (S&P units, escaper)",
        r.window,
        tally_line(&r.tally),
        r.gap.l,
        r.gap.u,
        r.gap.ratio(),
        if r.gap.exists() { "exists" } else { "none" },
        r.changed,
        r.n,
        r.burrau
    )
}

/// An input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    description: String,
    populations: Vec<Slice>,
    window: f64,
}

/// `escape-window-ladder` (the module's note).
pub struct EscapeWindowLadderGate;

impl Gate for EscapeWindowLadderGate {
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
        if !(input.window.is_finite() && input.window > 0.0) || input.populations.is_empty() {
            return Err(format!(
                "refused: {name}: a window and a population are needed"
            ));
        }
        let pops = input
            .populations
            .iter()
            .map(Slice::marched)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("refused: {name}: {e}"))?;
        let r = rung(&pops, input.window);
        let verdict = r.verdict(config.threshold.value);
        let mut scatter = vec![line(&r)];
        scatter.extend(r.lines.iter().cloned());
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if verdict.is_ok() {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded((r.tally.tp + r.tally.fp) as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: vec![match verdict {
                Ok(l) => format!("{l}: met"),
                Err(l) => format!("{l}: not met"),
            }],
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => {
                println!("{}", propose(0.0)?);
                Ok(())
            }
            _ => Err(format!(
                "{NAME}: unrecognised flags `{}`; use `--propose`",
                args.join(" ")
            )),
        }
    }
}

/// `--propose` (REQ-EVT-033): every rung and the control's, the window and the range.
pub fn propose(wrong: f64) -> Result<String, String> {
    let pops = revalidation_slices()
        .iter()
        .map(Slice::marched)
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = format!(
        "REQ-EVT-033, the escape window and the range a carried window may take, from the ladder {LADDER:?} at the \
         register's tau {} on the re-validation set (check 2, judged firings by T_h/2; criterion: at most {wrong} \
         wrong firings over ≥ {} decided firings):\n",
        Rule::register().tau,
        check2::MIN_DECIDED_FIRINGS
    );
    let rungs: Vec<Rung> = std::iter::once(0.125)
        .chain(LADDER)
        .map(|w| rung(&pops, w))
        .collect();
    for r in &rungs {
        out += &format!(
            "- {}: {}\n",
            line(r),
            match r.verdict(wrong) {
                Ok(_) => "criterion met",
                Err(_) => "criterion NOT met",
            }
        );
    }
    let ok: Vec<f64> = rungs
        .iter()
        .filter(|r| r.window >= LADDER[0] && r.verdict(wrong).is_ok())
        .map(|r| r.window)
        .collect();
    let held = ok.contains(&0.4);
    let (mut lo, mut hi) = (0.4, 0.4);
    for w in LADDER.iter().rev().filter(|&&w| w < 0.4) {
        if ok.contains(w) && lo / 2.0 <= *w * 1.0001 {
            lo = *w;
        }
    }
    for w in LADDER.iter().filter(|&&w| w > 0.4) {
        if ok.contains(w) && *w <= hi * 2.0001 {
            hi = *w;
        }
    }
    out += &format!(
        "\nThe window: 0.4 {}. The range a carried window may take, the ladder's rungs contiguous with 0.4 that meet the \
         criterion: [{lo}, {hi}]. R-610's horizon floor is one window: it follows the window if the value moves.\n",
        if held { "meets the criterion" } else { "does NOT meet the criterion" }
    );
    Ok(out)
}
