//! `symplectic-drift` (dd_integrator §5 test 2; REQ-VAL-031, its tolerance REQ-VAL-139's): on the long bounded orbit a
//! symplectic occupant's energy error oscillates about an offset with no secular trend, while RK4's and Euler's drift
//! secularly, and Euler's drift lights `SUSPECT_ENERGY`, the pass condition of the debug occupant.
//!
//! **The fixture** is test 2's, [`crate::drift::figure_eight`]: the figure-eight in the sim's units, canonicalised, run
//! to the horizon cap `T = 200` at the default sim key ([`crate::drift::sim_key`]), through the one wrapper
//! (`integrate_with`) and its cadence, at f64. A [`Recording`] cadence wraps the wrapper's own and passes every call on
//! unchanged, so the run is the one the drift view stores (the tests check it bitwise); it reads the relative energy
//! error `δ(t) = (E(t) − E₀)/|E₀|` at every completed macro-step, `E₀` the IC's energy at f64.
//!
//! **The statistic** ([`trend`]): `δ(t)` fitted by least squares to `a + s·t` over the run, with an intercept, since a
//! symplectic error oscillates about an offset and not about zero. The trend over the run is `|s|·T_run`; the
//! oscillation is the largest residual from the line, `A`. Their ratio `ρ = |s|·T_run / A` reads
//! - **bounded** at most the threshold: a trend smaller than the oscillation it sits in;
//! - **secular** above it: a trend the oscillation does not explain.
//!
//! **The threshold from first principles** (written before any run; REQ-VAL-139, provisional). For
//! `δ = A sin(ωt + φ)` over `[0, T]` the least-squares slope is `s = (12/T³)∫(t − T/2)·A sin(ωt + φ) dt`, and by parts
//! the integral is at most `T/ω + 2/ω²` in size, so `|s|·T ≤ 12A·(1/(ωT) + 2/(ωT)²)`. The figure-eight's period in
//! the sim's units is about 14.8 (6.326 in `G = mᵢ = 1`, times `√3` for the masses `1/3` at the same lengths and
//! shape, times `λ^{3/2} ≈ 1.355` for canonicalisation's length scale `λ = 1/√I ≈ 1.225`), so `T = 200` holds
//! `N ≈ 13.5` periods, `ωT = 2πN ≈ 85`, and an error oscillating at the orbit's period, or at any harmonic of it, gives
//! `ρ ≤ 0.15`. A pure drift has no residual and `ρ` without bound; drift and oscillation of equal size over the run give
//! `ρ ≈ 1`. The threshold is 0.25: above the oscillation's bound, well below a drift's. One threshold judges both
//! claims, so RK4 and Euler must read above the same value the symplectic occupants read below.
//!
//! **Resolvability** ([`ROUNDING_FACTOR`]): at f64, accumulated rounding is a random walk of about `ε√steps` in `δ`,
//! and a random walk reads as a trend. An input whose largest `|δ|` is below `100·ε·√steps` is refused, neither passed
//! nor failed: its error is rounding's, not the occupant's, and judges no claim.
//!
//! **Measured:** at the default key Yoshida-4 (largest `|δ|` 2.0·10⁻¹²), Yoshida-6 (1.1·10⁻¹³) and RK4 (1.4·10⁻¹²) are
//! all below the bound (5.7·10⁻¹² at 65,535 steps), and Yoshida-6's rounding reads `ρ = 2.3`, a false secular trend. So
//! the default-key inputs of those three are refused, and each is also judged with only `dt_macro` changed, ×4 for
//! Yoshida-4 and RK4 and ×8 for Yoshida-6, where its truncation error stands above rounding (`dt_scale`).
//!
//! **`SUSPECT_ENERGY`** is read as the drift view reads it: the read side's `suspect_energy` over the stored `dE_max`
//! latch and the stored `E₀` (rounded to f32, as the read side holds it) at the sim key's floor `ε_E = 10⁻⁶`. An input
//! may claim it lit or unlit; Euler's claim that it is lit is the pass condition.
//!
//! **Domain** (pitfalls §10): one orbit, one sim key (or the input's `dt_scale` times its `dt_macro`), f64. A run that
//! ends before the horizon (a terminal fired) is fitted over the steps it completed, and the report says so.
//!
//! The plan is unclear here; chose `fixtures/gates/symplectic-drift/` for the fixture directory the task calls
//! `long-bounded/`, because the gate runner reads a gate's inputs from the directory of its name; the orbit itself is
//! test 2's, already defined in `crate::drift`.

