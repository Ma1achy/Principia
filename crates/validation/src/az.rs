//! Aarseth–Zare's gates and proposals (TASK-M3-68; integrator_contract Part 2b § "Aarseth–Zare: the equations, the
//! reference body, the Levi-Civita inverse and the step control (transcribed)", Part 6; R-927, R-953):
//!
//! - **`lc-inverse`** ([`LcInverseGate`]; REQ-INT-044): the Levi-Civita round trip `R → u → L(u)u` at a `ρ` angle of
//!   179.9°, at f32 and f64, its error in units in the last place of `R`'s larger component at the width: the
//!   conditioned inverse at most one, the requirement's `≈ 5.96e-8` and `≈ 1e-16` (one unit in the last place of a
//!   component just below 1 is `2⁻²⁴ ≈ 5.96e-8` at f32 and `2⁻⁵³ ≈ 1.1e-16` at f64); the unstable form, the control,
//!   far above it (the reference's table: `2.2e-2` at f32, `4.0e-12` at f64, relative).
//! - **`az-final-step`** ([`AzFinalStepGate`]; REQ-INT-049, REQ-INT-114, REQ-INT-119): the figure-eight's closure
//!   after one period on the gate's own sync interval `T/32` at `η = 0.001`, under the step control in force with the
//!   landing clamp, secant and bisection (R-998), at most REQ-INT-114's proposed threshold, and its convergence order
//!   over the `η` ladder, fitted by least squares, within the input's margin of RK4's order 4; the controls, the closure
//!   without the landing fix and the landing that sets the clock alone over the overshot state, beside it. `--propose` prints REQ-INT-114's proposal and
//!   `--propose-sync` REQ-INT-080's (`Δ_sync` and `η`).
//! - **`separation-floor`** ([`SeparationFloorGate`]; REQ-INT-105): a measurement; `--propose` prints `TINY`'s
//!   proposal at f32 and f64 from Aarseth–Zare's pre-floor `A` and `B` and the inverse's guarded component; its
//!   controls are a `TINY` above the radial collision passage's smallest pre-floor value, which floors steps there,
//!   and the reference's, which floors none.
//!
//! **The conditions every run states.** The values the step control reads are calibrations, so each run here names
//! the ones it ran at: `TINY` the reference's (`1e-37` at f32, `1e-300` at f64; R-953) unless the run varies it, which
//! also floors `|R₃|` (R-999), the landing with no tolerance, its secant's 4 retakes and its bisection's 64 halvings
//! (R-998), R-160's `f = 0.02`, a step budget that does not bind, and the stepper RK4 (Part 2a's validation-grade
//! pair).

use std::path::Path;

use serde::Deserialize;

use kernel::driver::advance::Cadence;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::regularisation::az::{
    lc, lc_inverse, lc_inverse_unstable, march, AzAdvanced, AzCount, AzP, AzParams, AzQ, AzReal,
    End, ExplicitTest, Watch,
};
use kernel::regularisation::dt_max::DtMaxReport;

use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig, MeasurementConfig, MeasurementControl, MeasurementGate};

/// Appends one formatted line to a proposal's text.
macro_rules! say {
    ($out:expr, $($arg:tt)*) => {{
        $out.push_str(&format!($($arg)*));
        $out.push('\n');
    }};
}

/// A cadence that keeps nothing and never ends the march.
#[derive(Clone, Copy, Debug, Default)]
pub struct Quiet;

impl<R> Cadence<R> for Quiet {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}
    fn done(&self) -> bool {
        false
    }
}

/// A precision the gates run at.
pub trait Width: AzReal + Into<f64> {
    /// The precision's unit round-off, `2⁻²⁴` or `2⁻⁵³`: half a unit in the last place of 1.
    const UNIT: f64;
}

impl Width for f32 {
    const UNIT: f64 = f32::EPSILON as f64 / 2.0;
}

impl Width for f64 {
    const UNIT: f64 = f64::EPSILON / 2.0;
}

/// `x` at `R`'s width.
pub fn at<R: AzReal>(x: f64) -> R {
    R::at_width(x as f32, x)
}

/// The reference's `TINY` at `R`: `1e-37` at f32, `1e-300` at f64 (R-953).
pub fn reference_tiny<R: AzReal>() -> R {
    R::at_width(1e-37, 1e-300)
}

/// How a run lands at its sync boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Landing {
    /// The rule in force (Part 2b, steps 3 and 6; R-998): the clamp, the secant's 4 retakes and the bisection's 64
    /// halvings, the state landing on the boundary with its clock.
    Fix,
    /// The clamp with no retakes: the clamped step's state kept where it overshoots, the clock set to the boundary
    /// alone, what R-998 forbids (REQ-INT-119's control).
    ClockOnly,
    /// No clamp and no retakes: each interval's last step overshoots by up to a step (REQ-INT-049's control).
    Off,
}

impl Landing {
    /// The words a run's description gives it.
    pub fn name(self) -> &'static str {
        match self {
            Landing::Fix => "landing fix on",
            Landing::ClockOnly => "clock-only landing",
            Landing::Off => "landing fix off",
        }
    }
}

/// The run conditions at `R` (the module's note), at step factor `eta` and sync quantum `delta_sync`, landing as
/// `landing` says.
pub fn conditions<R: AzReal>(eta: f64, delta_sync: f64, landing: Landing) -> AzParams<R> {
    let fix = landing == Landing::Fix;
    AzParams {
        eta: at(eta),
        delta_sync: at(delta_sync),
        tiny: reference_tiny(),
        predictive_f: at(0.02),
        step_budget_rate: at(0.0),
        n_min: u32::MAX,
        clamp: landing != Landing::Off,
        secant_tries: if fix { 4 } else { 0 },
        bisect_tries: if fix { 64 } else { 0 },
    }
}

