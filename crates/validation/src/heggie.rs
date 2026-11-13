//! Heggie's gates and measurements (TASK-M3-08; integrator_contract Part 2b § "Heggie: the equations, the time
//! transformations, the maps and the step control (transcribed)", Part 1, Part 6; R-160, R-161, R-257, R-566):
//!
//! - **`radial-collision`** ([`RadialCollisionGate`]; REQ-VAL-045): the two-body radial collision, equal masses from rest
//!   with the third body far away ([`crate::az::radial`]), marched through the passage by Heggie × RK4 under Eq. 22,
//!   registered once at `t = 0`, and by Aarseth–Zare + RK4: it passes when the closest approach is below `1e-10` and the
//!   regularised energy's drift below the threshold, `1e-12` (dd_validation_orbits § 2, R-257), `E` the regularised
//!   energy (R-566). Its controls: the value the gate reads is bitwise the occupant's own regularised drift, so a reader
//!   pointed at the Cartesian read-out fails; the Cartesian read-out after the passage is printed beside it and, under
//!   Heggie, differs from it (Aarseth–Zare re-registers at every sync boundary with `E` the Cartesian energy there, so
//!   its two read-outs may coincide, and that arm is Heggie's alone; applied per R-369); and over a halving ladder of `η` the Cartesian read-out does not fall with `η` (the design's § 11 control),
//!   "falls" meaning that the ladder's finest rung reads at least `2^(n − 1)` times below its coarsest, `n` rungs, the
//!   first-order fall an integration error would show (the criterion written before the run).
//! - **`hamiltonian-fd`** ([`HamiltonianFdGate`]; REQ-VAL-170, REQ-VAL-171): each occupant's analytic rates against a
//!   central finite difference of its Hamiltonian over random states: Heggie's `Γ*` under Eq. 20 and `Γ** = Γ*·S^{−3/2}`
//!   under Eq. 22, and Aarseth–Zare's `Γ`, written here from Part 2b, not taken from the kernel; the position rates
//!   against `∂Γ/∂p`, the momentum rates against `−∂Γ/∂q` and `dt/dτ` against `−∂Γ/∂h` (`−∂Γ/∂E`). The discrepancy is
//!   the largest difference over the components divided by the largest analytic rate, at a state; the gate reads its
//!   largest over the states. Its control flips one derivative's sign. `--propose` gives REQ-VAL-171's step and
//!   tolerance.
//! - **`separation-floor`**'s Heggie arm ([`propose_floor_heggie`], [`floor_control_heggie`]; REQ-INT-105): Heggie's
//!   pre-floor `dt/dτ`, `S` and `Rᵢ` under Eq. 20 and Eq. 22 over the floor's fixtures, with the floored steps.
//!
//! **The conditions every run states**, as Aarseth–Zare's gates do ([`crate::az`]): `TINY` the reference's (`1e-37` at
//! f32, `1e-300` at f64; R-953) unless the run varies it, the landing tolerance the reference's `LAND_EPS_REL` (`1e-5`,
//! `1e-14`; RQ-795), R-160's `f = 0.02`, a step budget that does not bind, the clamp and the secant on, `Δ_sync = 1/8`,
//! and the stepper RK4.

use std::path::Path;

use serde::Deserialize;

use kernel::driver::advance::Cadence;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::regularisation::az::{self as kaz, AzP, AzQ, AzSystem, End, ExplicitTest};
use kernel::regularisation::form::Flow;
use kernel::regularisation::heggie::{
    self as hg, HgP, HgParams, HgQ, HgSystem, HgWatch, DEFAULT_TIME,
};
use kernel::regularisation::pairs::HeggieTime;

use crate::az::{at, conditions, energy, radial, reference_tiny, show, FloorFixture, Width};
use crate::decode_f32_eps::SplitMix;
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};

/// Appends one formatted line to a text.
macro_rules! say {
    ($out:expr, $($arg:tt)*) => {{
        $out.push_str(&format!($($arg)*));
        $out.push('\n');
    }};
}

/// Heggie's step-control values at `R` (the module's note), at step factor `eta` and sync quantum `delta_sync`, under
/// `time`.
pub fn hg_conditions<R: Width>(eta: f64, delta_sync: f64, time: HeggieTime) -> HgParams<R> {
    let c = conditions::<R>(eta, delta_sync, true);
    HgParams {
        eta: c.eta,
        delta_sync: c.delta_sync,
        tiny: c.tiny,
        predictive_f: c.predictive_f,
        land_eps_rel: c.land_eps_rel,
        step_budget_rate: c.step_budget_rate,
        n_min: c.n_min,
        clamp: true,
        secant_tries: 4,
        time,
    }
}

/// `s` at `R`'s width.
pub fn state_at<R: Width>(s: &State<f64>) -> State<R> {
    State {
        m: s.m.map(at),
        r: s.r.map(|v| v.map(at)),
        p: s.p.map(|v| v.map(at)),
    }
}

