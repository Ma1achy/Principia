//! logH's arms of the gates and measurements Heggie's and Aarseth–Zare's already have ([`crate::heggie`],
//! [`crate::az`]; integrator_contract Part 2b § "logH and TTL: the equations, the time transformations, the map and
//! the step control (transcribed)", Part 6):
//!
//! - **`hamiltonian-fd`**'s logH arms ([`draw_logh`]): the kernel's analytic rates against a central finite
//!   difference over random states. In the logH mode the Hamiltonian is `Λ = ln(K − E) − ln U`, written here from the
//!   contract with `E = −B` the frozen energy, drawn independently of the state so that the two denominators differ:
//!   the position rates against `∂Λ/∂p`, the momentum rates against `−∂Λ/∂r` and `dt/ds` against `−∂Λ/∂E`, the form
//!   the gate reads Heggie's and Aarseth–Zare's in. The TTL mode has no Hamiltonian of `(r, p)` alone, so its rates are
//!   read against the physical `K` and `U` differenced under its two denominators, `dW/ds` against `Ω` differenced
//!   along the velocities, and `dt/ds` against `1/W`, with `W` drawn independently of the state.
//! - **`separation-floor`**'s logH arm ([`propose_floor_logh`], [`floor_control_logh`]): the drift denominator before
//!   the floor, `K + B` in the logH mode and the carried `W` in the TTL mode, over the floor's fixtures, with the
//!   floored steps.
//!
//! **The conditions every floor run states** are the other arms' ([`crate::az::conditions`]): `η = 0.01`,
//! `Δ_sync = 1/8`, the reference's `TINY`, `f = 0.02` and the landing with no tolerance. Two things differ, and each
//! row says so. The step budget is [`FLOOR_BUDGET`] steps, not unbounded: the contract records that logH's predictive
//! limit can spend any budget at an exact collision, and one of the fixtures is one. And the radial collision passage
//! is also run with the predictive limit off, because with it on no logH march passes the collision (measured: each
//! ends not finite at the collision time).

use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::regularisation::az::{End, ExplicitTest};
use kernel::regularisation::logh::{self as lh, LhP, LhParams, LhQ, LhStepper, LhSystem, LhWatch};
use kernel::regularisation::pairs::LogHMode;

use crate::az::{
    at, burrau, conditions, hierarchy, radial, reference_tiny, show, FloorFixture, Landing, Quiet,
    Width, FLOOR_ETA,
};
use crate::decode_f32_eps::SplitMix;
use crate::heggie::{central, decades, state_at, tiny_above, uniform, Draw, Flat};

/// Appends one formatted line to a text.
macro_rules! say {
    ($out:expr, $($arg:tt)*) => {{
        $out.push_str(&format!($($arg)*));
        $out.push('\n');
    }};
}

/// A time mode's name.
pub fn mode_name(mode: LogHMode) -> &'static str {
    match mode {
        LogHMode::LogH => "logH",
        LogHMode::Ttl => "TTL",
    }
}

// ── hamiltonian-fd: logH's arms ────────────────────────────────────────────────────────────────────────────────

/// The three pair separations of the flat state `x = (r₀, r₁, r₂, …)`, pairs `(0, 1)`, `(0, 2)`, `(1, 2)`.
pub fn separations(x: &[f64]) -> [f64; 3] {
    let d = |i: usize, j: usize| (x[2 * i] - x[2 * j]).hypot(x[2 * i + 1] - x[2 * j + 1]);
    [d(0, 1), d(0, 2), d(1, 2)]
}

/// `K = Σ |pᵢ|²/(2mᵢ)` of the flat state `(r₀, r₁, r₂, p₀, p₁, p₂, …)`, written here from the contract, not read from
/// the kernel.
pub fn kinetic(x: &[f64], m: [f64; 3]) -> f64 {
    (0..3)
        .map(|i| (x[6 + 2 * i].powi(2) + x[7 + 2 * i].powi(2)) / (2.0 * m[i]))
        .sum()
}

/// `U = Σ mᵢmⱼ/rᵢⱼ` of the flat state.
pub fn potential(x: &[f64], m: [f64; 3]) -> f64 {
    let d = separations(x);
    m[0] * m[1] / d[0] + m[0] * m[2] / d[1] + m[1] * m[2] / d[2]
}

/// `Ω = m̄²·Σ 1/rᵢⱼ` of the flat state, `m̄` the mean mass.
pub fn omega(x: &[f64], m: [f64; 3]) -> f64 {
    let mean = (m[0] + m[1] + m[2]) / 3.0;
    separations(x).iter().map(|d| mean * mean / d).sum()
}