/// `|a − b|` over every position component, the Euclidean norm of the difference.
pub fn distance(a: &[[f64; 2]; 3], b: &[[f64; 2]; 3]) -> f64 {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// A state's positions at f64.
pub fn positions<R: Width>(s: &State<R>) -> [[f64; 2]; 3] {
    s.r.map(|v| v.map(Into::into))
}

/// The physical energy of `s`, `Σ|pᵢ|²/(2mᵢ) − Σ mᵢmⱼ/|rᵢ − rⱼ|`, at f64.
pub fn energy<R: Width>(s: &State<R>) -> f64 {
    let m: [f64; 3] = s.m.map(Into::into);
    let r = positions(s);
    let p: [[f64; 2]; 3] = s.p.map(|v| v.map(Into::into));
    let kinetic: f64 = (0..3)
        .map(|i| (p[i][0].powi(2) + p[i][1].powi(2)) / (2.0 * m[i]))
        .sum();
    let potential: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| m[i] * m[j] / ((r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1])))
        .sum();
    kinetic - potential
}

// ── lc-inverse ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The `lc-inverse` gate's name.
pub const LC_NAME: &str = "lc-inverse";

/// One round trip `R → u → L(u)u` of the inverse at a width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trip {
    /// `|L(u)u − R|/|R|`, the reference's figure.
    pub relative: f64,
    /// `|L(u)u − R|` in units in the last place of `R`'s larger component at the width, `2^(⌊log₂ m⌋ + 1)` times its
    /// unit round-off for that component `m`: the gate's measure.
    pub ulps: f64,
}

/// The round trip of the inverse (the conditioned one, or the unstable form) at `R`'s width, `R` the unit vector at
/// `angle` degrees rounded to the width, the error taken in f64.
pub fn lc_error<R: Width>(angle: f64, conditioned: bool) -> Trip {
    let th = angle.to_radians();
    let r: [R; 2] = [at(th.cos()), at(th.sin())];
    let u = if conditioned {
        lc_inverse(r, reference_tiny())
    } else {
        lc_inverse_unstable(r, reference_tiny())
    };
    let back = lc(u);
    let (x, y): (f64, f64) = (r[0].into(), r[1].into());
    let (bx, by): (f64, f64) = (back[0].into(), back[1].into());
    let error = (bx - x).hypot(by - y);
    let m = x.abs().max(y.abs());
    let ulp = 2f64.powi(m.log2().floor() as i32 + 1) * R::UNIT;
    Trip {
        relative: error / x.hypot(y),
        ulps: error / ulp,
    }
}

/// An `lc-inverse` input.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LcInput {
    pub description: String,
    /// `"conditioned"` or `"unstable"`.
    pub form: String,
    /// The angle of `ρ`, in degrees.
    pub angle: f64,
}

/// The `lc-inverse` gate (REQ-INT-044).
pub struct LcInverseGate;