/// The least pair separation of the positions `r`, at f64.
pub fn closest<R: Width>(r: &[[R; 2]; 3]) -> f64 {
    let r: [[f64; 2]; 3] = r.map(|v| v.map(Into::into));
    let d = |i: usize, j: usize| (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]);
    d(0, 1).min(d(0, 2)).min(d(1, 2))
}

/// A cadence that keeps the least pair separation of every kept step's Cartesian image and never ends the march.
#[derive(Clone, Copy, Debug)]
pub struct Closest {
    /// The least separation seen; `+∞` before any step.
    pub d_min: f64,
}

impl Default for Closest {
    fn default() -> Self {
        Closest {
            d_min: f64::INFINITY,
        }
    }
}

impl<R: Width> Cadence<R> for Closest {
    fn after_step(&mut self, state: &mut State<R>, _geom: &mut Geom<R>) {
        self.d_min = self.d_min.min(closest(&state.r));
    }

    fn done(&self) -> bool {
        false
    }
}

// ── separation-floor: Heggie's pre-floor values ─────────────────────────────────────────────────────────────────

/// What an [`HgRecorder`] saw: the least pre-floor `dt/dτ`, `S` and `Rᵢ` over a march's steps, at f64.
#[derive(Clone, Copy, Debug, Default)]
pub struct HgLeast {
    pub dt_dtau: Option<f64>,
    pub s: Option<f64>,
    pub r: Option<f64>,
}

/// `min(slot, x)`, seeded from the first.
fn fold(slot: &mut Option<f64>, x: f64) {
    *slot = Some(slot.map_or(x, |s| s.min(x)));
}

/// A watch that runs the occupant's finiteness test and records the least pre-floor values ([`HgLeast`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct HgRecorder {
    pub least: HgLeast,
}

impl<R: Width> HgWatch<R> for HgRecorder {
    fn finite(&mut self, q: &HgQ<R>, p: &HgP<R>) -> bool {
        HgWatch::finite(&mut ExplicitTest, q, p)
    }

    fn sized(&mut self, dt_dtau: R, s: R, r_least: R) {
        fold(&mut self.least.dt_dtau, dt_dtau.into());
        fold(&mut self.least.s, s.into());
        fold(&mut self.least.r, r_least.into());
    }
}

/// One fixture's Heggie floor measurement.
#[derive(Clone, Copy, Debug)]
pub struct HgFloorRun {
    pub least: HgLeast,
    pub floored: u32,
    pub steps: u32,
    pub end: End,
    pub t: f64,
}

/// Runs `f` under Heggie × RK4 at `R` under `time`, `TINY = tiny` (the reference's where `None`), `η = 0.01`.
pub fn hg_floor_run<R: Width>(f: &FloorFixture, time: HeggieTime, tiny: Option<f64>) -> HgFloorRun {
    let mut params = hg_conditions::<R>(crate::az::FLOOR_ETA, f.delta_sync, time);
    if let Some(t) = tiny {
        params.tiny = at(t);
    }
    let mut watch = HgRecorder::default();
    let out = hg::march::<Rk4, R, crate::az::Quiet, HgRecorder>(
        &state_at::<R>(&f.state),
        f.boundaries,
        &params,
        &mut crate::az::Quiet,
        &mut watch,
    );
    HgFloorRun {
        least: watch.least,
        floored: out.count.floored,
        steps: out.count.steps,
        end: out.count.end,
        t: out.t.into(),
    }
}

/// The name of a time transformation.
pub fn time_name(time: HeggieTime) -> &'static str {
    match time {
        HeggieTime::Eq20 => "Eq. 20",
        HeggieTime::Eq22 => "Eq. 22",
    }
}

