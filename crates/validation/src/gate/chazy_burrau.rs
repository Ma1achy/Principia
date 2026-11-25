//! `cargo xtask gate chazy-burrau` (chazy drill-down §12.2 item 1): the Burrau start classified at every
//! step of its march to a horizon past its escape, in the validation harness. Blocking: an internal correctness check
//! of the classifier's energy certificate and of its final class on the anchor.
//!
//! **The run.** The classical Burrau rest start (`tiers::burrau`) through Path A's CHECK 1 to the canonical gauge, as
//! SP-1 starts it (`super::burrau`), marched by the default occupant (Heggie × RK4, [`DefaultOccupant`]) at f64 to
//! [`BURRAU_HORIZON`] of Szebehely & Peters' units, past the escape the default rule first labels at 77.22 (measured,
//! `super::burrau`'s note). The production loop stops at escape fire only under the stop option; this march is
//! the post-escape march, the stop option off, so the label goes on to the horizon. It runs at the benchmark's pinned
//! `r_coll` ([`BENCHMARK_R_COLL`], 0: every encounter regularised through), which the report records; an input may name
//! another `r_coll`, the control's.
//!
//! **Classified every step**: after each internal step of the march, the label the read side gives
//! (`kernel::chazy::chazy_class`) on that step's state, the run's `E_0` and `R_0` and the latch's stored level then.
//!
//! **The criterion**, written before the first run: the march does not end in collision (a collision at the input's
//! `r_coll` fails and is reported); no step reads H, HP or P (`E < 0`: the rule's `E ≥ 0` guard, §2); the last step's
//! label is HE; and the last step's escaper (`chazy_escaper`) is the lightest body, Szebehely & Peters' escaper.

use kernel::chazy::{chazy_class, chazy_escaper, Motion};
use kernel::decode::Ic;
use kernel::detect::label::Fired;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::integrate_with;
use kernel::generated::links::Defaults;
use kernel::occupant::{Geom, State};
use kernel::payload::{sd_state, SimStateBaseOf, STATE_COLLISION};
use serde::Deserialize;

use super::{Gate, GateConfig, GateReport, RegionCount, Regions, Verdict};
use crate::ground_truth::tiers::{self, BURRAU_HORIZON};
use crate::ground_truth::{
    burrau_time_scale, march_key, path_a, DefaultOccupant, BENCHMARK_R_COLL,
};
use crate::import::unit_state;

/// The gate's name.
pub const NAME: &str = "chazy-burrau";

/// One input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    description: String,
    /// The collision radius, canonical units; the benchmark's pin when absent.
    #[serde(default)]
    r_coll: Option<f64>,
}

/// A cadence around the wrapper's own that classifies the state after every internal step.
struct Classifying {
    inner: WrapperCadence<f64>,
    m: [f64; 3],
    e_0: f64,
    r_0: f64,
    /// The label after each step, and the step's escaper.
    labels: Vec<(Motion, u32)>,
}

impl Cadence<f64> for Classifying {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.inner.after_step(state, geom);
        let code = chazy_class(
            self.m,
            state.r,
            state.p,
            self.e_0,
            self.r_0,
            self.inner.i_max().code(),
        );
        let label = Motion::from_code(code).unwrap_or(Motion::BSoFar);
        self.labels.push((label, chazy_escaper(self.m, state.r)));
    }
    fn done(&self) -> bool {
        Cadence::done(&self.inner)
    }
    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.inner.sample(state, index, quantum);
    }
    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }
    fn regularised_drift(&mut self, drift: f64) {
        self.inner.regularised_drift(drift);
    }
    fn bucket_d2(&self, r: &[[f64; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }
    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }
    fn resume(&mut self, stored: &SimStateBaseOf<f64>) {
        self.inner.resume(stored);
    }
    fn latched(&mut self) {
        self.inner.latched();
    }
    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }
    fn failure(&self) -> Fired {
        self.inner.failure()
    }
    fn finish(&self, out: SimStateBaseOf<f64>) -> SimStateBaseOf<f64> {
        self.inner.finish(out)
    }
}

/// The run: every step's label, the stored end state, and what it ran under.
pub struct Run {
    /// The collision radius, canonical units.
    pub r_coll: f64,
    /// The horizon, S&P's units.
    pub horizon: f64,
    /// The macro-step, canonical units.
    pub dt_macro: f64,
    /// Every step's label and escaper.
    pub labels: Vec<(Motion, u32)>,
    /// The stored end state.
    pub end: SimStateBaseOf<f64>,
    /// The lightest body.
    pub lightest: usize,
}