impl Gate for LcInverseGate {
    fn name(&self) -> &'static str {
        LC_NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = LcInput::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
        let conditioned = match input.form.as_str() {
            "conditioned" => true,
            "unstable" => false,
            other => {
                return Err(format!(
                    "refused: {name}: no form `{other}` (conditioned or unstable)"
                ))
            }
        };
        let threshold = config.threshold.value;
        let mut scatter = Vec::new();
        let mut negative = Vec::new();
        let mut pass = true;
        for (width, trip, other) in [
            (
                "f32",
                lc_error::<f32>(input.angle, conditioned),
                lc_error::<f32>(input.angle, !conditioned),
            ),
            (
                "f64",
                lc_error::<f64>(input.angle, conditioned),
                lc_error::<f64>(input.angle, !conditioned),
            ),
        ] {
            let ulps = trip.ulps;
            scatter.push(format!(
                "{width}, the {} inverse at {}°: relative error {:.3e}, {ulps:.3} units in R's last place; the {} \
                 form there: relative error {:.3e}, {:.3} units",
                input.form,
                input.angle,
                trip.relative,
                if conditioned { "unstable" } else { "conditioned" },
                other.relative,
                other.ulps
            ));
            if ulps <= threshold {
                negative.push(format!(
                    "{width}: within {threshold} unit(s) in R's last place"
                ));
            } else {
                pass = false;
                negative.push(format!(
                    "{width}: {ulps:.3e} units in R's last place, beyond {threshold}"
                ));
            }
        }
        scatter.push(
            "the reference's table at 179.9° (tb_lc.py, R-927): conditioned 5.96e-8 at f32 and ~1e-16 at f64, \
             unstable 2.2e-2 at f32 and 4.0e-12 at f64"
                .to_owned(),
        );
        Ok(GateReport {
            gate: LC_NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(2),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}

// ── The figure-eight ───────────────────────────────────────────────────────────────────────────────────────────

/// The `az-final-step` gate's name.
pub const FINAL_STEP_NAME: &str = "az-final-step";

/// The figure-eight orbit fixture, `fixtures/gates/az-figure-eight/orbit.json`: the published IC in `G = 1`,
/// `mᵢ = 1`, its period and energy (dd_validation_orbits § 0), and the reference's figures, quoted as measurements.
pub const ORBIT: &str = include_str!("../../../fixtures/gates/az-figure-eight/orbit.json");

/// The orbit fixture.
#[derive(Clone, Debug, Deserialize)]
pub struct Orbit {
    pub description: String,
    pub masses: [f64; 3],
    pub x1: [f64; 2],
    pub v3: [f64; 2],
    pub period: f64,
    pub energy: f64,
    pub reference: Vec<String>,
}

/// The orbit fixture, checked against `ledger::fixtures`'s figure-eight, the IC the rest of the workspace reads.
pub fn orbit() -> Result<Orbit, String> {
    let o: Orbit =
        serde_json::from_str(ORBIT).map_err(|e| format!("az-figure-eight/orbit.json: {e}"))?;
    checked(o)
}

/// `o`, refused where its IC or period differs from `ledger::fixtures`'s figure-eight.
pub fn checked(o: Orbit) -> Result<Orbit, String> {
    let f = ledger::fixtures::figure_eight();
    if o.x1 != f.x1 || o.v3 != f.v3 || o.period != f.period {
        return Err(
            "az-figure-eight/orbit.json differs from ledger::fixtures::figure_eight".to_owned(),
        );
    }
    Ok(o)
}

/// The figure-eight at `t = 0` from `o`: `x₁ = −x₂`, `x₃ = 0`, `v₁ = v₂ = −v₃/2`, the momenta `mᵢvᵢ`.
pub fn figure_eight<R: Width>(o: &Orbit) -> State<R> {
    let [x, y] = o.x1;
    let [vx, vy] = o.v3;
    let v = [[-vx / 2.0, -vy / 2.0], [-vx / 2.0, -vy / 2.0], [vx, vy]];
    State {
        m: o.masses.map(at),
        r: [[x, y], [-x, -y], [0.0, 0.0]].map(|p| p.map(at)),
        p: [0, 1, 2].map(|i| v[i].map(|c| at(o.masses[i] * c))),
    }
}

/// One figure-eight run: one period on the grid `T/n` at `eta`, landing as `landing` says.
#[derive(Clone, Copy, Debug)]
pub struct Closure {
    pub eta: f64,
    pub n: u32,
    pub landing: Landing,
    /// The closure `|r(T) − r(0)|` over every position component.
    pub closure: f64,
    /// `|E(T) − E(0)|/|E(0)|`, the Cartesian energy.
    pub energy: f64,
    /// The positions at `T`.
    pub end: [[f64; 2]; 3],
    /// The state at `T`.
    pub state: State<f64>,
    /// What the march counted.
    pub count: AzCount<f64>,
}

/// The figure-eight over one period on `n` sync intervals at `eta`, at f64.
pub fn closure(o: &Orbit, eta: f64, n: u32, landing: Landing) -> Closure {
    let s0 = figure_eight::<f64>(o);
    let params = conditions::<f64>(eta, o.period / f64::from(n), landing);
    let out = march::<Rk4, f64, Quiet, ExplicitTest>(
        &s0,
        0,
        n,
        AzCount::new(),
        &params,
        &mut Quiet,
        &mut ExplicitTest,
    );
    Closure {
        eta,
        n,
        landing,
        closure: distance(&positions(&out.state), &positions(&s0)),
        energy: ((energy(&out.state) - energy(&s0)) / energy(&s0)).abs(),
        end: positions(&out.state),
        state: out.state,
        count: out.count,
    }
}

/// The `η` ladder of REQ-INT-114's convergence (`[0.02, 0.001]`, dd_validation_orbits § 0.1's).
pub const LADDER: [f64; 5] = [0.02, 0.01, 0.005, 0.002, 0.001];

/// The convergence over a ladder: each rung's run, its error against a reference run at `reference_eta` (the landing
/// fix on), and the order fitted on the ladder: the least-squares slope of `ln e` against `ln η` over every rung.
#[derive(Clone, Debug)]
pub struct Convergence {
    pub rungs: Vec<(Closure, f64)>,
    pub reference: Closure,
    pub order: f64,
}

/// The convergence of the figure-eight over `ladder` on `n` intervals, landing as `landing` says, against a run at
/// `reference_eta` with the landing fix on.
pub fn convergence(
    o: &Orbit,
    ladder: &[f64],
    reference_eta: f64,
    n: u32,
    landing: Landing,
) -> Convergence {
    let reference = closure(o, reference_eta, n, Landing::Fix);
    let rungs: Vec<(Closure, f64)> = ladder
        .iter()
        .map(|&eta| {
            let c = closure(o, eta, n, landing);
            let e = distance(&c.end, &reference.end);
            (c, e)
        })
        .collect();
    let pairs: Vec<(f64, f64)> = rungs.iter().map(|(c, e)| (c.eta, *e)).collect();
    let order = fitted_order(&pairs);
    Convergence {
        rungs,
        reference,
        order,
    }
}

/// The least-squares slope of `y` against `x` over `points`.
pub fn slope(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;
    let mx = points.iter().map(|p| p.0).sum::<f64>() / n;
    let my = points.iter().map(|p| p.1).sum::<f64>() / n;
    let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let sxx: f64 = points.iter().map(|p| (p.0 - mx) * (p.0 - mx)).sum();
    sxy / sxx
}

/// An `az-final-step` input.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalStepInput {
    pub description: String,
    /// The step factor of the closure.
    pub eta: f64,
    /// The sync intervals per period.
    pub n: u32,
    /// How the runs land.
    pub landing: Landing,
    /// The `η` ladder of the convergence arm.
    pub ladder: Vec<f64>,
    /// The reference run's `η`.
    pub reference_eta: f64,
    /// The order expected for the stepper: RK4's 4 (REQ-INT-119).
    pub order_expected: f64,
    /// How far the fitted order may lie from `order_expected`, either way (REQ-INT-119; the criterion written before
    /// any run, R-649).
    pub order_margin: f64,
    /// The round-off floor the fit stays above, where the input states one ([`FloorCriterion`]); with none, every rung
    /// is fitted.
    #[serde(default)]
    pub floor: Option<FloorCriterion>,
}

/// The convergence arm's round-off floor (REQ-INT-119's second criterion): the floor `F` is the distance between the
/// reference run's end and a second reference run's at `second_eta`, both far below the ladder, so that their
/// truncation errors are negligible and what parts them is rounding; a rung is fitted only where its error exceeds
/// `factor·F`, and the arm passes only where at least `min_rungs` rungs are fitted.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorCriterion {
    /// The second reference run's `η`.
    pub second_eta: f64,
    /// How far above the floor a fitted rung's error must be.
    pub factor: f64,
    /// The fewest rungs the fit takes.
    pub min_rungs: usize,
}