/// REQ-INT-105's Heggie arm: each fixture's least pre-floor `dt/dτ`, `S` and `Rᵢ` under Heggie × RK4, Eq. 20 and Eq. 22,
/// at f32 and f64, with the floored steps, against the reference's `TINY`.
pub fn propose_floor_heggie() -> String {
    let mut out = String::new();
    say!(out, "\n## Heggie's arm (TASK-M3-08)\n");
    say!(out,
        "Each fixture run under Heggie × RK4, registered once at t = 0, at η = {}, Δ_sync = 0.125, TINY the reference's \
         (1e-37 at f32, 1e-300 at f64; R-953), the landing tolerance the reference's (RQ-795). The floor acts where \
         dt/dτ < TINY, S < TINY under Eq. 22, or any Rᵢ < TINY (integrator_contract Part 6).\n",
        crate::az::FLOOR_ETA
    );
    say!(
        out,
        "| fixture | time | width | least dt/dτ | least S | least Rᵢ | floored | steps | end |"
    );
    say!(out, "|---|---|---|---|---|---|---|---|---|");
    let mut least32: Option<f64> = None;
    let mut least64: Option<f64> = None;
    for f in [radial(), crate::az::hierarchy(), crate::az::burrau()] {
        for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
            for (width, run, least) in [
                ("f32", hg_floor_run::<f32>(&f, time, None), &mut least32),
                ("f64", hg_floor_run::<f64>(&f, time, None), &mut least64),
            ] {
                say!(
                    out,
                    "| {} | {} | {width} | {} | {} | {} | {} | {} | {:?} at t = {:.4} |",
                    f.name,
                    time_name(time),
                    show(run.least.dt_dtau),
                    show(run.least.s),
                    show(run.least.r),
                    run.floored,
                    run.steps,
                    run.end,
                    run.t
                );
                for x in [run.least.dt_dtau, run.least.r].into_iter().flatten() {
                    fold(least, x);
                }
            }
        }
    }
    let decades = |least: Option<f64>, tiny: f64| least.map_or(f64::NAN, |l| (l / tiny).log10());
    say!(out,
        "\nHeggie's least pre-floor dt/dτ or Rᵢ over the fixtures sits {:.1} decades above 1e-37 at f32 and {:.1} above \
         1e-300 at f64.",
        decades(least32, 1e-37),
        decades(least64, 1e-300)
    );
    out
}

/// `separation-floor`'s Heggie control: the radial collision passage under Heggie × RK4, Eq. 22, at f64, with `TINY`
/// ten times the passage's least pre-floor `dt/dτ` (`"above"`) or the reference's (`"reference"`): its markdown and
/// whether the floor acted on any step.
pub fn floor_control_heggie(
    name: &str,
    description: &str,
    tiny: &str,
) -> Result<(String, bool), String> {
    let passage = radial();
    let base = hg_floor_run::<f64>(&passage, DEFAULT_TIME, None);
    let tiny = match tiny {
        "above" => base
            .least
            .dt_dtau
            .map(|l| 10.0 * l)
            .ok_or(format!("{name}: the passage took no step"))?,
        "reference" => 1e-300,
        other => return Err(format!("{name}: no TINY `{other}` (above or reference)")),
    };
    let run = hg_floor_run::<f64>(&passage, DEFAULT_TIME, Some(tiny));
    let markdown = format!(
        "# separation-floor: {name}\n\n{description}\n\n- Heggie × RK4 (Eq. 22) on the passage at the reference's TINY: \
         least pre-floor dt/dτ {}, S {}, Rᵢ {}\n- TINY = {tiny:.3e}: {} floored step(s) of {}, the march {:?} at \
         t = {:.4}\n",
        show(base.least.dt_dtau),
        show(base.least.s),
        show(base.least.r),
        run.floored,
        run.steps,
        run.end,
        run.t
    );
    Ok((markdown, run.floored > 0))
}

// ── radial-collision ───────────────────────────────────────────────────────────────────────────────────────────

/// The `radial-collision` gate's name.
pub const RADIAL_NAME: &str = "radial-collision";

/// The closest approach the passage must go below (dd_validation_orbits § 2, R-257; REQ-VAL-045).
pub const D_MIN_BELOW: f64 = 1e-10;

/// The occupants the gate runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Occupant {
    /// Heggie × RK4 under Eq. 22, the default occupant (R-560, R-161), registered once at `t = 0`.
    Heggie,
    /// Aarseth–Zare + RK4.
    AarsethZare,
}

/// One passage: the closest approach, the occupant's regularised drift, the Cartesian read-out after the passage, the
/// steps and how the march ended.
#[derive(Clone, Copy, Debug)]
pub struct Passage {
    /// The least pair separation over the march: the least of the occupant's own regularised separations before each
    /// step and of each kept step's Cartesian image.
    pub d_min: f64,
    /// The occupant's regularised-energy drift, `|E_reg − E₀|/|E₀|`, `E₀` its registered energy at `t = 0` (R-566).
    pub regularised: f64,
    /// The Cartesian read-out after the passage, `|E(T) − E(0)|/|E(0)|` on the Cartesian states.
    pub cartesian: f64,
    pub steps: u32,
    pub end: End,
    pub t: f64,
}

/// A watch that runs the finiteness test and keeps the least of the occupant's own pair separations before each step:
/// Heggie's `Rᵢ`, Aarseth–Zare's `A` and `B`.
#[derive(Clone, Copy, Debug)]
pub struct Separations {
    pub least: f64,
}

impl Default for Separations {
    fn default() -> Self {
        Separations {
            least: f64::INFINITY,
        }
    }
}

impl HgWatch<f64> for Separations {
    fn finite(&mut self, q: &HgQ<f64>, p: &HgP<f64>) -> bool {
        HgWatch::finite(&mut ExplicitTest, q, p)
    }

    fn sized(&mut self, _dt_dtau: f64, _s: f64, r_least: f64) {
        self.least = self.least.min(r_least);
    }
}