/// logH's Hamiltonian in fictitious time at the flat state `(r, p, E)`: `Λ = ln(K − E) − ln U`, `E = −B` the frozen
/// energy, as integrator_contract Part 2b writes it (`Λ = ln(K + B) − ln U`).
pub fn lambda(x: &[f64], m: [f64; 3]) -> f64 {
    (kinetic(x, m) - x[12]).ln() - potential(x, m).ln()
}

/// The kernel's system and state at the flat state `x` with masses `m` in `mode`: `x[12]` is the frozen energy
/// `E = −B` in the logH mode and the carried `W` in the TTL mode, whichever the mode's drift divides by. The other is
/// set to 1; the mode does not read it.
pub fn logh_at(x: &[f64], m: [f64; 3], mode: LogHMode) -> (LhSystem<f64>, LhQ<f64>, LhP<f64>) {
    let pair = |c: usize| [x[c], x[c + 1]];
    let (b, w) = match mode {
        LogHMode::LogH => (-x[12], 1.0),
        LogHMode::Ttl => (1.0, x[12]),
    };
    (
        LhSystem::new(m, b, reference_tiny(), mode),
        LhQ {
            r: [pair(0), pair(2), pair(4)],
            t: 0.0,
        },
        LhP {
            p: [pair(6), pair(8), pair(10)],
            w,
        },
    )
}

/// The least pair separation a draw keeps: closer draws are drawn again, so that the central difference's truncation,
/// which grows as the inverse square of the separation, stays below the gate's tolerance at its step.
pub const FD_SEPARATION: f64 = 0.3;

/// One random state of logH in `mode` with `masses` from `rng`, off the solution: positions on `[−1.2, 1.2]`, every
/// pair at least [`FD_SEPARATION`] apart (drawn again otherwise), momenta on `[−1, 1]`, and, drawn independently of
/// the state, the frozen energy `E = −B` on `[−3, −0.5]` in the logH mode or `W` on `[0.5, 3]` in the TTL mode.
///
/// The logH mode's draw is a Hamiltonian's, read by the gate as Heggie's is. The TTL mode's carries its own
/// finite-difference vector ([`Draw::differenced`]): its momentum rates are the six of `p` and then `dW/ds`.
pub fn draw_logh(mode: LogHMode, masses: [f64; 3], rng: &mut SplitMix) -> Draw {
    let mut x: Flat = (0..6).map(|_| uniform(rng, -1.2, 1.2)).collect();
    while separations(&x).iter().any(|&s| s < FD_SEPARATION) {
        x = (0..6).map(|_| uniform(rng, -1.2, 1.2)).collect();
    }
    x.extend((0..6).map(|_| uniform(rng, -1.0, 1.0)));
    x.push(match mode {
        LogHMode::LogH => uniform(rng, -3.0, -0.5),
        LogHMode::Ttl => uniform(rng, 0.5, 3.0),
    });
    let rates = move |y: &[f64]| {
        let (s, q, p) = logh_at(y, masses, mode);
        let (dq, dp) = s.rates_of(&q, &p);
        let mut momentum: Vec<f64> = dp.p.iter().flatten().copied().collect();
        if mode == LogHMode::Ttl {
            momentum.push(dp.w);
        }
        (dq.r.iter().flatten().copied().collect(), momentum, dq.t)
    };
    match mode {
        LogHMode::LogH => Draw {
            x,
            hamiltonian: Box::new(move |y| lambda(y, masses)),
            rates: Box::new(rates),
            differenced: None,
        },
        LogHMode::Ttl => Draw {
            x,
            hamiltonian: Box::new(move |y| kinetic(y, masses) - potential(y, masses)),
            rates: Box::new(rates),
            differenced: Some(Box::new(move |y, step| {
                // dr/ds = (∂K/∂p)/W, dp/ds = (∂U/∂r)/Ω, dW/ds = (Σ ∂Ω/∂r·v)/Ω with v = ∂K/∂p, and dt/ds = 1/W.
                let w = y[12];
                let om = omega(y, masses);
                let k = |z: &[f64]| kinetic(z, masses);
                let u = |z: &[f64]| potential(z, masses);
                let o = |z: &[f64]| omega(z, masses);
                let v: Vec<f64> = (6..12).map(|c| central(&k, y, c, step)).collect();
                let mut fd: Vec<f64> = v.iter().map(|x| x / w).collect();
                fd.extend((0..6).map(|c| central(&u, y, c, step) / om));
                fd.push((0..6).map(|c| central(&o, y, c, step) * v[c]).sum::<f64>() / om);
                fd.push(1.0 / w);
                fd
            })),
        },
    }
}

// ── separation-floor: logH's drift denominator ─────────────────────────────────────────────────────────────────

/// The step budget of logH's floor runs, at rate 0: more than four times the most any run here takes with the
/// predictive limit on (44308, measured), so it binds only where a march stalls. A run's guard, not a calibration.
pub const FLOOR_BUDGET: u32 = 200_000;

