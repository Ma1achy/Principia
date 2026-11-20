//! `δ_dep` measured (TASK-M3-11; REQ-VAL-055; R-37; dd_validation_orbits §6; payload § "Why closure is here"): the
//! distributions of the shape-sphere chord `|n̂(t) − n̂(0)|` on periodic-orbit and generic fixtures, through the one
//! wrapper, at f32 and f64, and the gap `δ_dep` is set in (pitfalls §3: a threshold on a quantity spanning decades is
//! relative or gap-set).
//!
//! **What `δ_dep` must separate.** The closure minimum starts at the first sample whose chord exceeds `δ_dep`, and that
//! sample is the first the minimum takes, its chord just above `δ_dep`. So:
//! - a periodic orbit's return must come in below `δ_dep`, or the departure sample's own chord, not the return, is the
//!   minimum: **below**, the figure-eight's first-return chord, the least chord over the samples from a half to one
//!   and a half of its shape period, around its first return (its shape curve closes at half the orbit's period, where the Euler
//!   configuration it starts at recurs), at f32 and f64, at the default horizon's `dt_macro` (`10⁻³`) and at the
//!   horizon cap's (`200/65535`), the coarsest the sim key allows;
//! - every generic orbit must depart: **above**, the greatest chord over the horizon of each generic fixture, the six rest-start Pythagorean triangles (`benettin::chaotic_orbits`, Burrau first), dd_integrator §5
//!   test 7's close-encounter hierarchy and the figure-eight itself, at f32 and f64, at each fixture's own sim key.
//!
//! **Reported beside, in neither population:** the relative equilibria (Lagrange's orbits, `benettin::periodic_set`)
//! and the Kepler-embedded triple.
//! - The Lagrange orbits' shape is a fixed point in exact arithmetic. At f64 their chords stay near zero over the
//!   horizon and they never depart. The equal-mass orbit and (0.5, 0.3, 0.2) are linearly unstable (Routh's
//!   criterion), so at f32 rounding seeds the unstable growth and carries them off the fixed point: the equal-mass
//!   chord reaches past `δ_dep` within the horizon, a departure of the march at that width, reported, not a
//!   threshold's failure; (0.5, 0.3, 0.2)'s, on the same growth, is below `δ_dep` at the horizon, so whether it departs
//!   depends on the horizon, and a longer one carries it past. (0.98, 0.01, 0.01) is stable and stays near zero.
//! - The Kepler-embedded triple, whose third mass, `10⁻⁹`, pins the shape near a binary collision, is a real moving
//!   orbit: its shape circles on a small loop as the inner binary turns, the same chord at f32 and f64 to four figures,
//!   so motion, not rounding. Its excursion is below `δ_dep`, so it never departs and its closure registers stay unset
//!   over the horizon: the negative case of the rule, a bounded orbit whose shape moves by less than `δ_dep`.
//!
//! **The rule, written before the measurement ran.** `δ_dep` is the geometric middle of the gap, `√(below_max ·
//! above_min)`, rounded to two significant figures; with no gap (`below_max ≥ above_min`) there is no proposal.
//!
//! **The gate** (`cargo xtask gate delta-dep`) is a measurement, its controls alone asserting: the register's `δ_dep`
//! lies strictly inside the measured gap and is the proposal, the ledger's departed row is `packed_a` bit 15, one bit,
//! and the schema version is not the one before the bit was spent; `δ_dep` at either edge of the gap, the controls that
//! must fail, does not. `--propose` prints the report (`fixtures/gates/delta-dep/REPORT.md`).
//!
//! **The conditions every run states.** The occupant is Yoshida4 (as `benettin`'s), the sim key `drift::sim_key` at
//! the fixture's horizon, `r_coll = 0`; each sample is the state after a completed macro-step, read by one-step resumed
//! calls of the wrapper (`kernel::driver::wrapper::integrate_resumed_with`), the march the payload's registers sample.
//!
//! **A failed run is stated, never read.** Yoshida4 reports every macro-step it takes as reached, finite or not, and at
//! `r_coll = 0` nothing ends the march, so a run tests each sample itself: a macro-step that does not complete, a state
//! `(r, p)` or a chord that is not finite ends the run there, that sample is not kept, and the run records the step and
//! the cause ([`Run::failed`]). Every run below and above must be finite and full-length ([`Run::fault`]); one that is
//! not fails the judgement, leaves no proposal and is named in the report. The reductions carry a NaN through rather
//! than dropping it, so a failed sample can never pass as a smaller excursion.