impl kaz::Watch<f64> for Separations {
    fn finite(&mut self, q: &AzQ<f64>, p: &AzP<f64>) -> bool {
        kaz::Watch::finite(&mut ExplicitTest, q, p)
    }

    fn sized(&mut self, a: f64, b: f64) {
        self.least = self.least.min(a).min(b);
    }
}

/// `|x − x₀|/|x₀|`.
pub fn relative(x: f64, x0: f64) -> f64 {
    ((x - x0) / x0).abs()
}

/// The radial collision passage ([`radial`]) under `occupant` at step factor `eta`, at f64.
pub fn passage(occupant: Occupant, eta: f64) -> Passage {
    let f = radial();
    let s0 = f.state;
    let mut cadence = Closest::default();
    let mut watch = Separations::default();
    let (state, regularised, steps, end, t) = match occupant {
        Occupant::Heggie => {
            let params = hg_conditions::<f64>(eta, f.delta_sync, DEFAULT_TIME);
            let out = hg::march::<Rk4, f64, Closest, Separations>(
                &s0,
                f.boundaries,
                &params,
                &mut cadence,
                &mut watch,
            );
            let drift = relative(out.regularised(), out.system.h);
            (out.state, drift, out.count.steps, out.count.end, out.t)
        }
        Occupant::AarsethZare => {
            let params = conditions::<f64>(eta, f.delta_sync, true);
            let out = kaz::march::<Rk4, f64, Closest, Separations>(
                &s0,
                0,
                f.boundaries,
                kaz::AzCount::new(),
                &params,
                &mut cadence,
                &mut watch,
            );
            let drift = relative(out.regularised, out.registered);
            (out.state, drift, out.count.steps, out.count.end, out.t)
        }
    };
    Passage {
        d_min: watch.least.min(cadence.d_min),
        regularised,
        cartesian: relative(energy(&state), energy(&s0)),
        steps,
        end,
        t,
    }
}

/// What the gate reads from a passage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reader {
    /// The occupant's regularised-energy drift (R-566): the gate's reader.
    Regularised,
    /// The Cartesian read-out: the control's, which the reader check must refuse.
    Cartesian,
}

impl Reader {
    /// The value this reader takes from `p`.
    pub fn read(self, p: &Passage) -> f64 {
        match self {
            Reader::Regularised => p.regularised,
            Reader::Cartesian => p.cartesian,
        }
    }
}

/// Whether the read-outs `x`, over a ladder of `η` halving rung by rung, fall with `η`: the finest reads at least
/// `2^(n − 1)` times below the coarsest, `n` rungs (the module's note).
pub fn falls(x: &[f64]) -> bool {
    match (x.first(), x.last()) {
        (Some(&first), Some(&last)) if x.len() > 1 => first >= last * 2f64.powi(x.len() as i32 - 1),
        _ => false,
    }
}

/// Whether the read-outs' arm holds: the Cartesian read-out differs from the regularised drift, or the occupant is
/// Aarseth–Zare, which re-registers at every sync boundary with `E` the Cartesian energy there, so that its regularised
/// drift is not a read-out distinct from the Cartesian one and the arm is Heggie's alone (the module's note).
pub fn distinct(differs: bool, occupant: Occupant) -> bool {
    differs || occupant == Occupant::AarsethZare
}

/// A `radial-collision` input.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialInput {
    pub description: String,
    /// `"heggie"` or `"aarseth-zare"`.
    pub occupant: String,
    /// `"regularised"` or `"cartesian"`.
    pub reader: String,
    /// The step factor of the passage.
    pub eta: f64,
    /// The halving ladder of `η` the Cartesian read-out's control runs, coarsest first.
    pub ladder: Vec<f64>,
}

/// The `radial-collision` gate (REQ-VAL-045).
pub struct RadialCollisionGate;

