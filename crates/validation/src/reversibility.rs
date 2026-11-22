//! The reversibility diagnostic and its gate, `reversibility-xi` (integrator contract Part 6; dd_validation_orbits §1.1,
//! §3; TASK-M3-29): integrate forward one period, reverse, integrate back the same steps, reverse again, and measure how
//! far the state lands from where it started, `ξ`. On a free-fall (brake) orbit the run is time-symmetric by
//! construction, so a time-symmetric occupant returns to its start at round-off.
//!
//! **The route.** Part 6 names two: replay the recorded macro-step and substep schedule in reverse, or bind a
//! fixed-step reversible occupant with COM projection disabled. This is the second. The occupant is a logH pair
//! ([`LhStepper`]) stepped at one fixed fictitious step `h` by its own `regularised_step`, nothing else: no step control,
//! no landing on a sync grid (whose secant and bisection read the state, so are not symmetric), and no COM projection
//! (logH's march has none, and nothing here adds one). Under TTL the physical step still adapts, `dt = h/W`, through the
//! carried `W`, which is what keeps a fixed `h` usable through close approaches and is symmetric under the reversal.
//!
//! **The guard** (Part 2, R-500). The diagnostic ([`diagnose`]) refuses an occupant whose profile has
//! `step_reversible` false ([`CapabilityProfile::reversibility_diagnostic`]); the reversibility test ([`bind`], the
//! gate) refuses one whose profile has `run_reversible` false ([`CapabilityProfile::reversibility_test`]), which leaves
//! logH's TTL time mode with its leapfrog alone. [`measure`] is the measurement without either guard, for the controls.
//!
//! **ξ** (the definition dd_validation_orbits §1.1 gives). From the start `z₀ = (rᵢ, pᵢ)`, `n` steps of `h` forward,
//! `n` being the first count whose clock reaches the period `T`; the reversal map `(r, p, t, W) → (r, −p, −t, W)`; `n`
//! steps of `h`; the reversal map again, giving `z₂`. Then
//!
//! ```text
//! ξ_r = √( Σ mᵢ‖r₂ᵢ − r₀ᵢ‖² / I₀ ),   I₀ = Σ mᵢ‖r₀ᵢ − r_com‖²
//! ξ_p = √( Σ ‖p₂ᵢ − p₀ᵢ‖²/mᵢ / (2U₀) ), U₀ = Σ mᵢmⱼ/rᵢⱼ at the start
//! ξ   = √( ξ_r² + ξ_p² )
//! ```
//!
//! Dimensionless and unchanged by the scale gauge (`r → αr`, `p → α^{−1/2}p`: both ratios are invariant), and the
//! momentum scale is the potential's, not the start's kinetic energy, which is zero on a brake orbit. The clock's own
//! return, `|t₂|/T`, is read beside it.
//!
//! **What a reversible occupant reads.** A fixed-step symmetric map composed with its reversal is the identity in exact
//! arithmetic, whatever `h`, so `ξ` is round-off: `2n` steps at f64's unit round-off `u = 2⁻⁵³` give at most about
//! `2n·u ≈ 9·10⁻¹³` at `n = 4096`, times what the orbit's own instability amplifies an error by over one period.
//! [`XI_MAX`], `10⁻⁹`, allows that amplification a factor of 1000. A non-reversible occupant reads its truncation error
//! instead, which falls with `h`; a state-dependent step schedule (the adaptive substepping Part 6 names) reads one
//! too, whatever the stepper. The amplification is measured and reported beside each reading ([`Reading::amplification`]).
//!
//! **Pass criteria**, written before the first run:
//! - the forward run stays finite and reaches the period within [`STEP_CAP_FACTOR`] times the nominal step count;
//! - `ξ ≤ XI_MAX`;
//! - the clock's return `|t₂|/T ≤ XI_MAX`.
//!
//! Reported, never judged: `ξ_r` and `ξ_p` apart, `W`'s return, the step count and `h`, the forward run's closure
//! against the start at the step that first reaches `T` (a comparison with the published orbit, so a measurement: the
//! free-fall closure test is a separate, external task), its energy error, the amplification `A`, and
//! `ρ = ξ/(2n·u·A)`, the reading over the round-off the orbit's measured amplification allows.
//!
//! **The first run's finding.** The criterion's assumed amplification is wrong for these orbits: `A` is measured at
//! `1.9·10⁶`, `3.1·10⁶` and `3.6·10⁷` over one period on rows 1, 3 and 4, the least-`T*` free-fall orbits being far
//! from stable, and `ξ` reads `1.9·10⁻⁸`, `8.6·10⁻¹⁰` and `7.3·10⁻⁷`: rows 1 and 4 fail `ξ ≤ XI_MAX`. Their inputs
//! are recorded as expected failures with this measurement beside them, and the criterion is not moved. That their `ξ`
//! is round-off and not a broken reversal was measured: on logH-TTL × KDK `ξ` does not fall as `h` shrinks (row 3, at
//! 1024 to 65536 nominal steps: `8.8·10⁻⁹`, `1.4·10⁻⁹`, `8.6·10⁻¹⁰`, `1.1·10⁻⁹`, `1.9·10⁻¹⁰`, `1.3·10⁻⁹`), where
//! logH-TTL × RK4's falls at `h⁵` (`1.5·10⁻²`, `1.5·10⁻⁵`, `1.2·10⁻⁸` at 1024, 4096 and 16384); and `ρ` reads at most
//! `10⁻²` on the leapfrog, against `2.7` to `1.8·10⁷` for RK4 at the gate's step. A criterion `ρ ≤ 1` would read what
//! this one means to; it is reported, not judged, because it was written after the first run.
//!
//! The plan is unclear here (no calibration requirement names the reversibility threshold); chose the ground-truth
//! tolerances' (`gate.json`), because the brake orbits are that suite's fixtures and `XI_MAX` is an f64 exact-case
//! tolerance.