/// logH's step-control values at `R` for the floor's runs (the module's note): the other arms' conditions at step
/// factor `eta` and sync quantum `delta_sync`, in `mode`, the predictive limit's factor `f` (the others' `0.02` where
/// `None`) and a budget of [`FLOOR_BUDGET`] steps.
pub fn lh_conditions<R: Width>(
    eta: f64,
    delta_sync: f64,
    mode: LogHMode,
    f: Option<f64>,
) -> LhParams<R> {
    let c = conditions::<R>(eta, delta_sync, Landing::Fix);
    LhParams {
        eta: c.eta,
        delta_sync: c.delta_sync,
        tiny: c.tiny,
        predictive_f: f.map_or(c.predictive_f, at),
        step_budget_rate: c.step_budget_rate,
        n_min: FLOOR_BUDGET,
        clamp: c.clamp,
        secant_tries: c.secant_tries,
        bisect_tries: c.bisect_tries,
        mode,
    }
}

/// What an [`LhRecorder`] saw: the least drift and kick denominators before the floor over a march's steps, and the
/// first step's drift denominator, the registration's, at f64.
#[derive(Clone, Copy, Debug, Default)]
pub struct LhLeast {
    pub drift: Option<f64>,
    pub kick: Option<f64>,
    pub first_drift: Option<f64>,
}

/// `min(slot, x)`, seeded from the first.
fn fold(slot: &mut Option<f64>, x: f64) {
    *slot = Some(slot.map_or(x, |s| s.min(x)));
}

/// A watch that runs the occupant's finiteness test and records the least pre-floor denominators ([`LhLeast`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct LhRecorder {
    pub least: LhLeast,
}

impl<R: Width> LhWatch<R> for LhRecorder {
    fn finite(&mut self, q: &LhQ<R>, p: &LhP<R>) -> bool {
        LhWatch::finite(&mut ExplicitTest, q, p)
    }

    fn sized(&mut self, drift: R, kick: R) {
        if self.least.first_drift.is_none() {
            self.least.first_drift = Some(drift.into());
        }
        fold(&mut self.least.drift, drift.into());
        fold(&mut self.least.kick, kick.into());
    }
}

/// One fixture's logH floor measurement.
#[derive(Clone, Copy, Debug)]
pub struct LhFloorRun {
    pub least: LhLeast,
    pub floored: u32,
    pub steps: u32,
    pub end: End,
    pub t: f64,
    /// `(K + B − U)/U` where the march ended: the energy's change since registration over `U`.
    pub residual: f64,
}

/// Runs `fixture` under logH × `S` at `R` in `mode`, `TINY = tiny` (the reference's where `None`), `η = 0.01`, the
/// predictive limit's factor `f` (`0.02` where `None`).
pub fn lh_floor_run<S: LhStepper, R: Width>(
    fixture: &FloorFixture,
    mode: LogHMode,
    tiny: Option<f64>,
    f: Option<f64>,
) -> LhFloorRun {
    let mut params = lh_conditions::<R>(FLOOR_ETA, fixture.delta_sync, mode, f);
    if let Some(t) = tiny {
        params.tiny = at(t);
    }
    let mut watch = LhRecorder::default();
    let out = lh::march::<S, R, Quiet, LhRecorder>(
        &state_at::<R>(&fixture.state),
        fixture.boundaries,
        &params,
        &mut Quiet,
        &mut watch,
    );
    LhFloorRun {
        least: watch.least,
        floored: out.count.floored,
        steps: out.count.steps,
        end: out.count.end,
        t: out.t.into(),
        residual: out.system.residual(&out.q, &out.p).into(),
    }
}

/// The rows of one fixture: each mode with logH's own leapfrog (× KDK) and with RK4, at f32 and f64, the predictive
/// limit's factor `f`.
fn floor_rows(
    out: &mut String,
    fixture: &FloorFixture,
    f: Option<f64>,
    least32: &mut Option<f64>,
    least64: &mut Option<f64>,
    at_start: &mut (u32, u32),
) {
    for mode in [LogHMode::LogH, LogHMode::Ttl] {
        for (stepper, run32, run64) in [
            (
                "KDK",
                lh_floor_run::<Kdk, f32>(fixture, mode, None, f),
                lh_floor_run::<Kdk, f64>(fixture, mode, None, f),
            ),
            (
                "RK4",
                lh_floor_run::<Rk4, f32>(fixture, mode, None, f),
                lh_floor_run::<Rk4, f64>(fixture, mode, None, f),
            ),
        ] {
            for (width, run, least) in
                [("f32", run32, &mut *least32), ("f64", run64, &mut *least64)]
            {
                say!(
                    out,
                    "| {} | {} | {stepper} | {} | {width} | {} | {} | {} | {} | {:?} at t = {:.4} | {:.3e} |",
                    fixture.name,
                    mode_name(mode),
                    f.map_or("0.02".to_owned(), |f| format!("{f}")),
                    show(run.least.drift),
                    show(run.least.kick),
                    run.floored,
                    run.steps,
                    run.end,
                    run.t,
                    run.residual
                );
                if let Some(x) = run.least.drift {
                    fold(least, x);
                }
                at_start.1 += 1;
                if run.least.drift == run.least.first_drift {
                    at_start.0 += 1;
                }
            }
        }
    }
}