/// The rungs `(η, error)` of `rungs` whose error exceeds `factor·floor`, in ladder order.
pub fn above_floor(rungs: &[(f64, f64)], floor: f64, factor: f64) -> Vec<(f64, f64)> {
    rungs
        .iter()
        .copied()
        .filter(|&(_, e)| e > factor * floor)
        .collect()
}

/// The order fitted on `rungs` `(η, error)`: the least-squares slope of `ln e` against `ln η` ([`slope`]).
pub fn fitted_order(rungs: &[(f64, f64)]) -> f64 {
    let points: Vec<(f64, f64)> = rungs.iter().map(|&(h, e)| (h.ln(), e.ln())).collect();
    slope(&points)
}

/// The lines describing one run.
pub fn describe(c: &Closure) -> String {
    let dt_max = match c.count.dt_max.report() {
        DtMaxReport::Largest { dt_max, skipped } => {
            format!("dt_max {dt_max:.3e} ({skipped} skipped)")
        }
        DtMaxReport::NoUsableStep { skipped } => format!("no usable step ({skipped} skipped)"),
    };
    format!(
        "η = {}, n = {}, {}: closure {:.4e}, |ΔE/E| {:.2e}; {} steps, {} re-registrations, {} reference \
         switches, the predictive limit bound {} step(s), {} cap hits, {} overshoots, {} interval(s) closed by a \
         further step, at most {} halving(s) in a landing, {dt_max}",
        c.eta,
        c.n,
        c.landing.name(),
        c.closure,
        c.energy,
        c.count.steps,
        c.count.re_registrations,
        c.count.switches,
        c.count.predictive_bound,
        c.count.cap_hits,
        c.count.overshoots,
        c.count.closings,
        c.count.most_bisections
    )
}

/// The `az-final-step` gate (REQ-INT-049; its threshold REQ-INT-114's).
pub struct AzFinalStepGate;