use std::fmt;
use std::path::Path;

use serde::Deserialize;

use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{CapabilityProfile, State};
use kernel::physics::PairGeom;
use kernel::regularisation::logh::{
    register, reversed, LhP, LhQ, LhRegistered, LhStepper, LhSystem,
};
use kernel::regularisation::pairs::LogHMode;

use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};

/// The gate's name.
pub const NAME: &str = "reversibility-xi";

/// The brake orbits, relative to the workspace root.
pub const FIXTURES: &str = "fixtures/ground_truth/free_fall/orbits.json";

/// The most `ξ`, and the most `|t₂|/T`, a reversible occupant reads: `2n·u` at `n = 4096` with a factor of 1000 for the
/// orbit's amplification (the module's note).
pub const XI_MAX: f64 = 1e-9;

/// f64's unit round-off `u = 2⁻⁵³`.
pub const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;

/// The nominal steps per period: `h = T/(n·dt/ds₀)`, the step whose physical length at the start is `T/n`.
pub const STEPS_PER_PERIOD: u32 = 4096;

/// The forward run's most steps, as a multiple of the nominal count.
pub const STEP_CAP_FACTOR: u32 = 64;

/// logH's floor `TINY`, the comparison harness's.
pub const TINY: f64 = 1e-300;

/// The amplification's perturbation, relative to the start's hyperradius `√(I₀/M)`.
pub const PERTURBATION: f64 = 1e-8;

// ── The orbits ───────────────────────────────────────────────────────────────────────────────────────────────────

/// One brake orbit as committed: the database's digits, as strings, and its row.
#[derive(Clone, Debug, Deserialize)]
pub struct BrakeOrbit {
    /// The fixture's name.
    pub name: String,
    /// The 1-based line of `sol_80.txt`.
    pub row: u32,
    /// The third body's `x`.
    pub x: String,
    /// The third body's `y`.
    pub y: String,
    /// The period `T`.
    pub period: String,
    /// `T* = T|E|^{3/2}`.
    pub scaled_period: String,
}

#[derive(Deserialize)]
struct OrbitsFile {
    orbits: Vec<BrakeOrbit>,
}

fn number(what: &str, s: &str) -> Result<f64, String> {
    s.trim()
        .parse::<f64>()
        .map_err(|e| format!("refused: {what} `{s}`: {e}"))
}