use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

use kernel::decode::Ic;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::welford::chord2;
use kernel::driver::wrapper::{
    integrate_resumed_with, integrate_with, WrapperReal, WrapperUniforms,
};
use kernel::occupant::yoshida4::Yoshida4;
use kernel::payload::{tm_t_end_step, PAYLOAD_SCHEMA_VERSION};
use kernel::shape::shape;
use ledger::constants::DELTA_DEP_VALUE;
use ledger::schema::Location;

use crate::drift::{self, ic_at, sim_key, AsR};
use crate::gate::{MeasurementConfig, MeasurementControl, MeasurementGate, Verdict};
use crate::measure::benettin::{chaotic_orbits, kepler_embedded, periodic_set};

/// The gate's name.
pub const NAME: &str = "delta-dep";

/// The payload schema version before this task spent `packed_a` bit 15 on the departed bit (TASK-M3-11).
pub const VERSION_BEFORE_DEPARTED: u64 = 0xd602_ac69_9cd5_5a52;

/// The horizon cap's `dt_macro`, the coarsest the sim key allows (integrator_contract Part 5; R-132).
pub const COARSEST_HORIZON: f64 = drift::T_CAP;

/// One fixture: its name, its canonical IC and its horizon.
#[derive(Clone, Copy, Debug)]
pub struct Fixture {
    pub name: &'static str,
    pub ic: Ic<f64>,
    pub horizon: f64,
}

/// Why a sample is no sample (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The macro-step did not complete: the march stopped before its horizon.
    Short,
    /// The state `(r, p)` after the macro-step is not finite.
    State,
    /// The chord after the macro-step is not finite.
    Chord,
}

impl Cause {
    /// The cause, in words.
    pub fn word(self) -> &'static str {
        match self {
            Cause::Short => "the march stopped before its horizon",
            Cause::State => "the state is not finite",
            Cause::Chord => "the chord is not finite",
        }
    }
}

/// A run's failure: the first macro-step, `1 … n`, whose sample is no sample, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub step: u32,
    pub cause: Cause,
}

/// The sample after macro-step `k`: its chord `√d2`, or the failure, `reached` the macro-steps the march has completed
/// and `state_finite` whether its `(r, p)` is finite. A step not reached is tested first, then the state, then the
/// chord.
pub fn sample(k: u32, reached: u32, state_finite: bool, d2: f64) -> Result<f64, Failure> {
    let fail = |cause| Err(Failure { step: k, cause });
    if reached != k {
        return fail(Cause::Short);
    }
    if !state_finite {
        return fail(Cause::State);
    }
    let c = d2.sqrt();
    if !c.is_finite() {
        return fail(Cause::Chord);
    }
    Ok(c)
}