use serde::Deserialize;

use kernel::decode::Ic;
use kernel::detect::label::Fired;
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::substep::Quantise;
use kernel::driver::wrapper::{integrate_with, monitored};
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{CapabilityProfile, Geom, State, Stepper};
use kernel::payload::{
    pb_dE_max, sd_state, suspect_energy, tm_t_end_step, PayloadReal, SimStateBaseOf,
};
use kernel::physics::energy::energy;
use kernel::physics::pairs::PairGeom;

use crate::drift::{figure_eight, sim_key, Fixture, EPS_DEFAULT};
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};

/// The gate's name.
pub const NAME: &str = "symplectic-drift";

/// The resolvability factor: an input whose largest `|δ|` is below `ROUNDING_FACTOR·ε·√steps` is refused (the module's
/// note).
pub const ROUNDING_FACTOR: f64 = 100.0;

/// A cadence that passes every call to `inner` unchanged and records the relative energy error at each completed
/// macro-step (the module's note).
pub struct Recording<'a, C> {
    inner: &'a mut C,
    e0: f64,
    dt: f64,
    /// The last relative error a step left, not yet recorded.
    last: Option<f64>,
    /// `(t, δ)` at each completed macro-step.
    pub series: Vec<(f64, f64)>,
    /// Whether [`Cadence::clock`] is passed on: always, but for the tests' control, which shows the check that the
    /// recorder is passive can fail.
    #[doc(hidden)]
    pub forward_clock: bool,
}

impl<'a, C> Recording<'a, C> {
    /// A recorder around `inner`, from energy `e0`, on the macro-step `dt`.
    pub fn new(inner: &'a mut C, e0: f64, dt: f64) -> Self {
        Recording {
            inner,
            e0,
            dt,
            last: None,
            series: Vec::new(),
            forward_clock: true,
        }
    }
}

impl<C: Cadence<f64>> Cadence<f64> for Recording<'_, C> {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.inner.after_step(state, geom);
        self.last = Some((energy(&state.m, &state.p, &geom.pair) - self.e0) / self.e0.abs());
    }

    fn done(&self) -> bool {
        self.inner.done()
    }

    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.inner.sample(state, index, quantum);
    }

    fn bucket_d2(&self, r: &[[f64; 2]; 3]) -> f32
    where
        f64: Quantise,
    {
        self.inner.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        if let Some(d) = self.last.take() {
            self.series.push((f64::from(completed) * self.dt, d));
        }
        if self.forward_clock {
            self.inner.clock(completed);
        }
    }

    fn resume(&mut self, stored: &SimStateBaseOf<f64>)
    where
        f64: PayloadReal,
    {
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

    fn finish(&self, out: SimStateBaseOf<f64>) -> SimStateBaseOf<f64>
    where
        f64: PayloadReal,
    {
        self.inner.finish(out)
    }
}

/// A recorded run: the stored state as the wrapper and its monitor leave it, and the energy-error series.
#[derive(Clone, Debug)]
pub struct Recorded {
    /// The stored state, its monitored fields written.
    pub out: SimStateBaseOf<f64>,
    /// `(t, δ)` at each completed macro-step.
    pub series: Vec<(f64, f64)>,
    /// The macro-step.
    pub dt: f64,
    /// The horizon in macro-steps.
    pub horizon: u32,
}

/// The IC's energy at f64.
pub fn ic_energy(ic: &Ic<f64>) -> f64 {
    energy(&ic.m, &ic.p, &PairGeom::new(&ic.r))
}

/// The run of the occupant `A` from `fixture` at its sim key with `dt_macro` scaled by `dt_scale` (and the horizon's
/// step count with it), through the wrapper and its cadence, recorded. `forward_clock` is the recorder's (its note).
pub fn record<A: Advance>(fixture: &Fixture, dt_scale: f64, forward_clock: bool) -> Recorded {
    let ic = fixture.ic;
    let mut key = sim_key::<f64>(fixture.horizon);
    key.dt_macro *= dt_scale;
    key.horizon_steps = (fixture.horizon / key.dt_macro).ceil() as u32;
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let (raw, series) = {
        let mut rec = Recording::new(&mut cadence, ic_energy(&ic), key.dt_macro);
        rec.forward_clock = forward_clock;
        let raw = integrate_with::<A, f64, _>(&ic, &key, &mut rec);
        (raw, rec.series)
    };
    Recorded {
        out: monitored(raw, &cadence),
        series,
        dt: key.dt_macro,
        horizon: key.horizon_steps,
    }
}