impl Gate for AzFinalStepGate {
    fn name(&self) -> &'static str {
        FINAL_STEP_NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input =
            FinalStepInput::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
        if input.ladder.len() < 2 || input.n == 0 {
            return Err(format!(
                "refused: {name}: a ladder of two rungs or more, and n ≥ 1"
            ));
        }
        let o = orbit()?;
        let c = closure(&o, input.eta, input.n, input.landing);
        let conv = convergence(
            &o,
            &input.ladder,
            input.reference_eta,
            input.n,
            input.landing,
        );
        let threshold = config.threshold.value;
        let mut scatter = vec![describe(&c)];
        for (r, e) in &conv.rungs {
            scatter.push(format!(
                "rung: {}; error against the reference run {e:.3e}",
                describe(r)
            ));
        }
        scatter.push(format!("reference run: {}", describe(&conv.reference)));
        scatter.push(format!(
            "order fitted on the ladder {:.3} (η {} → {}; RK4's {} ± {})",
            conv.order,
            input.ladder[0],
            input.ladder[input.ladder.len() - 1],
            input.order_expected,
            input.order_margin
        ));
        scatter.extend(
            o.reference
                .iter()
                .map(|r| format!("the reference's figure, a measurement: {r}")),
        );
        let mut negative = Vec::new();
        let closes = c.closure <= threshold;
        let (order, enough) = match input.floor {
            None => (conv.order, true),
            Some(fc) => {
                let second = closure(&o, fc.second_eta, input.n, Landing::Fix);
                let floor = distance(&conv.reference.end, &second.end);
                let pairs: Vec<(f64, f64)> = conv.rungs.iter().map(|(r, e)| (r.eta, *e)).collect();
                let kept = above_floor(&pairs, floor, fc.factor);
                let enough = kept.len() >= fc.min_rungs;
                scatter.push(format!(
                    "the round-off floor: {floor:.3e}, the reference runs at η = {} and {} apart; the rungs above \
                     {}× it: {:?}; {} of at least {}",
                    input.reference_eta,
                    fc.second_eta,
                    fc.factor,
                    kept.iter().map(|k| k.0).collect::<Vec<_>>(),
                    kept.len(),
                    fc.min_rungs
                ));
                let order = if kept.len() >= 2 {
                    fitted_order(&kept)
                } else {
                    f64::NAN
                };
                scatter.push(format!(
                    "order fitted on the rungs above the floor {order:.3}"
                ));
                (order, enough)
            }
        };
        let converges = enough && (order - input.order_expected).abs() <= input.order_margin;
        if !enough {
            negative.push("too few rungs above the round-off floor to fit an order".to_owned());
        }
        negative.push(if closes {
            format!(
                "the closure {:.4e} is within the threshold {threshold:.3e}",
                c.closure
            )
        } else {
            format!(
                "the closure {:.4e} exceeds the threshold {threshold:.3e}",
                c.closure
            )
        });
        negative.push(if converges {
            format!(
                "the fitted order {order:.3} is within {} of {}",
                input.order_margin, input.order_expected
            )
        } else {
            format!(
                "the fitted order {order:.3} is not within {} of {}",
                input.order_margin, input.order_expected
            )
        });
        Ok(GateReport {
            gate: FINAL_STEP_NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if closes && converges {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(u32::try_from(input.ladder.len()).unwrap_or(u32::MAX)),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        let text = match proposal(args)? {
            Proposal::FinalStep => {
                propose_final_step_with(0.001, 32, &LADDER, REFERENCE_ETA, 65_536)
            }
            Proposal::Sync => propose_sync_with(&LADDER, &SYNC_LADDER, REFERENCE_ETA),
        }?;
        print!("{text}");
        Ok(())
    }
}

/// The proposals `az-final-step` gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proposal {
    /// `--propose`: REQ-INT-114's, at `η = 0.001`, `n = 32`, the ladder [`LADDER`] against [`REFERENCE_ETA`], the IC's
    /// own closure at `T/65536`.
    FinalStep,
    /// `--propose-sync`: REQ-INT-080's, over [`LADDER`] and [`SYNC_LADDER`] against [`REFERENCE_ETA`].
    Sync,
}

/// The proposal `args` asks `az-final-step` for, or the usage.
pub fn proposal(args: &[&str]) -> Result<Proposal, String> {
    match args {
        ["--propose"] => Ok(Proposal::FinalStep),
        ["--propose-sync"] => Ok(Proposal::Sync),
        _ => Err(format!(
            "gate {FINAL_STEP_NAME}: unrecognised flags `{}`; usage: --propose | --propose-sync",
            args.join(" ")
        )),
    }
}

/// The reference run's `η` for the convergence ladder: a tenth of the ladder's least.
pub const REFERENCE_ETA: f64 = 1e-4;

/// The proposed threshold's margin over the measured closure (REQ-INT-114).
pub const MARGIN: f64 = 2.0;

/// REQ-INT-114's proposal: the closure at `eta` on `n` intervals (the flag's `η = 0.001`, `n = 32`), the convergence
/// over `ladder` against a run at `reference_eta`, the closure without the landing fix beside it, the IC's own closure
/// under Cartesian RK4 at `T/ic_steps`, and the threshold proposed from the measured closure with its margin.
pub fn propose_final_step_with(
    eta: f64,
    n: u32,
    ladder: &[f64],
    reference_eta: f64,
    ic_steps: u32,
) -> Result<String, String> {
    let mut out = String::new();
    let o = orbit()?;
    say!(
        out,
        "# REQ-INT-114: the figure-eight closure threshold, proposed (az-final-step --propose)\n"
    );
    say!(out, "{}\n", o.description);
    let c = closure(&o, eta, n, Landing::Fix);
    let off = closure(&o, eta, n, Landing::Off);
    say!(
        out,
        "- the closure under the rule in force: {}",
        describe(&c)
    );
    say!(
        out,
        "- the closure without the landing fix: {}",
        describe(&off)
    );
    let on = convergence(&o, ladder, reference_eta, n, Landing::Fix);
    let clock_only = convergence(&o, ladder, reference_eta, n, Landing::ClockOnly);
    let without = convergence(&o, ladder, reference_eta, n, Landing::Off);
    for (what, conv) in [
        ("with the landing fix", &on),
        ("with the clock-only landing", &clock_only),
        ("without the landing fix", &without),
    ] {
        say!(out, "\n## The convergence ladder, {what} (errors against η = {reference_eta}, landing on)\n");
        say!(
            out,
            "| η | closure | error | steps | re-registrations | predictive limit bound | closed by a further step \
             | most halvings |"
        );
        say!(out, "|---|---|---|---|---|---|---|---|");
        for (r, e) in &conv.rungs {
            say!(
                out,
                "| {} | {:.4e} | {e:.3e} | {} | {} | {} | {} | {} |",
                r.eta,
                r.closure,
                r.count.steps,
                r.count.re_registrations,
                r.count.predictive_bound,
                r.count.closings,
                r.count.most_bisections
            );
        }
        say!(out, "\norder fitted on the ladder: {:.3}", conv.order);
    }
    say!(out, "\nreference run: {}", describe(&on.reference));
    // The IC's own closure: Cartesian RK4 at a fine fixed step, no regularisation.
    let floor = cartesian_closure(&o, ic_steps);
    say!(out,
        "\nthe IC's own closure, Cartesian RK4 at dt = T/{ic_steps} with no regularisation: {floor:.4e}; the published IC \
         and period are given to 8 and 9 digits, so no integrator closes it below this"
    );
    for r in &o.reference {
        say!(
            out,
            "the reference's figure, a measurement, never the threshold: {r}"
        );
    }
    say!(out,
        "\n**Proposed threshold:** {:.1e}, {MARGIN}× the measured closure {:.4e} rounded up; the order criterion: RK4's \
         4 ± 0.5, fitted on the ladder (REQ-INT-119; measured {:.3} with the fix, {:.3} with the clock-only landing, \
         {:.3} without the fix)",
        round_up(MARGIN * c.closure),
        c.closure,
        on.order,
        clock_only.order,
        without.order
    );
    Ok(out)
}

/// `x` rounded up to two significant digits.
pub fn round_up(x: f64) -> f64 {
    let e = x.log10().floor() - 1.0;
    let scale = 10f64.powf(e);
    (x / scale).ceil() * scale
}

/// The figure-eight's closure under Cartesian RK4 at the fixed step `T/steps`, the kernel's stepper with no
/// regularisation.
pub fn cartesian_closure(o: &Orbit, steps: u32) -> f64 {
    use kernel::occupant::Stepper;
    let s0 = figure_eight::<f64>(o);
    let dt = o.period / f64::from(steps);
    let mut s = s0;
    let mut g = Geom::new(&s.m, &s.r);
    for _ in 0..steps {
        (s, g) = Rk4::step(&s, dt, &g);
    }
    distance(&positions(&s), &positions(&s0))
}

/// The sync quanta REQ-INT-080's proposal measures: the production grid's scale `13/32` down to a display frame.
pub const SYNC_LADDER: [f64; 6] = [0.40625, 0.2, 0.1, 0.05, 0.025, 0.0125];

/// The figure-eight from `t = 0` to the last boundary at or before its period on the grid `delta_sync`, at `eta`:
/// the positions there, `|ΔE/E|`, the counts.
pub fn on_grid(o: &Orbit, eta: f64, delta_sync: f64) -> (AzAdvanced<f64>, f64) {
    let s0 = figure_eight::<f64>(o);
    let params = conditions::<f64>(eta, delta_sync, Landing::Fix);
    let k = kernel::regularisation::az::boundary_at_or_before(o.period, delta_sync);
    let out = march::<Rk4, f64, Quiet, ExplicitTest>(
        &s0,
        0,
        k,
        AzCount::new(),
        &params,
        &mut Quiet,
        &mut ExplicitTest,
    );
    let de = ((energy(&out.state) - energy(&s0)) / energy(&s0)).abs();
    (out, de)
}

/// One row of REQ-INT-080's measurement: the quantum, the step factor, the error against the reference run and the
/// cost in steps per unit time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyncRow {
    pub delta: f64,
    pub eta: f64,
    pub error: f64,
    pub rate: f64,
}