impl BrakeOrbit {
    /// The start: unit masses at rest at `(−½, 0)`, `(½, 0)` and `(x, y)`, `G = 1` (the database's convention).
    pub fn start(&self) -> Result<State<f64>, String> {
        let x = number("x", &self.x)?;
        let y = number("y", &self.y)?;
        Ok(State {
            m: [1.0; 3],
            r: [[-0.5, 0.0], [0.5, 0.0], [x, y]],
            p: [[0.0; 2]; 3],
        })
    }

    /// The period `T`.
    pub fn period(&self) -> Result<f64, String> {
        number("period", &self.period)
    }

    /// `T*` as published.
    pub fn scaled_period(&self) -> Result<f64, String> {
        number("scaled_period", &self.scaled_period)
    }
}

/// The committed brake orbits under `root`.
pub fn orbits(root: &Path) -> Result<Vec<BrakeOrbit>, String> {
    let path = root.join(FIXTURES);
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("refused: {}: {e}", path.display()))?;
    let file: OrbitsFile =
        serde_json::from_str(&text).map_err(|e| format!("refused: {}: {e}", path.display()))?;
    Ok(file.orbits)
}

/// The committed brake orbit named `name`.
pub fn orbit(root: &Path, name: &str) -> Result<BrakeOrbit, String> {
    orbits(root)?
        .into_iter()
        .find(|o| o.name == name)
        .ok_or_else(|| format!("refused: no brake orbit `{name}` in {FIXTURES}"))
}

// ── The state's scales ───────────────────────────────────────────────────────────────────────────────────────────

/// `U = Σ mᵢmⱼ/rᵢⱼ`.
pub fn potential(s: &State<f64>) -> f64 {
    let d = |i: usize, j: usize| (s.r[i][0] - s.r[j][0]).hypot(s.r[i][1] - s.r[j][1]);
    s.m[1] * s.m[2] / d(1, 2) + s.m[2] * s.m[0] / d(2, 0) + s.m[0] * s.m[1] / d(0, 1)
}

/// `K = Σ ‖pᵢ‖²/(2mᵢ)`.
pub fn kinetic(s: &State<f64>) -> f64 {
    (0..3)
        .map(|i| (s.p[i][0] * s.p[i][0] + s.p[i][1] * s.p[i][1]) / (2.0 * s.m[i]))
        .sum()
}

/// `E = K − U`.
pub fn energy(s: &State<f64>) -> f64 {
    kinetic(s) - potential(s)
}

/// `I = Σ mᵢ‖rᵢ − r_com‖²`.
pub fn moment(s: &State<f64>) -> f64 {
    let m: f64 = s.m.iter().sum();
    let com = [0, 1].map(|k| (0..3).map(|i| s.m[i] * s.r[i][k]).sum::<f64>() / m);
    (0..3)
        .map(|i| s.m[i] * ((s.r[i][0] - com[0]).powi(2) + (s.r[i][1] - com[1]).powi(2)))
        .sum()
}

/// `(ξ_r, ξ_p)` between `a` and `b`, scaled by `reference`'s `I` and `U` (the module's note).
pub fn phase_distance(a: &State<f64>, b: &State<f64>, reference: &State<f64>) -> (f64, f64) {
    let m = reference.m;
    let dr2: f64 = (0..3)
        .map(|i| m[i] * ((a.r[i][0] - b.r[i][0]).powi(2) + (a.r[i][1] - b.r[i][1]).powi(2)))
        .sum();
    let dp2: f64 = (0..3)
        .map(|i| ((a.p[i][0] - b.p[i][0]).powi(2) + (a.p[i][1] - b.p[i][1]).powi(2)) / m[i])
        .sum();
    (
        (dr2 / moment(reference)).sqrt(),
        (dp2 / (2.0 * potential(reference))).sqrt(),
    )
}

// ── The guard ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Why an occupant is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Its `step_reversible` is false: neither of Part 6's routes may bind it.
    NotStepReversible,
    /// Its `run_reversible` is false: the reversibility test does not bind it.
    NotRunReversible,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotStepReversible => {
                "the occupant's profile has step_reversible false: the reversibility diagnostic does not bind it"
            }
            Self::NotRunReversible => {
                "the occupant's profile has run_reversible false: the reversibility test binds only logH's TTL \
                 time mode"
            }
        })
    }
}