/// The trend statistic of a series (the module's note).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trend {
    /// The fitted slope `s`.
    pub slope: f64,
    /// The fitted intercept.
    pub offset: f64,
    /// The run's length `T_run`, the last sample's time.
    pub span: f64,
    /// The trend over the run, `|s|·T_run`.
    pub trend: f64,
    /// The oscillation `A`, the largest residual from the line.
    pub oscillation: f64,
    /// `ρ = |s|·T_run / A`.
    pub ratio: f64,
    /// The largest `|δ|`.
    pub peak: f64,
}

/// The least-squares line through `series` `(t, δ)`, and the trend statistic. A non-finite value anywhere gives a
/// non-finite ratio, never a dropped sample.
pub fn trend(series: &[(f64, f64)]) -> Trend {
    let k = series.len() as f64;
    let mt = series.iter().map(|p| p.0).sum::<f64>() / k;
    let md = series.iter().map(|p| p.1).sum::<f64>() / k;
    let stt: f64 = series.iter().map(|p| (p.0 - mt).powi(2)).sum();
    let std: f64 = series.iter().map(|p| (p.0 - mt) * (p.1 - md)).sum();
    let slope = std / stt;
    let offset = md - slope * mt;
    let fold = |a: f64, x: f64| {
        if x.is_nan() || a.is_nan() {
            f64::NAN
        } else {
            a.max(x)
        }
    };
    let oscillation = series
        .iter()
        .map(|&(t, d)| (d - offset - slope * t).abs())
        .fold(0.0, fold);
    let peak = series.iter().map(|p| p.1.abs()).fold(0.0, fold);
    let span = series.last().map_or(0.0, |p| p.0);
    let trend = slope.abs() * span;
    Trend {
        slope,
        offset,
        span,
        trend,
        oscillation,
        ratio: trend / oscillation,
        peak,
    }
}

/// What an input claims of the occupant's energy error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Claim {
    /// It oscillates with no secular trend: `ρ` at most the threshold.
    Bounded,
    /// It drifts secularly: `ρ` above the threshold.
    Secular,
}

/// A `symplectic-drift` input.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The occupant: `euler`, `kdk`, `yoshida4`, `yoshida6` or `rk4`.
    pub occupant: String,
    /// The claim judged.
    pub claim: Claim,
    /// Whether `SUSPECT_ENERGY` is claimed lit, if the input claims it either way.
    #[serde(default)]
    pub suspect_energy: Option<bool>,
    /// The factor on the sim key's `dt_macro`, 1 by default.
    #[serde(default)]
    pub dt_scale: Option<f64>,
}

/// One occupant's measured run.
#[derive(Clone, Debug)]
pub struct Measured {
    /// The occupant's profile.
    pub profile: CapabilityProfile,
    /// The run.
    pub run: Recorded,
    /// Its trend.
    pub trend: Trend,
    /// Whether the read side lights `SUSPECT_ENERGY` on the stored latches.
    pub suspect: bool,
    /// The stored `dE_max / max(ε_E, |E₀|)` the predicate reads.
    pub suspect_ratio: f64,
}

fn measured<S: Stepper>(fixture: &Fixture, dt_scale: f64) -> Measured {
    let run = record::<S>(fixture, dt_scale, true);
    let trend = trend(&run.series);
    let de_max = pb_dE_max(run.out.packed_b);
    let e_0 = run.out.E_0 as f32;
    let eps = EPS_DEFAULT as f32;
    Measured {
        profile: S::profile(),
        trend,
        suspect: suspect_energy(de_max, e_0, eps),
        suspect_ratio: f64::from(de_max / eps.max(e_0.abs())),
        run,
    }
}

/// The occupant `name` measured on `fixture` at `dt_scale`.
pub fn measure(name: &str, fixture: &Fixture, dt_scale: f64) -> Option<Measured> {
    Some(match name {
        "euler" => measured::<Euler>(fixture, dt_scale),
        "kdk" => measured::<Kdk>(fixture, dt_scale),
        "yoshida4" => measured::<Yoshida4>(fixture, dt_scale),
        "yoshida6" => measured::<Yoshida6>(fixture, dt_scale),
        "rk4" => measured::<Rk4>(fixture, dt_scale),
        _ => return None,
    })
}

/// The resolvability bound for a run of `steps` macro-steps: `ROUNDING_FACTOR·ε·√steps`.
pub fn rounding_bound(steps: usize) -> f64 {
    ROUNDING_FACTOR * f64::EPSILON * (steps as f64).sqrt()
}