impl Gate for RadialCollisionGate {
    fn name(&self) -> &'static str {
        RADIAL_NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input =
            RadialInput::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
        let occupant = match input.occupant.as_str() {
            "heggie" => Occupant::Heggie,
            "aarseth-zare" => Occupant::AarsethZare,
            other => {
                return Err(format!(
                    "refused: {name}: no occupant `{other}` (heggie or aarseth-zare)"
                ))
            }
        };
        let reader = match input.reader.as_str() {
            "regularised" => Reader::Regularised,
            "cartesian" => Reader::Cartesian,
            other => {
                return Err(format!(
                    "refused: {name}: no reader `{other}` (regularised or cartesian)"
                ))
            }
        };
        if input.ladder.len() < 2 {
            return Err(format!("refused: {name}: a ladder of two rungs or more"));
        }
        let threshold = config.threshold.value;
        let p = passage(occupant, input.eta);
        let read = reader.read(&p);
        let ladder: Vec<f64> = input
            .ladder
            .iter()
            .map(|&eta| passage(occupant, eta).cartesian)
            .collect();
        let mut scatter = vec![format!(
            "{occupant:?} + RK4 at η = {}: d_min {:.3e}, the regularised drift {:.3e}, the Cartesian read-out after the \
             passage {:.3e} ({:.1}× the regularised), {} steps, {:?} at t = {:.4}",
            input.eta,
            p.d_min,
            p.regularised,
            p.cartesian,
            p.cartesian / p.regularised,
            p.steps,
            p.end,
            p.t
        )];
        scatter.push(format!("the reader ({:?}) reads {read:.3e}", reader));
        for (eta, c) in input.ladder.iter().zip(&ladder) {
            scatter.push(format!(
                "the ladder: η = {eta}, the Cartesian read-out {c:.3e}"
            ));
        }
        scatter.push(
            "the design's measurement (integrator_contract Part 1): the Cartesian read-out 280× to 7100× worse than the \
             regularised after the passage; the AZ validation's 6.2e-15 at d_min = 1.35e-11 (Part 2a)"
                .to_owned(),
        );
        let passes = p.d_min < D_MIN_BELOW;
        let within = read < threshold;
        let bitwise = read.to_bits() == p.regularised.to_bits();
        let differs = p.cartesian != p.regularised;
        let distinct = distinct(differs, occupant);
        let falling = falls(&ladder);
        let negative = vec![
            if passes {
                format!(
                    "the passage goes below d_min {D_MIN_BELOW:e}: {:.3e}",
                    p.d_min
                )
            } else {
                format!(
                    "the passage does not go below d_min {D_MIN_BELOW:e}: {:.3e}",
                    p.d_min
                )
            },
            if within {
                format!("the read drift {read:.3e} is below the threshold {threshold:e}")
            } else {
                format!("the read drift {read:.3e} is not below the threshold {threshold:e}")
            },
            if bitwise {
                "the reader check: the value read is bitwise the occupant's regularised drift"
                    .to_owned()
            } else {
                format!(
                    "the reader check fails: the value read, {read:.3e}, is not the occupant's regularised drift, {:.3e}",
                    p.regularised
                )
            },
            match (differs, occupant) {
                (true, _) => "the Cartesian read-out differs from the regularised drift".to_owned(),
                (false, Occupant::Heggie) => {
                    "the Cartesian read-out equals the regularised drift".to_owned()
                }
                (false, Occupant::AarsethZare) => {
                    "the Cartesian read-out equals the regularised drift, as it may under \
                     Aarseth–Zare, which re-registers at every boundary from the Cartesian state"
                        .to_owned()
                }
            },
            if falling {
                "the Cartesian read-out falls with η over the ladder: an integration error, not the read-out's \
                 conditioning"
                    .to_owned()
            } else {
                "the Cartesian read-out does not fall with η over the ladder".to_owned()
            },
        ];
        Ok(GateReport {
            gate: RADIAL_NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if passes && within && bitwise && distinct && !falling {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(u32::try_from(ladder.len()).unwrap_or(u32::MAX)),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}

// ── hamiltonian-fd ─────────────────────────────────────────────────────────────────────────────────────────────

/// The `hamiltonian-fd` gate's name.
pub const FD_NAME: &str = "hamiltonian-fd";

/// The occupants the finite-difference check runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FdOccupant {
    /// Heggie under Eq. 20: `Γ*`.
    HeggieEq20,
    /// Heggie under Eq. 22: `Γ** = Γ*·S^{−3/2}`.
    HeggieEq22,
    /// Aarseth–Zare: `Γ`.
    AarsethZare,
}

impl FdOccupant {
    /// Every occupant, in the order the proposal prints them.
    pub const ALL: [FdOccupant; 3] = [
        FdOccupant::HeggieEq20,
        FdOccupant::HeggieEq22,
        FdOccupant::AarsethZare,
    ];

    /// The input's name for it.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "heggie-eq20" => Some(FdOccupant::HeggieEq20),
            "heggie-eq22" => Some(FdOccupant::HeggieEq22),
            "aarseth-zare" => Some(FdOccupant::AarsethZare),
            _ => None,
        }
    }
}

/// A value on `[lo, hi]` from `rng`, at f64.
pub fn uniform(rng: &mut SplitMix, lo: f64, hi: f64) -> f64 {
    let t = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    lo + (hi - lo) * t
}

/// A regularised state as a flat vector, its positions first, then its momenta, then the frozen energy: Heggie's
/// `(Q₀, Q₁, Q₂, P₀, P₁, P₂, h)`, Aarseth–Zare's `(u₁, u₂, p₁, p₂, E)`.
pub type Flat = Vec<f64>;

/// The Hamiltonian at a flat state.
pub type Hamiltonian = Box<dyn Fn(&[f64]) -> f64>;

/// The analytic rates at a flat state, as `(∂Γ/∂p, −∂Γ/∂q, −∂Γ/∂h)`: the position rates, the momentum rates and `dt/dτ`.
pub type Rates = Box<dyn Fn(&[f64]) -> (Vec<f64>, Vec<f64>, f64)>;