impl Run {
    /// The criterion (the module's note); `Err` naming what failed.
    pub fn verdict(&self) -> Result<(), String> {
        if sd_state(self.end.packed_a) == STATE_COLLISION {
            return Err(format!(
                "the Burrau march ends in collision at r_coll = {:e}, after {} steps: no final motion to classify",
                self.r_coll,
                self.labels.len()
            ));
        }
        if let Some((k, (l, _))) = self
            .labels
            .iter()
            .enumerate()
            .find(|(_, (l, _))| matches!(l, Motion::H | Motion::Hp | Motion::P))
        {
            return Err(format!(
                "step {k} of {} reads {} at E < 0",
                self.labels.len(),
                l.name()
            ));
        }
        let Some(&(last, escaper)) = self.labels.last() else {
            return Err("the march took no step".to_owned());
        };
        if last != Motion::He {
            return Err(format!("the last step reads {}, not HE", last.name()));
        }
        if escaper as usize != self.lightest {
            return Err(format!(
                "the last step's escaper is body {escaper}, not the lightest, body {}",
                self.lightest
            ));
        }
        Ok(())
    }

    /// From which step the label is HE to the end, as a measurement for the report.
    fn settled(&self) -> String {
        match self.labels.iter().rposition(|(l, _)| *l != Motion::He) {
            None => format!(" HE at every one of {} steps", self.labels.len()),
            Some(k) if k + 1 == self.labels.len() => " not HE at the last step".to_owned(),
            Some(k) => format!(" HE from step {} of {}", k + 1, self.labels.len()),
        }
    }

    /// The report's lines.
    pub fn lines(&self) -> Vec<String> {
        let mut counts = std::collections::BTreeMap::new();
        for (l, _) in &self.labels {
            *counts.entry(l.name()).or_insert(0usize) += 1;
        }
        let last = self.labels.last().map(|(l, e)| (l.name(), *e));
        let mut runs: Vec<(&str, usize)> = Vec::new();
        for (l, _) in &self.labels {
            match runs.last_mut() {
                Some((name, n)) if *name == l.name() => *n += 1,
                _ => runs.push((l.name(), 1)),
            }
        }
        vec![
            format!(
                "Burrau through Path A, Heggie × RK4 at f64, stop option off, to {} of S&P's units, dt_macro {:e}, \
                 r_coll {:e} (the benchmark's pin is {BENCHMARK_R_COLL:e})",
                self.horizon, self.dt_macro, self.r_coll
            ),
            format!(
                "end state {}, {} steps classified, the last {last:?} (label, escaper; the lightest is body {})",
                sd_state(self.end.packed_a),
                self.labels.len(),
                self.lightest
            ),
            format!("labels over the steps: {counts:?}"),
            format!(
                "the label's runs, in order (label, steps): {:?}{}",
                &runs[..runs.len().min(40)],
                if runs.len() > 40 { " …" } else { "" }
            ),
            format!("settling:{}", self.settled()),
        ]
    }
}

/// The Burrau start classified at every step to [`BURRAU_HORIZON`] at `r_coll`.
pub fn run(r_coll: f64) -> Result<Run, String> {
    let start = tiers::burrau();
    let x = unit_state(&start)?;
    let scale = burrau_time_scale();
    let c = path_a::check1::<Defaults>(&x);
    let ic: Ic<f64> = c.decoded;
    let key = march_key(BURRAU_HORIZON * scale, r_coll).map_err(|e| format!("{e:?}"))?;
    let e_0 = energy(&ic);
    let total: f64 = ic.m.iter().sum();
    let r_0 = (kernel::chazy::inertia(&ic.m, &ic.r) / total).sqrt();
    let mut cadence = Classifying {
        inner: WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, key.radius),
        m: ic.m,
        e_0,
        r_0,
        labels: Vec::new(),
    };
    let out = integrate_with::<DefaultOccupant, f64, _>(&ic, &key.loop_values, &mut cadence);
    let end = cadence.finish(out);
    Ok(Run {
        r_coll,
        horizon: BURRAU_HORIZON,
        dt_macro: key.loop_values.dt_macro,
        labels: cadence.labels,
        end,
        lightest: tiers::lightest(start.m),
    })
}

/// The energy `Σ |p|²/2m − Σ m_i m_j / d_ij` of `ic`, written here.
fn energy(ic: &Ic<f64>) -> f64 {
    let k: f64 = (0..3)
        .map(|n| (ic.p[n][0].powi(2) + ic.p[n][1].powi(2)) / (2.0 * ic.m[n]))
        .sum();
    let v: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            let d = (ic.r[i][0] - ic.r[j][0]).hypot(ic.r[i][1] - ic.r[j][1]);
            ic.m[i] * ic.m[j] / d
        })
        .sum();
    k - v
}

/// The gate.
pub struct ChazyBurrauGate;

impl Gate for ChazyBurrauGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input: Input =
            serde_json::from_value(fixture.clone()).map_err(|e| format!("{name}: {e}"))?;
        if config.threshold.value != 0.0 {
            return Err(format!(
                "refused: gate.json's threshold {} is not 0, the steps reading H, HP or P a pass allows",
                config.threshold.value
            ));
        }
        let r = run(input.r_coll.unwrap_or(BENCHMARK_R_COLL))?;
        let verdict = r.verdict();
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
            scatter: r.lines(),
            regions: Some(Regions {
                count: RegionCount::Recorded(1),
                minimum: config.min_regions.clone(),
            }),
            negative_results: vec![match verdict {
                Ok(()) => "none: the last label HE, no H, HP or P at any step".to_owned(),
                Err(e) => e,
            }],
        })
    }
}