/// The `symplectic-drift` gate (REQ-VAL-031; its tolerance REQ-VAL-139's).
pub struct SymplecticDriftGate;

impl Gate for SymplecticDriftGate {
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
        let scale = input.dt_scale.unwrap_or(1.0);
        if !(scale.is_finite() && scale > 0.0) {
            return Err(format!(
                "refused: {name}: dt_scale {scale} is not finite and positive"
            ));
        }
        let orbit = figure_eight();
        let m = measure(&input.occupant, &orbit, scale).ok_or_else(|| {
            format!(
                "refused: {name}: no occupant `{}` (euler, kdk, yoshida4, yoshida6, rk4)",
                input.occupant
            )
        })?;
        let steps = m.run.series.len();
        if steps < 3 {
            return Err(format!(
                "refused: {name}: {steps} macro-step(s) recorded; a trend needs at least 3"
            ));
        }
        let floor = rounding_bound(steps);
        let t = m.trend;
        if t.peak.is_finite() && t.peak < floor {
            return Err(format!(
                "refused: {name}: the largest |δ| {:.3e} is below the rounding bound {floor:.3e} \
                 ({ROUNDING_FACTOR}·ε·√{steps}): the error is rounding's, and judges no claim",
                t.peak
            ));
        }
        let tol = config.threshold.value;
        let secular = t.ratio.is_nan() || t.ratio > tol;
        let trend_ok = match input.claim {
            Claim::Bounded => !secular,
            Claim::Secular => secular,
        };
        let suspect_ok = input.suspect_energy.is_none_or(|lit| lit == m.suspect);
        let pass = trend_ok && suspect_ok;
        let horizon = m.run.horizon;
        let ended = tm_t_end_step(m.run.out.times);
        let mut scatter = vec![
            format!(
                "{}: symplectic {}, dt_macro {:.4e}, {steps} macro-steps recorded over T_run = {:.3} \
                 (the run ended at step {ended} of {horizon}, state {})",
                input.occupant,
                m.profile.symplectic,
                m.run.dt,
                t.span,
                sd_state(m.run.out.packed_a)
            ),
            format!(
                "{}: δ(t) = {:.4e} + {:.4e}·t; trend over the run |s|·T = {:.4e}; oscillation (largest residual) \
                 A = {:.4e}; largest |δ| {:.4e}; ρ = {:.4}",
                input.occupant, t.offset, t.slope, t.trend, t.oscillation, t.peak, t.ratio
            ),
        ];
        scatter.extend(quarters(&m.run.series));
        scatter.push(format!(
            "{}: SUSPECT_ENERGY {} (stored dE_max/max(ε_E, |E₀|) = {:.4e})",
            input.occupant,
            if m.suspect { "lit" } else { "unlit" },
            m.suspect_ratio
        ));
        let mut negative = vec![format!(
            "ρ = {:.4} reads {} against {tol}; the input claims {:?}: {}",
            t.ratio,
            if secular { "secular" } else { "bounded" },
            input.claim,
            if trend_ok {
                "as claimed"
            } else {
                "not as claimed"
            }
        )];
        if let Some(lit) = input.suspect_energy {
            negative.push(format!(
                "SUSPECT_ENERGY is {}; the input claims it {}: {}",
                if m.suspect { "lit" } else { "unlit" },
                if lit { "lit" } else { "unlit" },
                if suspect_ok {
                    "as claimed"
                } else {
                    "not as claimed"
                }
            ));
        }
        if (ended as usize) < horizon as usize {
            negative.push(format!(
                "the run ended before the horizon, at step {ended} of {horizon}; the trend is over the steps it \
                 completed"
            ));
        }
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(steps as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}

/// The series read in quarters of the run: each quarter's mean and largest `|δ|`, so the distribution over the run is
/// in the report and not only its fit.
pub fn quarters(series: &[(f64, f64)]) -> Vec<String> {
    let k = series.len();
    (0..4)
        .map(|q| {
            let part = &series[q * k / 4..(q + 1) * k / 4];
            let n = part.len().max(1) as f64;
            let mean = part.iter().map(|p| p.1).sum::<f64>() / n;
            let peak = part.iter().map(|p| p.1.abs()).fold(0.0, f64::max);
            format!(
                "quarter {}: mean δ {mean:.4e}, largest |δ| {peak:.4e}",
                q + 1
            )
        })
        .collect()
}