/// Heggie's `(Q, P)` and system at a flat state with `masses` and `time`.
pub fn heggie_at(
    x: &[f64],
    masses: [f64; 3],
    time: HeggieTime,
) -> (HgSystem<f64>, HgQ<f64>, HgP<f64>) {
    let v = |i: usize| [x[i], x[i + 1]];
    (
        HgSystem::new(masses, x[12], reference_tiny(), time),
        HgQ {
            q: [v(0), v(2), v(4)],
            t: 0.0,
        },
        HgP {
            p: [v(6), v(8), v(10)],
        },
    )
}

/// Heggie's Hamiltonian in fictitious time: `Γ*` under Eq. 20, `Γ*·S^{−3/2}` under Eq. 22, `S` floored at `TINY`.
pub fn heggie_hamiltonian(system: &HgSystem<f64>, q: &HgQ<f64>, p: &HgP<f64>) -> f64 {
    let g = system.gamma(q, p);
    match system.time {
        HeggieTime::Eq20 => g,
        HeggieTime::Eq22 => {
            let s =
                q.q.iter()
                    .map(|v| v[0] * v[0] + v[1] * v[1])
                    .sum::<f64>()
                    .max(system.tiny);
            g / (s * s.sqrt())
        }
    }
}

/// Aarseth–Zare's system and `(u, p)` at a flat state with masses `(m_a, m_b, m_c)`.
pub fn az_at(x: &[f64], mabc: [f64; 3]) -> (AzSystem<f64>, AzQ<f64>, AzP<f64>) {
    let [ma, mb, mc] = mabc;
    let total = ma + mb + mc;
    (
        AzSystem {
            a: 0,
            m: mabc,
            mabc,
            mu: [ma * mb / (ma + mb), ma * mc / (ma + mc)],
            k: [
                mb * (ma + mc) / total,
                mc * (ma + mb) / total,
                -(mb * mc) / total,
            ],
            e: x[8],
            tiny: reference_tiny(),
            r3_floor: 1e-300,
        },
        AzQ {
            u1: [x[0], x[1]],
            u2: [x[2], x[3]],
            t: 0.0,
        },
        AzP {
            p1: [x[4], x[5]],
            p2: [x[6], x[7]],
        },
    )
}

/// Aarseth–Zare's `Γ = A·B·(H − E)` as integrator_contract Part 2b writes it, independently of the kernel:
/// `B|p₁|²/(8μ₁) + A|p₂|²/(8μ₂) + (L₁p₁)·(L₂p₂)/(4m_a) − m_a m_b B − m_a m_c A − A·B·m_b m_c/|R₃| − E·A·B`, `|R₃|`
/// floored at the system's floor.
pub fn az_gamma(system: &AzSystem<f64>, q: &AzQ<f64>, p: &AzP<f64>) -> f64 {
    let l = |u: [f64; 2], w: [f64; 2]| [u[0] * w[0] - u[1] * w[1], u[1] * w[0] + u[0] * w[1]];
    let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
    let (a, b) = (dot(q.u1, q.u1), dot(q.u2, q.u2));
    let (r1, r2) = (l(q.u1, q.u1), l(q.u2, q.u2));
    let r3 = ((r2[0] - r1[0]).hypot(r2[1] - r1[1])).max(system.r3_floor);
    let [ma, mb, mc] = system.mabc;
    let [mu1, mu2] = system.mu;
    b * dot(p.p1, p.p1) / (8.0 * mu1)
        + a * dot(p.p2, p.p2) / (8.0 * mu2)
        + dot(l(q.u1, p.p1), l(q.u2, p.p2)) / (4.0 * ma)
        - ma * mb * b
        - ma * mc * a
        - a * b * mb * mc / r3
        - system.e * a * b
}

/// A draw: the occupant's masses, its flat state, its Hamiltonian and its analytic rates.
pub struct Draw {
    pub x: Flat,
    pub hamiltonian: Hamiltonian,
    pub rates: Rates,
}