/// The largest error at `Δ_sync = 0.1` the proposed `η` may have (the criterion fixed before the run).
pub const SYNC_ERROR: f64 = 1e-9;

/// REQ-INT-080's decision on `table`, under the criteria fixed before the run: `η` the first of `ladder` (largest
/// first) whose error at `Δ_sync = 0.1` is at most [`SYNC_ERROR`], the last rung where none is; `Δ_sync` 0.1 if, at
/// that `η`, its error is within 10× and its cost within 1.1× of the grid `coarse`'s, else 0.2, the next larger
/// quantum.
pub fn decide_sync(table: &[SyncRow], ladder: &[f64], coarse: f64) -> (f64, f64) {
    let at = |d: f64, e: f64| table.iter().find(|r| r.delta == d && r.eta == e).copied();
    let eta = ladder
        .iter()
        .copied()
        .find(|&e| at(0.1, e).is_some_and(|r| r.error <= SYNC_ERROR))
        .unwrap_or(ladder[ladder.len() - 1]);
    let keep_tenth = match (at(coarse, eta), at(0.1, eta)) {
        (Some(c), Some(t)) => t.error <= 10.0 * c.error.max(1e-16) && t.rate <= 1.1 * c.rate,
        _ => false,
    };
    (eta, if keep_tenth { 0.1 } else { 0.2 })
}