/// The diagnostic's guard on `profile`: `step_reversible`.
pub fn guard(profile: &CapabilityProfile) -> Result<(), Refused> {
    if profile.reversibility_diagnostic() {
        Ok(())
    } else {
        Err(Refused::NotStepReversible)
    }
}

/// How the steps are sized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Schedule {
    /// One fixed fictitious step `h`: the route.
    Fixed,
    /// `h·√(κ₀/κ(r))`, `κ` the time mode's kick denominator read at each step's start: a state-dependent schedule,
    /// which a reversed run does not retrace. A control's only. Its first form, `h·κ₀/κ`, shrank the steps so far near
    /// the close approaches that the forward run never reached the period (measured on row 1: 0.77 `T` in 262144
    /// steps), so the control failed on reach, not on `ξ`; the square root reaches it on every orbit.
    Adaptive,
}

// ── The measurement ──────────────────────────────────────────────────────────────────────────────────────────────

/// One reading of the diagnostic.
#[derive(Clone, Copy, Debug)]
pub struct Reading {
    /// The steps each way, `n`.
    pub steps: u32,
    /// The fictitious step `h`.
    pub h: f64,
    /// Whether the forward run reached the period, finite, within the cap.
    pub reached: bool,
    /// The forward run's clock at its last step, over `T`.
    pub t_forward: f64,
    /// `ξ`.
    pub xi: f64,
    /// `ρ = ξ/(2n·u·A)`, reported.
    pub rho: f64,
    /// `ξ_r`.
    pub xi_r: f64,
    /// `ξ_p`.
    pub xi_p: f64,
    /// `|t₂|/T`.
    pub clock: f64,
    /// `|W₂ − W₀|/W₀`.
    pub w: f64,
    /// The forward run's `(ξ_r, ξ_p)` distance from the start at its last step.
    pub closure: (f64, f64),
    /// `|E(T) − E₀|/|E₀|` at the forward run's last step.
    pub energy: f64,
    /// The forward run's amplification of a perturbation over its `n` steps, in `ξ`'s metric.
    pub amplification: f64,
}

/// Whether every component of `(q, p)` is finite.
fn finite(q: &LhQ<f64>, p: &LhP<f64>) -> bool {
    q.r.iter().flatten().all(|x| x.is_finite())
        && p.p.iter().flatten().all(|x| x.is_finite())
        && q.t.is_finite()
        && p.w.is_finite()
}

/// `n` steps of `S` from `(q, p)` on `system`, sized by `size`.
fn steps<S: LhStepper>(
    system: &LhSystem<f64>,
    (q, p): (LhQ<f64>, LhP<f64>),
    n: u32,
    size: &dyn Fn(&LhQ<f64>) -> f64,
) -> (LhQ<f64>, LhP<f64>) {
    let (mut q, mut p) = (q, p);
    for _ in 0..n {
        (q, p) = S::regularised_step(system, &q, &p, size(&q));
    }
    (q, p)
}