/// One random state of `occupant` from `rng`: masses on `[0.3, 3]`, regularised positions on `[−1.2, 1.2]`, momenta on
/// `[−1, 1]` and the frozen energy on `[−3, 1]`; for Aarseth–Zare, `|R₃|` at least `0.1`, redrawn otherwise.
pub fn draw(occupant: FdOccupant, rng: &mut SplitMix) -> Draw {
    let masses = [
        uniform(rng, 0.3, 3.0),
        uniform(rng, 0.3, 3.0),
        uniform(rng, 0.3, 3.0),
    ];
    match occupant {
        FdOccupant::HeggieEq20 | FdOccupant::HeggieEq22 => {
            let time = if occupant == FdOccupant::HeggieEq20 {
                HeggieTime::Eq20
            } else {
                HeggieTime::Eq22
            };
            let mut x: Flat = (0..6).map(|_| uniform(rng, -1.2, 1.2)).collect();
            x.extend((0..6).map(|_| uniform(rng, -1.0, 1.0)));
            x.push(uniform(rng, -3.0, 1.0));
            Draw {
                x,
                hamiltonian: Box::new(move |y| {
                    let (s, q, p) = heggie_at(y, masses, time);
                    heggie_hamiltonian(&s, &q, &p)
                }),
                rates: Box::new(move |y| {
                    let (s, q, p) = heggie_at(y, masses, time);
                    let (dq, dp) = s.rates(&q, &p);
                    (
                        dq.q.iter().flatten().copied().collect(),
                        dp.p.iter().flatten().copied().collect(),
                        dq.t,
                    )
                }),
            }
        }
        FdOccupant::AarsethZare => {
            let mut x: Flat;
            loop {
                x = (0..4).map(|_| uniform(rng, -1.2, 1.2)).collect();
                let (u1, u2) = ([x[0], x[1]], [x[2], x[3]]);
                let r = |u: [f64; 2]| [u[0] * u[0] - u[1] * u[1], 2.0 * u[0] * u[1]];
                let (r1, r2) = (r(u1), r(u2));
                if (r2[0] - r1[0]).hypot(r2[1] - r1[1]) >= 0.1 {
                    break;
                }
            }
            x.extend((0..4).map(|_| uniform(rng, -1.0, 1.0)));
            x.push(uniform(rng, -3.0, 1.0));
            Draw {
                x,
                hamiltonian: Box::new(move |y| {
                    let (s, q, p) = az_at(y, masses);
                    az_gamma(&s, &q, &p)
                }),
                rates: Box::new(move |y| {
                    let (s, q, p) = az_at(y, masses);
                    let (dq, dp) = s.rates(&q, &p);
                    (
                        vec![dq.u1[0], dq.u1[1], dq.u2[0], dq.u2[1]],
                        vec![dp.p1[0], dp.p1[1], dp.p2[0], dp.p2[1]],
                        dq.t,
                    )
                }),
            }
        }
    }
}

/// The central difference of `h` in component `c` of `x` at relative step `step`: `δ = step·max(|x_c|, 1)`.
pub fn central(h: &dyn Fn(&[f64]) -> f64, x: &[f64], c: usize, step: f64) -> f64 {
    let d = step * x[c].abs().max(1.0);
    let mut up = x.to_vec();
    let mut down = x.to_vec();
    up[c] += d;
    down[c] -= d;
    (h(&up) - h(&down)) / (up[c] - down[c])
}

/// The discrepancy at one draw (the module's note): the largest `|analytic − finite difference|` over the components,
/// divided by the largest analytic rate; with `flip`, the first momentum rate's sign flipped first, the control's.
pub fn discrepancy_at(d: &Draw, step: f64, flip: bool) -> f64 {
    let (dq, mut dp, dt) = (d.rates)(&d.x);
    if flip {
        dp[0] = -dp[0];
    }
    let n = dq.len();
    let mut analytic = dq.clone();
    analytic.extend(dp.iter().copied());
    analytic.push(dt);
    let mut fd: Vec<f64> = (0..n)
        .map(|c| central(&*d.hamiltonian, &d.x, n + c, step))
        .collect();
    fd.extend((0..n).map(|c| -central(&*d.hamiltonian, &d.x, c, step)));
    fd.push(-central(&*d.hamiltonian, &d.x, 2 * n, step));
    let scale = analytic.iter().fold(0.0f64, |a, x| a.max(x.abs()));
    let worst = analytic
        .iter()
        .zip(&fd)
        .fold(0.0f64, |a, (x, y)| a.max((x - y).abs()));
    worst / scale
}

/// The largest discrepancy over `samples` draws of `occupant` from `seed`, at relative step `step`.
pub fn discrepancy(occupant: FdOccupant, step: f64, samples: u32, seed: u64, flip: bool) -> f64 {
    let mut rng = SplitMix::new(seed);
    (0..samples)
        .map(|_| discrepancy_at(&draw(occupant, &mut rng), step, flip))
        .fold(0.0f64, f64::max)
}

/// A `hamiltonian-fd` input.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FdInput {
    pub description: String,
    /// `"heggie-eq20"`, `"heggie-eq22"` or `"aarseth-zare"`.
    pub occupant: String,
    /// The relative finite-difference step (REQ-VAL-171, proposed).
    pub step: f64,
    /// The random states drawn.
    pub samples: u32,
    /// The draws' seed.
    pub seed: u64,
    /// Whether the first momentum rate's sign is flipped: the control.
    pub flip: bool,
}

/// The `hamiltonian-fd` gate (REQ-VAL-170; its step and tolerance REQ-VAL-171's).
pub struct HamiltonianFdGate;