/// REQ-INT-080's proposal: `Δ_sync` and `η` for Aarseth–Zare, from the figure-eight's error and drift over the `η`
/// ladder `ladder` and the quanta `sync` (the first the coarse grid), each run against one at `reference_eta`, and the
/// cost in steps per unit time, under the criteria fixed before the run ([`decide_sync`]).
pub fn propose_sync_with(
    ladder: &[f64],
    sync: &[f64],
    reference_eta: f64,
) -> Result<String, String> {
    let mut out = String::new();
    let o = orbit()?;
    say!(
        out,
        "# REQ-INT-080: Aarseth–Zare's Δ_sync and η, proposed (az-final-step --propose-sync)\n"
    );
    say!(out,
        "The figure-eight (unit masses) from t = 0 to the last boundary at or before its period, each run against \
         one at η = {reference_eta} on the same grid; the error is |r − r_ref| over every position component.\n"
    );
    say!(out, "| Δ_sync | η | t reached | error | \\|ΔE/E\\| | steps per unit time | re-registrations | switches |");
    say!(out, "|---|---|---|---|---|---|---|---|");
    let mut table = Vec::new();
    for &d in sync {
        let (reference, _) = on_grid(&o, reference_eta, d);
        for &eta in ladder {
            let (run, de) = on_grid(&o, eta, d);
            let error = distance(&positions(&run.state), &positions(&reference.state));
            let rate = f64::from(run.count.steps) / run.t;
            say!(
                out,
                "| {d} | {eta} | {:.6} | {error:.3e} | {de:.2e} | {rate:.1} | {} | {} |",
                run.t,
                run.count.re_registrations,
                run.count.switches
            );
            table.push(SyncRow {
                delta: d,
                eta,
                error,
                rate,
            });
        }
    }
    let coarse = sync[0];
    let (eta, proposed) = decide_sync(&table, ladder, coarse);
    say!(out,
        "\nThe criteria, fixed before the run: η is the largest on the ladder whose error at Δ_sync = 0.1 is at most \
         {SYNC_ERROR:e}; Δ_sync is 0.1 if, at that η, its error is within 10× and its cost within 1.1× of the {coarse} \
         grid's, else 0.2, the next larger quantum on the ladder."
    );
    say!(out,
        "\n**Proposed:** η = {eta}; Δ_sync = {proposed}. The closure window's 0.4 time units are {:.2} boundaries at \
         Δ_sync = {proposed} (0.4/Δ_sync at each quantum: {}).",
        0.4 / proposed,
        sync.iter().map(|d| format!("{d}: {:.2}", 0.4 / d)).collect::<Vec<_>>().join(", ")
    );
    let row = |d: f64| table.iter().find(|r| r.delta == d && r.eta == eta).copied();
    let base = row(coarse).map_or(f64::NAN, |c| c.rate);
    say!(out,
        "Aarseth–Zare's cost at η = {eta}, steps per unit time against the {coarse} grid's: {} (its entry size \
         η·Δ_sync/(A·B) scales with Δ_sync, so its steps per interval do not).",
        sync
            .iter()
            .filter_map(|&d| row(d).map(|r| format!("{d}: {:.2}×", r.rate / base)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    say!(out,
        "Part 2a's cost and label-flip evidence (Heggie, prin-rs sync_grid_probe): 0.1 costs 1–3% more steps than the \
         13/32 grid, 0.025 17–25%, 0.0125 40–52%; 0, at most 1 and at most 4 of 1024 outcome labels flip. Aarseth–Zare's \
         own label flips are not measured here: no detector runs under a regularised occupant yet."
    );
    Ok(out)
}

// ── separation-floor ───────────────────────────────────────────────────────────────────────────────────────────

/// The `separation-floor` gate's name.
pub const FLOOR_NAME: &str = "separation-floor";

/// What a [`Recorder`] saw: the least pre-floor `A`, `B` and `A·B` over a march's steps, and the least guarded
/// component of the inverse at its registrations, at f64: the component the conditioned inverse tests against `TINY`
/// and divides by, `u₀ = a` on `x ≥ 0` and `|u₁| = b` on `x < 0`, which is the larger of `|u₀|` and `|u₁|`, at least
/// `√(|R|/2)` (Part 2b, the conditioned inverse).
#[derive(Clone, Copy, Debug, Default)]
pub struct Least {
    pub a: Option<f64>,
    pub b: Option<f64>,
    pub ab: Option<f64>,
    pub guard: Option<f64>,
}

/// `min(slot, x)`, seeded from the first.
fn fold(slot: &mut Option<f64>, x: f64) {
    *slot = Some(slot.map_or(x, |s| s.min(x)));
}

/// A watch that runs the occupant's finiteness test and records the least pre-floor values ([`Least`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct Recorder {
    pub least: Least,
}

impl<R: Width> Watch<R> for Recorder {
    fn finite(&mut self, q: &AzQ<R>, p: &AzP<R>) -> bool {
        ExplicitTest.finite(q, p)
    }

    fn registered(&mut self, q: &AzQ<R>) {
        for u in [q.u1, q.u2] {
            let (u0, u1): (f64, f64) = (u[0].into(), u[1].into());
            fold(&mut self.least.guard, u0.abs().max(u1.abs()));
        }
    }

    fn sized(&mut self, a: R, b: R) {
        fold(&mut self.least.a, a.into());
        fold(&mut self.least.b, b.into());
        fold(&mut self.least.ab, (a * b).into());
    }
}

/// A fixture of the floor's measurement: its name, its state at f64, the grid and the boundaries it runs to.
#[derive(Clone, Copy, Debug)]
pub struct FloorFixture {
    pub name: &'static str,
    pub state: State<f64>,
    pub delta_sync: f64,
    pub boundaries: u32,
}

/// The radial collision passage: two of three equal masses (`1/3`) from rest at `(±1/2, 0)`, the third far away at
/// rest at `(0, 20)`, on the perpendicular bisector, so the pair falls head-on and collides at about `t = 1.36`; run to
/// `t = 2`. A stand-in for REQ-VAL-045's gate (dd_validation_orbits § 2: "equal masses from rest, third body far
/// away"), which TASK-M3-08 builds.
pub fn radial() -> FloorFixture {
    let m = 1.0 / 3.0;
    FloorFixture {
        name: "the radial collision passage",
        state: State {
            m: [m; 3],
            r: [[-0.5, 0.0], [0.5, 0.0], [0.0, 20.0]],
            p: [[0.0; 2]; 3],
        },
        delta_sync: 0.125,
        boundaries: 16,
    }
}

/// dd_integrator § 5 test 7's close-encounter IC ([`crate::drift::close_encounter`]), to `t = 10`.
pub fn hierarchy() -> FloorFixture {
    let ic = crate::drift::close_encounter().ic;
    FloorFixture {
        name: "test 7's close-encounter hierarchy",
        state: State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        },
        delta_sync: 0.125,
        boundaries: 80,
    }
}

/// Burrau's configuration (the triple `(3, 4, 5)`, normalised, from rest; `kernel::burrau`), its close encounters,
/// to `t = 20`.
pub fn burrau() -> FloorFixture {
    let s = kernel::burrau::euclid(2, 1).map(|t| t.burrau::<f64>());
    let s = s.expect("(2, 1) is primitive");
    FloorFixture {
        name: "Burrau's (3, 4, 5) from rest",
        state: State {
            m: s.m,
            r: s.r,
            p: [[0.0; 2]; 3],
        },
        delta_sync: 0.125,
        boundaries: 160,
    }
}

/// One fixture's floor measurement at `R`, at `TINY = tiny` (the reference's where `None`), `η = 0.01`.
#[derive(Clone, Copy, Debug)]
pub struct FloorRun {
    pub least: Least,
    pub floored: u32,
    pub steps: u32,
    pub non_finite_entries: u32,
    pub end: End,
    pub t: f64,
}

/// The step factor of the floor's runs.
pub const FLOOR_ETA: f64 = 0.01;

/// Runs `f` at `R` with `TINY = tiny`.
pub fn floor_run<R: Width>(f: &FloorFixture, tiny: Option<f64>) -> FloorRun {
    let mut params = conditions::<R>(FLOOR_ETA, f.delta_sync, Landing::Fix);
    if let Some(t) = tiny {
        params.tiny = at(t);
    }
    let s = State {
        m: f.state.m.map(at),
        r: f.state.r.map(|v| v.map(at)),
        p: f.state.p.map(|v| v.map(at)),
    };
    let mut watch = Recorder::default();
    let out = march::<Rk4, R, Quiet, Recorder>(
        &s,
        0,
        f.boundaries,
        AzCount::new(),
        &params,
        &mut Quiet,
        &mut watch,
    );
    FloorRun {
        least: watch.least,
        floored: out.count.floored,
        steps: out.count.steps,
        non_finite_entries: out.count.non_finite_entries,
        end: out.count.end,
        t: out.t.into(),
    }
}

/// `Some(x)` as `x:e`, `None` as "none".
pub fn show(x: Option<f64>) -> String {
    x.map_or_else(|| "none".to_owned(), |x| format!("{x:.3e}"))
}

/// REQ-INT-105's proposal for Aarseth–Zare: `TINY` at f32 and f64, from each fixture's least pre-floor `A`, `B`,
/// `A·B` and guarded component, the floored steps, the underflow of `TINY·TINY`, the least `A·B` against
/// `η·dt_left/FLT_MAX` and the non-finite entry steps; the reference's values beside.
pub fn propose_floor() -> Result<String, String> {
    let mut out = String::new();
    say!(
        out,
        "# REQ-INT-105: the floor TINY for Aarseth–Zare, proposed (separation-floor --propose)\n"
    );
    say!(out,
        "Each fixture run under Aarseth–Zare + RK4 at η = {FLOOR_ETA}, Δ_sync = 0.125, with TINY the reference's \
         (1e-37 at f32, 1e-300 at f64; R-953), which also floors |R₃| (R-999), and the landing with no tolerance \
         (R-998).\n"
    );
    say!(out,
        "| fixture | width | least A | least B | least A·B | least guarded component | floored | non-finite entries \
         | steps | end |"
    );
    say!(out, "|---|---|---|---|---|---|---|---|---|---|");
    let fixtures = [radial(), hierarchy(), burrau()];
    let mut least_f32 = Least::default();
    let mut least_f64 = Least::default();
    for f in &fixtures {
        for (width, run, least) in [
            ("f32", floor_run::<f32>(f, None), &mut least_f32),
            ("f64", floor_run::<f64>(f, None), &mut least_f64),
        ] {
            say!(
                out,
                "| {} | {width} | {} | {} | {} | {} | {} | {} | {} | {:?} at t = {:.4} |",
                f.name,
                show(run.least.a),
                show(run.least.b),
                show(run.least.ab),
                show(run.least.guard),
                run.floored,
                run.non_finite_entries,
                run.steps,
                run.end,
                run.t
            );
            for (slot, x) in [
                (&mut least.a, run.least.a),
                (&mut least.b, run.least.b),
                (&mut least.ab, run.least.ab),
            ] {
                if let Some(x) = x {
                    fold(slot, x);
                }
            }
        }
    }
    let tiny32 = 1e-37f32;
    let entry_floor = FLOOR_ETA * 0.125 / f64::from(f32::MAX);
    say!(out,
        "\nTINY·TINY at the reference's values: {:e} at f32 (1e-37·1e-37) and {:e} at f64 (1e-300·1e-300); where it is 0, \
         a step with both factors floored has an infinite entry size at that width.",
        tiny32 * tiny32,
        1e-300f64 * 1e-300
    );
    say!(out,
        "A step with both factors floored at f32 has an infinite entry size and ends its march at the explicit \
         finiteness test, its state latched (kernel test az_doubly_degenerate_is_finite): {:?}",
        doubly_degenerate()
    );
    say!(out,
        "The least pre-floor A·B at f32 over the fixtures, {}, against η·dt_left/FLT_MAX = {entry_floor:.3e}, below which \
         the f32 entry size is not finite with neither factor floored.",
        show(least_f32.ab)
    );
    let decades = |least: Option<f64>, tiny: f64| least.map_or(f64::NAN, |l| (l / tiny).log10());
    say!(out,
        "\n**Proposed:** TINY = 1e-37 at f32 and 1e-300 at f64, the reference's values: the least pre-floor A and B \
         over the fixtures sit {:.1} and {:.1} decades above them at f32, {:.1} and {:.1} at f64, so no step of these \
         fixtures is floored, and the floor stays a guard against a degenerate state, not a step-size rule.",
        decades(least_f32.a, 1e-37),
        decades(least_f32.b, 1e-37),
        decades(least_f64.a, 1e-300),
        decades(least_f64.b, 1e-300)
    );
    Ok(out)
}

/// The doubly degenerate f32 state's march (the kernel test's positions, at unit masses, which the degeneracy does not
/// depend on): its end, steps and non-finite entries.
pub fn doubly_degenerate() -> (End, u32, u32) {
    let s = State {
        m: [1.0; 3],
        r: [[0.0, 0.0], [1e-38, 0.0], [0.0, 2e-38]],
        p: [[0.0; 2]; 3],
    };
    let mut params = conditions::<f32>(FLOOR_ETA, 0.125, Landing::Fix);
    params.n_min = 64;
    let out = march::<Rk4, f32, Quiet, ExplicitTest>(
        &s,
        0,
        1,
        AzCount::new(),
        &params,
        &mut Quiet,
        &mut ExplicitTest,
    );
    (out.count.end, out.count.steps, out.count.non_finite_entries)
}

/// A `separation-floor` control input.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorControl {
    pub description: String,
    /// `"above"`: a `TINY` ten times the passage's least pre-floor `A·B`'s smaller factor; `"reference"`: the
    /// reference's.
    pub tiny: String,
}