/// The greater of `a` and `b`, NaN where either is (`f64::max` drops it).
fn max_nan(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// The lesser of `a` and `b`, NaN where either is (`f64::min` drops it).
fn min_nan(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// One run's chords: the fixture, the width, its `dt_macro`, the chord after each completed macro-step `1 … n`, the
/// macro-step count of its horizon, and the failure that ended it, if one did (the module's note).
#[derive(Clone, Debug)]
pub struct Run {
    pub name: &'static str,
    pub width: &'static str,
    pub dt: f64,
    pub chords: Vec<f64>,
    pub horizon_steps: u32,
    pub failed: Option<Failure>,
}

impl Run {
    /// The greatest chord, NaN where any chord is.
    pub fn max(&self) -> f64 {
        self.chords.iter().fold(0.0, |m, &c| max_nan(m, c))
    }

    /// The greatest chord between consecutive samples' chords, a lower bound on a step's arc; NaN where any chord is.
    pub fn step(&self) -> f64 {
        self.chords
            .windows(2)
            .fold(self.chords.first().copied().unwrap_or(0.0), |m, w| {
                max_nan(m, (w[1] - w[0]).abs())
            })
    }

    /// The least chord over the samples at times in `[from, to]`, with its time; the first NaN in the window, with its
    /// time, where there is one.
    pub fn least_in(&self, from: f64, to: f64) -> Option<(f64, f64)> {
        self.chords
            .iter()
            .enumerate()
            .map(|(i, &c)| ((i + 1) as f64 * self.dt, c))
            .filter(|&(t, _)| t >= from && t <= to)
            .fold(None, |best: Option<(f64, f64)>, (t, c)| match best {
                Some((b, _)) if b.is_nan() || b <= c => best,
                _ => Some((c, t)),
            })
    }

    /// Whether, and where, the run is not finite and full-length: its recorded failure; else its first chord that is
    /// not finite; else, where it holds other than one chord a macro-step of its horizon, the step after its last.
    pub fn fault(&self) -> Option<Failure> {
        self.failed
            .or_else(|| {
                self.chords
                    .iter()
                    .position(|c| !c.is_finite())
                    .map(|i| Failure {
                        step: i as u32 + 1,
                        cause: Cause::Chord,
                    })
            })
            .or_else(|| {
                (self.chords.len() != self.horizon_steps as usize).then(|| Failure {
                    step: self.chords.len() as u32 + 1,
                    cause: Cause::Short,
                })
            })
    }

    /// The run's failure as a line: the run, its width, `dt_macro` and step, and the cause; `None` where it has none.
    pub fn fault_line(&self) -> Option<String> {
        self.fault().map(|f| {
            format!(
                "run {} {} dt {:.4e}: failed at step {}, {}",
                self.name,
                self.width,
                self.dt,
                f.step,
                f.cause.word()
            )
        })
    }

    /// The report's flag on a failed run, empty where it has none.
    fn flag(&self) -> String {
        self.fault()
            .map(|f| format!(" (failed at step {}: {})", f.step, f.cause.word()))
            .unwrap_or_default()
    }
}

/// The figure-eight's period in the canonical frame: `ledger::fixtures`' period `P` in `G = 1`, `mᵢ = 1`; at masses
/// `1/3` with the velocities scaled by `√(1/3)` (`drift::figure_eight`) the orbit keeps its positions and runs `√3`
/// slower; canonicalising scales lengths by `λ`, read as the ratio of a canonical to a published separation, and times
/// by `λ^{3/2}` (`G = M = 1` both sides).
pub fn eight_period() -> f64 {
    let published = ledger::fixtures::figure_eight();
    let [x, y] = published.x1;
    let raw = 2.0 * x.hypot(y);
    let ic = drift::figure_eight().ic;
    let canon = (ic.r[1][0] - ic.r[0][0]).hypot(ic.r[1][1] - ic.r[0][1]);
    published.period * 3f64.sqrt() * (canon / raw).powf(1.5)
}

/// The figure-eight's shape period, half its orbit's: the Euler configuration it starts at, body 2 midway between the
/// others, recurs rotated half a period later, so its shape point recurs.
pub fn eight_shape_period() -> f64 {
    0.5 * eight_period()
}

/// The chords of `fixture` at `R` under its sim key at `horizon`: each completed macro-step's, by one-step resumed
/// calls. The march stops at the first sample that is no sample ([`sample`]: a macro-step that does not complete, a
/// state or chord that is not finite), keeps no chord for it and records its step and cause in [`Run::failed`]; the
/// wrapper does not stop it there, Yoshida4 reporting every step reached and nothing ending the march at `r_coll = 0`.
pub fn run<R: WrapperReal + Into<f64>>(fixture: &Fixture, horizon: f64, width: &'static str) -> Run
where
    f64: AsR<R>,
{
    let ic = ic_at::<R>(&fixture.ic);
    let key = sim_key::<R>(horizon);
    let n_0 = shape(ic.r, ic.m);
    let zero = WrapperUniforms {
        horizon_steps: 0,
        ..key
    };
    let mut s =
        integrate_with::<Yoshida4, R, _>(&ic, &zero, &mut WrapperCadence::new(&ic.m, &ic.r, &ic.p));
    let mut chords = Vec::with_capacity(key.horizon_steps as usize);
    let mut failed = None;
    for k in 1..=key.horizon_steps {
        let mut cadence = WrapperCadence::new(&ic.m, &s.r, &s.p);
        s = integrate_resumed_with::<Yoshida4, R, _>(&ic, &key, &mut cadence, &s, 1);
        let state_finite =
            s.r.iter()
                .chain(s.p.iter())
                .flatten()
                .all(|&x| Into::<f64>::into(x).is_finite());
        let d2: f64 = chord2(shape(s.r, ic.m), n_0).into();
        match sample(k, tm_t_end_step(s.times), state_finite, d2) {
            Ok(c) => chords.push(c),
            Err(f) => {
                failed = Some(f);
                break;
            }
        }
    }
    Run {
        name: fixture.name,
        width,
        dt: key.dt_macro.into(),
        chords,
        horizon_steps: key.horizon_steps,
        failed,
    }
}

/// `fixture` at both widths.
fn both(fixture: &Fixture, horizon: f64) -> [Run; 2] {
    [
        run::<f32>(fixture, horizon, "f32"),
        run::<f64>(fixture, horizon, "f64"),
    ]
}

fn of(o: crate::measure::benettin::Orbit) -> Fixture {
    Fixture {
        name: o.name,
        ic: o.ic,
        horizon: o.horizon,
    }
}

/// The figure-eight, to the default horizon.
pub fn eight() -> Fixture {
    Fixture {
        name: "figure-eight",
        ic: drift::figure_eight().ic,
        horizon: drift::T_DEFAULT,
    }
}

/// The generic fixtures (the module's note).
pub fn generic() -> Vec<Fixture> {
    let mut out: Vec<Fixture> = chaotic_orbits().into_iter().map(of).collect();
    let close = drift::close_encounter();
    out.push(Fixture {
        name: "close-encounter hierarchy (test 7)",
        ic: close.ic,
        horizon: close.horizon,
    });
    out.push(eight());
    out
}

/// The fixtures reported beside, in neither population (the module's note).
pub fn beside() -> Vec<Fixture> {
    let mut out: Vec<Fixture> = periodic_set()
        .into_iter()
        .filter(|o| o.name.starts_with("Lagrange"))
        .map(of)
        .collect();
    out.push(of(kepler_embedded()));
    out
}

/// One population: its runs' values, named.
#[derive(Clone, Debug)]
pub struct Population {
    pub values: Vec<(String, f64)>,
}

impl Population {
    /// The least value, NaN where any value is.
    pub fn lo(&self) -> f64 {
        self.values
            .iter()
            .fold(f64::INFINITY, |m, (_, v)| min_nan(m, *v))
    }

    /// The greatest value, NaN where any value is.
    pub fn hi(&self) -> f64 {
        self.values.iter().fold(0.0, |m, (_, v)| max_nan(m, *v))
    }
}

/// The measurement: the figure-eight's first returns (below), the generic fixtures' greatest chords (above), the runs
/// reported beside, and the shape period.
#[derive(Clone, Debug)]
pub struct Measured {
    pub returns: Vec<(Run, f64, f64)>,
    pub generic: Vec<Run>,
    pub beside: Vec<Run>,
    pub shape_period: f64,
}

impl Measured {
    /// Runs the measurement.
    pub fn run() -> Self {
        let p = eight_shape_period();
        let mut returns = Vec::new();
        for horizon in [drift::T_DEFAULT, COARSEST_HORIZON] {
            let fixture = Fixture { horizon, ..eight() };
            for r in both(&fixture, horizon).into_iter() {
                // A run that failed before the window has no first return: NaN, and the failure is the run's, which
                // the judgement and the report state.
                let (least, at) = r.least_in(0.5 * p, 1.5 * p).unwrap_or((f64::NAN, f64::NAN));
                returns.push((r, least, at));
            }
        }
        let generic = generic().iter().flat_map(|f| both(f, f.horizon)).collect();
        let beside = beside().iter().flat_map(|f| both(f, f.horizon)).collect();
        Measured {
            returns,
            generic,
            beside,
            shape_period: p,
        }
    }

    /// The population below `δ_dep`: the first-return chords.
    pub fn below(&self) -> Population {
        Population {
            values: self
                .returns
                .iter()
                .map(|(r, least, _)| (format!("{} {} dt {:.3e}", r.name, r.width, r.dt), *least))
                .collect(),
        }
    }

    /// The population above `δ_dep`: the generic fixtures' greatest chords.
    pub fn above(&self) -> Population {
        Population {
            values: self
                .generic
                .iter()
                .map(|r| (format!("{} {}", r.name, r.width), r.max()))
                .collect(),
        }
    }

    /// The runs below and above that are not finite and full-length ([`Run::fault`]), in their order.
    pub fn failures(&self) -> Vec<&Run> {
        self.returns
            .iter()
            .map(|(r, _, _)| r)
            .chain(self.generic.iter())
            .filter(|r| r.fault().is_some())
            .collect()
    }

    /// The rule's proposal (the module's note), or `None` with no gap or with any run below or above failed.
    pub fn proposal(&self) -> Option<f64> {
        let (below, above) = (self.below().hi(), self.above().lo());
        (self.failures().is_empty() && below < above).then(|| two_figures((below * above).sqrt()))
    }
}

/// `x` rounded to two significant figures.
pub fn two_figures(x: f64) -> f64 {
    // Divided by an exact power of ten, so the result is the decimal literal's nearest double.
    let k = 1 - x.log10().floor() as i32;
    let ten = 10f64.powi(k.abs());
    if k >= 0 {
        (x * ten).round() / ten
    } else {
        (x / ten).round() * ten
    }
}

/// The measurement, run once per process.
pub fn measured() -> &'static Measured {
    static M: OnceLock<Measured> = OnceLock::new();
    M.get_or_init(Measured::run)
}

macro_rules! say {
    ($out:expr, $($arg:tt)*) => {{
        $out.push_str(&format!($($arg)*));
        $out.push('\n');
    }};
}

/// The report `--propose` prints (`fixtures/gates/delta-dep/REPORT.md`).
pub fn report(m: &Measured) -> String {
    let mut out = String::new();
    say!(out, "# REQ-VAL-055: `δ_dep`, the closure's departure threshold, measured (delta-dep --propose)\n");
    say!(out,
        "The chord `|n̂(t) − n̂(0)|` after each completed macro-step, through the wrapper (Yoshida4, `drift::sim_key` at \
         the fixture's horizon, `r_coll = 0`), at f32 and f64. `validation::measure::delta_dep` states the populations \
         and the rule: `δ_dep = √(below_max · above_min)`, two significant figures, written before the run.\n"
    );
    say!(out, "## Below: the figure-eight's first return\n");
    say!(out,
        "The least chord over `[P_s/2, 3P_s/2]`, the samples around the first return at `P_s`, `P_s = {:.6}` the shape period (half the orbit's, {:.6}).\n",
        m.shape_period,
        2.0 * m.shape_period
    );
    say!(
        out,
        "| width | dt_macro | first-return chord | at t | greatest step change |"
    );
    say!(out, "|---|---|---|---|---|");
    for (r, least, at) in &m.returns {
        say!(
            out,
            "| {}{} | {:.4e} | {:.3e} | {:.4} | {:.3e} |",
            r.width,
            r.flag(),
            r.dt,
            least,
            at,
            r.step()
        );
    }
    say!(out, "\n## Above: the generic fixtures' greatest chord\n");
    say!(
        out,
        "| fixture | width | dt_macro | steps / horizon | greatest chord | greatest step change |"
    );
    say!(out, "|---|---|---|---|---|---|");
    for r in &m.generic {
        say!(
            out,
            "| {} | {}{} | {:.4e} | {} / {} | {:.4} | {:.3e} |",
            r.name,
            r.width,
            r.flag(),
            r.dt,
            r.chords.len(),
            r.horizon_steps,
            r.max(),
            r.step()
        );
    }
    say!(
        out,
        "\n## Beside: the relative equilibria and the Kepler-embedded triple (in neither population)\n"
    );
    say!(out,
        "The Lagrange orbits' shapes are fixed points in exact arithmetic: at f64 their chords stay near zero. Equal \
         masses and (0.5, 0.3, 0.2) are linearly unstable (Routh), so at f32 rounding seeds the growth: the equal-mass \
         chord passes `δ_dep` within the horizon and departs, and (0.5, 0.3, 0.2)'s, on the same growth, is below it \
         at the horizon, so whether it departs depends on the horizon. (0.98, 0.01, 0.01) is stable. The \
         Kepler-embedded triple is a real moving orbit, the same chord at f32 and f64: its excursion is below `δ_dep`, \
         so it never departs and its closure registers stay unset.\n"
    );
    say!(
        out,
        "| fixture | width | dt_macro | steps / horizon | greatest chord |"
    );
    say!(out, "|---|---|---|---|---|");
    for r in &m.beside {
        say!(
            out,
            "| {} | {}{} | {:.4e} | {} / {} | {:.3e} |",
            r.name,
            r.width,
            r.flag(),
            r.dt,
            r.chords.len(),
            r.horizon_steps,
            r.max()
        );
    }
    say!(out, "\n## Failed runs\n");
    let failed: Vec<String> = m
        .returns
        .iter()
        .map(|(r, _, _)| r)
        .chain(&m.generic)
        .chain(&m.beside)
        .filter_map(Run::fault_line)
        .collect();
    if failed.is_empty() {
        say!(out, "None: every run is finite and full-length.");
    } else {
        for line in &failed {
            say!(out, "- {line}");
        }
    }
    let (below, above) = (m.below(), m.above());
    say!(out, "\n## The gap\n");
    say!(
        out,
        "Below: {} values in [{:.3e}, {:.3e}]. Above: {} values in [{:.4}, {:.4}].",
        below.values.len(),
        below.lo(),
        below.hi(),
        above.values.len(),
        above.lo(),
        above.hi()
    );
    match m.proposal() {
        Some(d) => say!(out,
            "\n**Proposed:** `δ_dep = {d}`, the geometric middle of the gap ({:.3e}, {:.4}), rounded to two significant \
             figures: {:.2} decades above the largest first return and {:.2} below the smallest generic excursion. The \
             register holds {}.",
            below.hi(), above.lo(), (d / below.hi()).log10(), (above.lo() / d).log10(), DELTA_DEP_VALUE
        ),
        None if !m.failures().is_empty() => say!(out,
            "\n**No proposal:** runs below or above failed: {} (Failed runs).", m.failures().len()
        ),
        None => say!(out, "\n**No gap:** the largest first return is not below the smallest excursion; no proposal."),
    }
    out
}

/// Whether the ledger's `departed` row is `packed_a` bit 15, one bit (R-545).
pub fn departed_row() -> bool {
    let ledger = ledger::layout();
    ledger::gen::validate(&ledger)
        .map(|entries| {
            entries.iter().any(|e| {
                e.name == "departed"
                    && e.location
                        == Location::Packed {
                            word: "packed_a",
                            offset: 15,
                            width: 1,
                        }
            })
        })
        .unwrap_or(false)
}

/// A `delta-dep` control input.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeltaControl {
    pub description: String,
    /// `"register"`: the register's `δ_dep`; `"below"`: the largest first return, the gap's lower edge; `"above"`:
    /// the smallest generic excursion, its upper edge.
    pub delta: String,
}

/// Judges `delta` against the measurement: every run below and above finite and full-length, `delta` strictly inside
/// the gap, the proposal, the departed row at bit 15, and the schema version moved. Returns the verdict and the lines
/// saying why, one for each failed run, naming it, its width, `dt_macro` and step.
pub fn judge(m: &Measured, delta: f64) -> (Verdict, Vec<String>) {
    let (below, above) = (m.below().hi(), m.above().lo());
    let failures = m.failures();
    let mut checks: Vec<(bool, String)> = failures
        .iter()
        .filter_map(|r| r.fault_line())
        .map(|line| (false, line))
        .collect();
    if failures.is_empty() {
        checks.push((
            true,
            format!(
                "every run below and above is finite and full-length ({} runs)",
                m.returns.len() + m.generic.len()
            ),
        ));
    }
    checks.extend([
        (delta > below, format!("δ = {delta:e} above the largest first return {below:.3e}")),
        (delta < above, format!("δ = {delta:e} below the smallest generic excursion {above:.4}")),
        (m.proposal() == Some(delta), format!("δ = {delta:e} the proposal {:?}", m.proposal())),
        (departed_row(), "the ledger's departed row is packed_a bit 15, one bit".to_owned()),
        (
            PAYLOAD_SCHEMA_VERSION != VERSION_BEFORE_DEPARTED,
            format!("the schema version {PAYLOAD_SCHEMA_VERSION:#018x} is not {VERSION_BEFORE_DEPARTED:#018x}, the one before the bit was spent"),
        ),
    ]);
    let pass = checks.iter().all(|(ok, _)| *ok);
    let lines = checks
        .iter()
        .map(|(ok, what)| format!("- {}: {what}", if *ok { "holds" } else { "fails" }))
        .collect();
    (if pass { Verdict::Pass } else { Verdict::Fail }, lines)
}

/// The `delta-dep` measurement gate (REQ-VAL-055).
pub struct DeltaDepGate;

impl MeasurementGate for DeltaDepGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let control = DeltaControl::deserialize(fixture).map_err(|e| format!("{name}: {e}"))?;
        let m = measured();
        let delta = match control.delta.as_str() {
            "register" => DELTA_DEP_VALUE,
            "below" => m.below().hi(),
            "above" => m.above().lo(),
            other => return Err(format!("{name}: no δ `{other}` (register, below or above)")),
        };
        let (verdict, lines) = judge(m, delta);
        Ok(MeasurementControl {
            markdown: format!(
                "# delta-dep: {name}\n\n{}\n\n{}\n",
                control.description,
                lines.join("\n")
            ),
            description: control.description,
            verdict,
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => {
                print!("{}", report(measured()));
                Ok(())
            }
            _ => Err(format!(
                "gate {NAME}: unrecognised flags `{}`; usage: --propose",
                args.join(" ")
            )),
        }
    }
}