impl Gate for HamiltonianFdGate {
    fn name(&self) -> &'static str {
        FD_NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = FdInput::deserialize(fixture).map_err(|e| format!("refused: {name}: {e}"))?;
        let occupant = FdOccupant::parse(&input.occupant).ok_or(format!(
            "refused: {name}: no occupant `{}` (heggie-eq20, heggie-eq22 or aarseth-zare)",
            input.occupant
        ))?;
        if input.samples == 0 {
            return Err(format!("refused: {name}: no samples"));
        }
        let threshold = config.threshold.value;
        let worst = discrepancy(occupant, input.step, input.samples, input.seed, input.flip);
        let pass = worst <= threshold;
        Ok(GateReport {
            gate: FD_NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter: vec![format!(
                "{occupant:?}{}: the largest discrepancy over {} states at step {:e}: {worst:.3e}",
                if input.flip {
                    ", one derivative's sign flipped"
                } else {
                    ""
                },
                input.samples,
                input.step
            )],
            regions: Some(Regions {
                count: RegionCount::Recorded(input.samples),
                minimum: config.min_regions.clone(),
            }),
            negative_results: vec![if pass {
                format!("the discrepancy {worst:.3e} is within {threshold:e}")
            } else {
                format!("the discrepancy {worst:.3e} exceeds {threshold:e}")
            }],
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => {
                print!("{}", propose_fd(&FD_LADDER, FD_SAMPLES, FD_SEED));
                Ok(())
            }
            _ => Err(format!(
                "gate {FD_NAME}: unrecognised flags `{}`; usage: --propose",
                args.join(" ")
            )),
        }
    }
}

/// The relative steps the proposal measures.
pub const FD_LADDER: [f64; 7] = [1e-2, 1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8];

/// The states the proposal draws per occupant and step.
pub const FD_SAMPLES: u32 = 64;

/// The proposal's seed.
pub const FD_SEED: u64 = 0x4865_6767_6965;

/// The tolerance's margin over the largest discrepancy at the proposed step.
pub const FD_MARGIN: f64 = 10.0;

/// `x` rounded up to one significant digit.
pub fn round_up_one(x: f64) -> f64 {
    let scale = 10f64.powf(x.log10().floor());
    (x / scale).ceil() * scale
}

/// The proposal's decision on `table`, rows `(step, the largest discrepancy over the occupants)`, under the criterion
/// fixed before the run: the step whose largest discrepancy is least (the first such), the tolerance [`FD_MARGIN`]
/// times that discrepancy rounded up to one significant digit.
pub fn decide_fd(table: &[(f64, f64)]) -> (f64, f64) {
    let mut best = table[0];
    for &row in &table[1..] {
        if row.1 < best.1 {
            best = row;
        }
    }
    (best.0, round_up_one(FD_MARGIN * best.1))
}

/// REQ-VAL-171's proposal for Heggie and Aarseth–Zare: the largest discrepancy per occupant over `ladder`, `samples`
/// states each from `seed`, the decision ([`decide_fd`]), and the sign-flipped control at the proposed step.
pub fn propose_fd(ladder: &[f64], samples: u32, seed: u64) -> String {
    let mut out = String::new();
    say!(
        out,
        "# REQ-VAL-171: the Hamiltonian check's step and tolerance, proposed (hamiltonian-fd --propose)\n"
    );
    say!(out,
        "Heggie (Eq. 20, Eq. 22) and Aarseth–Zare (TASK-M3-08; logH is TASK-M3-75's): {samples} random states each, the \
         largest |analytic − central difference| over the rates divided by the largest analytic rate, at each relative \
         step δ·max(|x|, 1).\n"
    );
    say!(
        out,
        "| step | Heggie Eq. 20 | Heggie Eq. 22 | Aarseth–Zare | largest |"
    );
    say!(out, "|---|---|---|---|---|");
    let mut table = Vec::new();
    for &step in ladder {
        let d: Vec<f64> = FdOccupant::ALL
            .iter()
            .map(|&o| discrepancy(o, step, samples, seed, false))
            .collect();
        let largest = d.iter().copied().fold(0.0f64, f64::max);
        say!(
            out,
            "| {step:e} | {:.3e} | {:.3e} | {:.3e} | {largest:.3e} |",
            d[0],
            d[1],
            d[2]
        );
        table.push((step, largest));
    }
    let (step, tolerance) = decide_fd(&table);
    say!(out,
        "\nThe criterion, fixed before the run: the step whose largest discrepancy over the occupants is least; the \
         tolerance {FD_MARGIN}× that discrepancy, rounded up to one significant digit."
    );
    let flipped: Vec<String> = FdOccupant::ALL
        .iter()
        .map(|&o| format!("{o:?} {:.3e}", discrepancy(o, step, samples, seed, true)))
        .collect();
    say!(
        out,
        "The control, one momentum rate's sign flipped, at that step: {}.",
        flipped.join(", ")
    );
    say!(
        out,
        "\n**Proposed:** step {step:e}, tolerance {tolerance:e}."
    );
    out
}