/// logH's arm of the floor's proposal: each fixture's least drift and kick denominators before the floor under logH,
/// in each time mode, with logH's own leapfrog and with RK4, at f32 and f64, with the floored steps, against the
/// reference's `TINY`; and the radial collision passage again with the predictive limit off.
pub fn propose_floor_logh() -> String {
    let mut out = String::new();
    say!(out, "\n## logH's arm (TASK-M3-75)\n");
    say!(out,
        "Each fixture run under logH, registered once at t = 0, in its logH and TTL time modes, with logH's own leapfrog \
         (× KDK) and with RK4, at η = {FLOOR_ETA}, Δ_sync = 0.125, TINY the reference's (1e-37 at f32, 1e-300 at f64), \
         the landing with no tolerance, and a step budget of {FLOOR_BUDGET} steps so that a stalled march ends. The \
         floor acts where the drift denominator (K + B in the logH mode, the carried W in the TTL mode) is below TINY, \
         or either denominator is not positive (integrator_contract Part 6). The last column is (K + B − U)/U where \
         the march ended, the energy's change since registration over U.\n"
    );
    let header = "| fixture | mode | stepper | f | width | least drift denominator | least kick denominator | floored \
                  | steps | end | (E − E₀)/U |";
    say!(out, "{header}");
    say!(out, "|---|---|---|---|---|---|---|---|---|---|---|");
    let (mut least32, mut least64) = (None, None);
    let mut at_start = (0, 0);
    for fixture in [radial(), hierarchy(), burrau()] {
        floor_rows(
            &mut out,
            &fixture,
            None,
            &mut least32,
            &mut least64,
            &mut at_start,
        );
    }
    say!(out,
        "\nThe radial collision passage with the predictive limit off (f = 0): with it on, every march above ends not \
         finite at the collision, about t = 1.36, so the rows above stop there.\n"
    );
    say!(out, "{header}");
    say!(out, "|---|---|---|---|---|---|---|---|---|---|---|");
    floor_rows(
        &mut out,
        &radial(),
        Some(0.0),
        &mut least32,
        &mut least64,
        &mut at_start,
    );
    say!(out,
        "\nlogH's least pre-floor drift denominator over these runs sits {:.1} decades above 1e-37 at f32 and {:.1} \
         above 1e-300 at f64. In {} of the {} runs the least value is the first step's, the registration's: the drift \
         denominator grows at a close approach (K + B follows U, W follows Ω), so its floor is not a separation floor; \
         it guards a denominator that is not positive.",
        decades(least32, 1e-37),
        decades(least64, 1e-300),
        at_start.0,
        at_start.1
    );
    out
}

/// `separation-floor`'s logH control: the radial collision passage under logH's leapfrog in `mode`, at f64, with `TINY`
/// ten times the passage's least pre-floor drift denominator (`"above"`) or the reference's (`"reference"`): its
/// markdown and whether the floor acted on any step.
pub fn floor_control_logh(
    name: &str,
    description: &str,
    tiny: &str,
    mode: LogHMode,
) -> Result<(String, bool), String> {
    let passage = radial();
    let base = lh_floor_run::<Kdk, f64>(&passage, mode, None, None);
    let tiny = match tiny {
        "above" => base
            .least
            .drift
            .map(tiny_above)
            .ok_or(format!("{name}: the passage took no step"))?,
        "reference" => 1e-300,
        other => return Err(format!("{name}: no TINY `{other}` (above or reference)")),
    };
    let run = lh_floor_run::<Kdk, f64>(&passage, mode, Some(tiny), None);
    let markdown = format!(
        "# separation-floor: {name}\n\n{description}\n\n- logH × KDK ({} mode) on the passage at the reference's TINY: \
         least pre-floor drift denominator {}, kick denominator {}\n- TINY = {tiny:.3e}: {} floored step(s) of {}, the \
         march {:?} at t = {:.4}\n",
        mode_name(mode),
        show(base.least.drift),
        show(base.least.kick),
        run.floored,
        run.steps,
        run.end,
        run.t
    );
    Ok((markdown, run.floored > 0))
}