/// The measurement without the guard: logH × `S` in `mode` from `start`, forward to the period `period` at
/// `steps_per_period` nominal steps, reversed, back, reversed (the module's note). The controls' only entry; the
/// diagnostic is [`diagnose`].
pub fn measure<S: LhStepper>(
    mode: LogHMode,
    start: &State<f64>,
    period: f64,
    steps_per_period: u32,
    schedule: Schedule,
) -> Reading {
    let LhRegistered {
        system,
        q: q0,
        p: p0,
    } = register(start, TINY, mode);
    let h = period / (f64::from(steps_per_period) * system.dt_ds(&p0));
    let kick0 = system.kick_raw(&PairGeom::new(&q0.r));
    let size = move |q: &LhQ<f64>| match schedule {
        Schedule::Fixed => h,
        Schedule::Adaptive => h * (kick0 / system.kick_raw(&PairGeom::new(&q.r))).sqrt(),
    };
    let cap = steps_per_period.saturating_mul(STEP_CAP_FACTOR);
    let (mut q, mut p) = (q0, p0);
    let mut n = 0u32;
    while q.t < period && n < cap && finite(&q, &p) {
        (q, p) = S::regularised_step(&system, &q, &p, size(&q));
        n += 1;
    }
    let reached = q.t >= period && finite(&q, &p);
    let (qf, pf) = (q, p);
    let (qr, pr) = reversed(&qf, &pf);
    let (q2, p2) = steps::<S>(&system, (qr, pr), n, &size);
    let (qb, pb) = reversed(&q2, &p2);
    let back = system.image(&qb, &pb);
    let (xi_r, xi_p) = phase_distance(&back, start, start);
    let at_t = system.image(&qf, &pf);
    let e0 = energy(start);

    // The amplification: the third body moved along x by `PERTURBATION` hyperradii, registered alone and stepped the
    // same `n` steps of the same schedule's `h`; the two forward runs' distance at the end over at the start.
    let scale = (moment(start) / start.m.iter().sum::<f64>()).sqrt();
    let mut moved = *start;
    moved.r[2][0] += PERTURBATION * scale;
    let LhRegistered {
        system: s1,
        q: q1,
        p: p1,
    } = register(&moved, TINY, mode);
    let (q1, p1) = steps::<S>(&s1, (q1, p1), n, &size);
    let pd = |a: &State<f64>, b: &State<f64>| {
        let (r, p) = phase_distance(a, b, start);
        r.hypot(p)
    };
    let amplification = pd(&s1.image(&q1, &p1), &at_t) / pd(&moved, start);

    Reading {
        steps: n,
        h,
        reached,
        t_forward: qf.t / period,
        xi: xi_r.hypot(xi_p),
        rho: xi_r.hypot(xi_p) / (2.0 * f64::from(n) * UNIT_ROUNDOFF * amplification),
        xi_r,
        xi_p,
        clock: qb.t.abs() / period,
        w: (pb.w - p0.w).abs() / p0.w,
        closure: phase_distance(&at_t, start, start),
        energy: ((energy(&at_t) - e0) / e0).abs(),
        amplification,
    }
}

/// The reversibility diagnostic, Part 6's route (b): logH × `S` in `mode` at one fixed step, refused unless its profile
/// has `step_reversible`.
pub fn diagnose<S: LhStepper>(
    mode: LogHMode,
    start: &State<f64>,
    period: f64,
    steps_per_period: u32,
) -> Result<Reading, Refused> {
    guard(&S::pair_profile(mode))?;
    Ok(measure::<S>(
        mode,
        start,
        period,
        steps_per_period,
        Schedule::Fixed,
    ))
}

/// The reversibility test's guard on `profile`: the diagnostic's, `step_reversible`, then `run_reversible` (logH's TTL
/// time mode with its leapfrog alone).
pub fn test_guard(profile: &CapabilityProfile) -> Result<(), Refused> {
    guard(profile)?;
    if profile.reversibility_test() {
        Ok(())
    } else {
        Err(Refused::NotRunReversible)
    }
}

/// The reversibility test's binding: logH × `S` in `mode`, refused by [`test_guard`], then the diagnostic.
pub fn bind<S: LhStepper>(
    mode: LogHMode,
    start: &State<f64>,
    period: f64,
    steps_per_period: u32,
) -> Result<Reading, Refused> {
    test_guard(&S::pair_profile(mode))?;
    diagnose::<S>(mode, start, period, steps_per_period)
}

/// Whether `x` is at most `max`: false where `x` is NaN, so a NaN reading fails.
fn within(x: f64, max: f64) -> bool {
    x <= max
}

/// The pass criteria (the module's note) on `r` against `xi_max`: the verdict, and one line per criterion that failed,
/// or one saying none did.
pub fn judge(r: &Reading, xi_max: f64) -> (Verdict, Vec<String>) {
    let mut failed = Vec::new();
    if !r.reached {
        failed.push(format!(
            "the forward run did not reach the period finite within {} steps (clock {:.6} T)",
            r.steps, r.t_forward
        ));
    }
    if !within(r.xi, xi_max) {
        failed.push(format!("ξ {:.3e} is above {xi_max:e}", r.xi));
    }
    if !within(r.clock, xi_max) {
        failed.push(format!(
            "the clock's return |t₂|/T {:.3e} is above {xi_max:e}",
            r.clock
        ));
    }
    if failed.is_empty() {
        (
            Verdict::Pass,
            vec![format!(
                "none: ξ {:.3e} and |t₂|/T {:.3e} within {xi_max:e}, the period reached in {} steps",
                r.xi, r.clock, r.steps
            )],
        )
    } else {
        (Verdict::Fail, failed)
    }
}