/// The `separation-floor` measurement gate (REQ-INT-105).
pub struct SeparationFloorGate;

impl MeasurementGate for SeparationFloorGate {
    fn name(&self) -> &'static str {
        FLOOR_NAME
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let control = FloorControl::deserialize(fixture).map_err(|e| format!("{name}: {e}"))?;
        let passage = radial();
        let base = floor_run::<f64>(&passage, None);
        let least = base.least.a.zip(base.least.b).map(|(a, b)| a.min(b));
        let tiny = match control.tiny.as_str() {
            "above" => least
                .map(|l| 10.0 * l)
                .ok_or(format!("{name}: the passage took no step"))?,
            "reference" => 1e-300,
            other => return Err(format!("{name}: no TINY `{other}` (above or reference)")),
        };
        let run = floor_run::<f64>(&passage, Some(tiny));
        let markdown = format!(
            "# separation-floor: {name}\n\n{}\n\n- the passage's least pre-floor A and B at the reference's TINY: {}, \
             {}\n- TINY = {tiny:.3e}: {} floored step(s) of {}, the march {:?} at t = {:.4}\n",
            control.description,
            show(base.least.a),
            show(base.least.b),
            run.floored,
            run.steps,
            run.end,
            run.t
        );
        Ok(MeasurementControl {
            description: control.description,
            verdict: if run.floored > 0 {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            markdown,
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => propose_floor().map(|s| print!("{s}")),
            _ => Err(format!(
                "gate {FLOOR_NAME}: unrecognised flags `{}`; usage: --propose",
                args.join(" ")
            )),
        }
    }
}