/// The reading as the report's scatter lines.
pub fn table(r: &Reading) -> Vec<String> {
    vec![
        format!(
            "steps each way {}, h {:.6e}, forward clock {:.9} T",
            r.steps, r.h, r.t_forward
        ),
        format!(
            "ξ {:.3e} (ξ_r {:.3e}, ξ_p {:.3e}), |t₂|/T {:.3e}, |W₂ − W₀|/W₀ {:.3e}",
            r.xi, r.xi_r, r.xi_p, r.clock, r.w
        ),
        format!(
            "reported: amplification {:.3e}, ρ = ξ/(2n·u·A) {:.3e}; closure at the last forward step ξ_r {:.3e}, \
             ξ_p {:.3e}; |ΔE/E| {:.3e}",
            r.amplification, r.rho, r.closure.0, r.closure.1, r.energy
        ),
    ]
}

// ── The gate ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// An input of the gate.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    description: String,
    /// The brake orbit's name in [`FIXTURES`].
    orbit: String,
    /// `kdk`, `yoshida4`, `yoshida6`, `rk4` or `euler`.
    stepper: String,
    /// `ttl` or `logh`.
    mode: String,
    #[serde(default = "default_steps")]
    steps_per_period: u32,
    #[serde(default = "default_schedule")]
    schedule: Schedule,
    #[serde(default)]
    control: bool,
}

fn default_steps() -> u32 {
    STEPS_PER_PERIOD
}

fn default_schedule() -> Schedule {
    Schedule::Fixed
}

/// The time mode named `mode`.
pub fn mode(mode: &str) -> Result<LogHMode, String> {
    match mode {
        "ttl" => Ok(LogHMode::Ttl),
        "logh" => Ok(LogHMode::LogH),
        other => Err(format!("refused: no time mode `{other}`; ttl or logh")),
    }
}

/// The gate's binding of logH × `S`: [`bind`] on the fixed schedule; on the adaptive one (a control), the same two
/// guards, then the measurement on that schedule.
fn run<S: LhStepper>(
    mode: LogHMode,
    start: &State<f64>,
    period: f64,
    steps_per_period: u32,
    schedule: Schedule,
) -> Result<Reading, String> {
    let refused = |r: Refused| format!("refused: {r}");
    match schedule {
        Schedule::Fixed => bind::<S>(mode, start, period, steps_per_period).map_err(refused),
        Schedule::Adaptive => {
            test_guard(&S::pair_profile(mode)).map_err(refused)?;
            Ok(measure::<S>(
                mode,
                start,
                period,
                steps_per_period,
                schedule,
            ))
        }
    }
}

/// The gate: each input one brake orbit on one logH pair.
#[derive(Debug)]
pub struct ReversibilityXiGate;

impl Gate for ReversibilityXiGate {
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
        if input.schedule == Schedule::Adaptive && !input.control {
            return Err(
                "refused: the adaptive schedule is not Part 6's route; only a control takes it"
                    .to_owned(),
            );
        }
        if input.steps_per_period == 0 {
            return Err("refused: no steps per period".to_owned());
        }
        let orbit = orbit(&crate::import::root(), &input.orbit)?;
        let start = orbit.start()?;
        let period = orbit.period()?;
        let m = mode(&input.mode)?;
        let (n, s) = (input.steps_per_period, input.schedule);
        let reading = match input.stepper.as_str() {
            "kdk" => run::<Kdk>(m, &start, period, n, s),
            "yoshida4" => run::<Yoshida4>(m, &start, period, n, s),
            "yoshida6" => run::<Yoshida6>(m, &start, period, n, s),
            "rk4" => run::<Rk4>(m, &start, period, n, s),
            "euler" => run::<Euler>(m, &start, period, n, s),
            other => Err(format!("refused: no stepper `{other}`")),
        }?;
        let (verdict, negative) = judge(&reading, config.threshold.value);
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict,
            threshold: config.threshold.clone(),
            scatter: table(&reading),
            regions: Some(Regions {
                count: RegionCount::Recorded(1),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}
