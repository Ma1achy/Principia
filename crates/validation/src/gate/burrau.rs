//! The Burrau anchor smoke tests (TASK-M3-27): continuous and blocking, for gross breakage only, never steering design.
//! `cargo xtask gate burrau-smoke` runs SP-1; `--target SP-2` and `--target PL-8a` run the other two, and `--propose`
//! after a target prints that target's calibration proposal, which asserts nothing.
//!
//! **SP-1** ([`sp1`]): the classical Burrau rest start through Path A, marched by an occupant at the benchmark's
//! `r_coll = 0` to its own horizon `t_max` (a parameter in [`SP1_T_MAX_RANGE`], [`SP1_T_MAX`] by default; the sim key's
//! default horizon is untouched), read structurally at `t_max`: binary plus escaper, the escaper the lightest body. The
//! escape label and its `t_end` window stay on the march to [`BURRAU_HORIZON`] ([`tiers::burrau_live`]).
//!
//! The plan is unclear here: it asks SP-1 at `t_max` 60–65 to assert the outcome class and a `t_end` window, but the
//! default rule (Fewbody) first labels Burrau's escape at 77.22 of S&P's units (measured), so at any horizon in 60–65
//! the label is `bounded` and `t_end` is the horizon itself, which no window can test. Chose to read SP-1's class and
//! escaper at `t_max` from the march's own end state, independently of the detector ([`tiers::outcome`]: the lightest
//! body unbound and ten pair separations out, the pair bound), and to keep the label and its window on the march to
//! 100, where they can fail; the label at `t_max` is reported. `t_max = 65` because the structural read's ten pair
//! separations depend on the binary's phase (a ratio near 12 at 63, none at 62, measured).
//!
//! **SP-2** ([`sp2`]): the same start on the convergence reference (`reference::brutus`), unperturbed and perturbed
//! (`x₃ + 10⁻¹⁰`, exact at the rung's precision), to `t = 100`; every velocity reversed there, on to `t = 200`.
//! Compared with Portegies Zwart & Boekholt 2018's Table 2 (`docs/incoming/2026-10-29_burrau_pzb2018/table2/
//! table2_reconciled.csv`) at 100 and 200 and with the start at 200. **The criteria, written before the first run to
//! 200**, in the reference's own scaled distance at its own precision against the printed digits
//! ([`brutus::scaled_to_decimal`], [`brutus::disagreement_at`]), `δ(t)` the converged ladder's last two rungs'
//! disagreement at `t`, with no tuned number:
//!
//! - `t = 100`: `‖ours − Table 2‖ ≤ δ(100)`: a converged reference agrees with the published one within its own
//!   convergence. This holds only while `δ(100)` is above the table's own error: on `--propose`'s ladder (`δ ≈ 2·10⁻¹⁶`)
//!   the two agree to 13.7 digits, the table's own return to its start at 200 being 1.9·10⁻¹³ against ours at
//!   10⁻¹⁹ (measured), so a ladder tighter than about `10⁻¹³` would fail here on the table, not on the reference.
//! - `t = 200`, reversibility: `‖ours − start‖ ≤ δ(200)`: the exact reversed flow is back at the start, and the
//!   reference is within `δ(200)` of the exact flow.
//! - `t = 200`, trajectory: `‖ours − Table 2‖ ≤ δ(200) + ‖Table 2 − start‖`: the triangle inequality through the exact
//!   return, the table's own departure from the start (its own reversal error) known before the run.
//!
//! **PL-8a** ([`pl8a`]): the nominal Burrau pixel (Path A's decode at f64) marched by the default occupant at f64 and,
//! for the control, at f32 (the same start rounded to f32, so the width is the one thing changed), sampled every half
//! unit of S&P's time to 60, against a converged reference from the same start. The **chord** is
//! [`brutus::scaled_f64`] (relative to the system's size and speed), the **parting time** the first sample whose chord
//! exceeds [`PL8A_PARTING`]. Pass: the f64 run does not part before f64's horizon, [`PL8A_F64_HORIZON`]; and the f32 run
//! parts within the window, before the f64 run (a run that does not part counts as parting at infinity).
//!
//! The regularised march's state is its last sync boundary's, `k·Δ_sync`, not `n·dt_macro`'s (measured: at
//! `n·dt_macro = 0.104` the state was still the start), so each sample is taken at a boundary, with `n` landing the
//! march mid-interval after it, and the reference is read at the boundary's time.

use std::path::Path;

use kernel::decode::Ic;
use kernel::driver::advance::Advance;
use kernel::driver::wrapper::{integrate_detecting, WrapperUniforms};
use kernel::generated::links::Defaults;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::payload::{sd_detail, sd_state, tm_t_end_step};
use kernel::regularisation::sync::{boundary_at_or_before, delta_sync};
use serde::Deserialize;

use super::{Gate, GateConfig, GateReport, RegionCount, Regions, Verdict};
use crate::ground_truth::tiers::{self, BURRAU_HORIZON, BURRAU_WINDOW};
use crate::ground_truth::{
    burrau_time_scale, march_key, path_a, DefaultOccupant, BENCHMARK_R_COLL,
};
use crate::import::unit_state;
use crate::reference::brutus::{self, Converged, Extras, Ladder, Nudge, Output};
use crate::reference::brutus_gate::{from_ic, from_published};

/// The gate's name.
pub const NAME: &str = "burrau-smoke";

// ── SP-1 ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// SP-1's horizon by default, in S&P's units (the module's note).
pub const SP1_T_MAX: f64 = 65.0;

/// The horizons SP-1 takes, in S&P's units: the target's own break-up (dd_validation_orbits §1.5).
pub const SP1_T_MAX_RANGE: [f64; 2] = [60.0, 65.0];

/// SP-1's run: the march to `t_max`, read structurally, and the label's march to [`BURRAU_HORIZON`].
#[derive(Clone, Debug)]
pub struct Sp1 {
    /// The occupant's name.
    pub occupant: &'static str,
    /// The horizon, S&P's units.
    pub t_max: f64,
    /// The march's end state at `t_max`, read structurally.
    pub outcome: tiers::Outcome,
    /// The two-body energy of the pair left behind (the two bodies other than the lightest); negative where bound.
    pub pair_energy: f64,
    /// The lightest body's two-body energy about the pair.
    pub escaper_energy: f64,
    /// The label at `t_max`, `(state, detail)` (reported).
    pub label_at_t_max: (u32, u32),
    /// The stop option's first label on the march to [`BURRAU_HORIZON`], `(state, detail, t in S&P's units)`.
    pub first_label: (u32, u32, f64),
    /// The label criterion's verdict on the march to [`BURRAU_HORIZON`] ([`tiers::burrau_live`]).
    pub label: Result<(), String>,
    /// The march to [`BURRAU_HORIZON`] under the stop option, whose first label the criterion reads.
    pub stopped: kernel::payload::SimStateBaseOf<f64>,
    /// The live march to [`BURRAU_HORIZON`], the default, whose label at the horizon must be the first one.
    pub live: kernel::payload::SimStateBaseOf<f64>,
    /// The march's macro-step.
    pub dt_macro: f64,
}

impl Sp1 {
    /// The verdict: the class and escaper at `t_max`, then the label's criterion; `Err` naming what failed.
    pub fn verdict(&self) -> Result<(), String> {
        let lightest = tiers::lightest(tiers::burrau().m);
        if self.outcome.escaper != Some(lightest) || self.pair_energy >= 0.0 {
            return Err(format!(
                "SP-1 under {}: the Burrau march at t_max = {} of S&P's units is not binary plus the lightest body \
                 escaping: escaper {:?} (the lightest is body {lightest}), escaper energy {:e}, the pair's energy {:e}, \
                 distance {:.3} against the pair's separation {:.3}",
                self.occupant,
                self.t_max,
                self.outcome.escaper,
                self.escaper_energy,
                self.pair_energy,
                self.outcome.distance,
                self.outcome.pair_separation
            ));
        }
        self.label
            .clone()
            .map_err(|e| format!("SP-1 under {}: {e}", self.occupant))
    }

    /// The report's lines.
    pub fn lines(&self) -> Vec<String> {
        vec![
            format!(
                "occupant {}, t_max {} of S&P's units, r_coll {BENCHMARK_R_COLL}",
                self.occupant, self.t_max
            ),
            format!(
                "at t_max: escaper {:?}, its distance {:.3} from the pair, the pair's separation {:.3}, its energy {:e}, \
                 the pair's {:e}",
                self.outcome.escaper,
                self.outcome.distance,
                self.outcome.pair_separation,
                self.escaper_energy,
                self.pair_energy
            ),
            format!(
                "the label at t_max (reported): state {}, detail {}",
                self.label_at_t_max.0, self.label_at_t_max.1
            ),
            format!(
                "the first label to {BURRAU_HORIZON}: state {}, detail {}, at {:.3} of S&P's units (window \
                 {BURRAU_WINDOW:?}): {}",
                self.first_label.0,
                self.first_label.1,
                self.first_label.2,
                match &self.label {
                    Ok(()) => "met".to_owned(),
                    Err(e) => e.clone(),
                }
            ),
        ]
    }
}

/// The two-body energy of bodies `i` and `j` of `x`, `½μ|Δv|² − mᵢmⱼ/r`, `G = 1`.
fn pair_energy(x: &Ic<f64>, i: usize, j: usize) -> f64 {
    let mu = x.m[i] * x.m[j] / (x.m[i] + x.m[j]);
    let dv = [0, 1].map(|c| x.p[i][c] / x.m[i] - x.p[j][c] / x.m[j]);
    let r = (x.r[i][0] - x.r[j][0]).hypot(x.r[i][1] - x.r[j][1]);
    0.5 * mu * (dv[0] * dv[0] + dv[1] * dv[1]) - x.m[i] * x.m[j] / r
}

/// SP-1 under the occupant `A`, named `occupant`, to `t_max` of S&P's units; `Err` refuses a `t_max` outside
/// [`SP1_T_MAX_RANGE`] or a start the front end will not take.
pub fn sp1<A: Advance>(occupant: &'static str, t_max: f64) -> Result<Sp1, String> {
    if !(SP1_T_MAX_RANGE[0]..=SP1_T_MAX_RANGE[1]).contains(&t_max) {
        return Err(format!(
            "SP-1 refused: t_max = {t_max} is outside {SP1_T_MAX_RANGE:?} of S&P's units, the target's own break-up"
        ));
    }
    let start = tiers::burrau();
    let x = unit_state(&start)?;
    let scale = burrau_time_scale();
    let c = path_a::check1::<Defaults>(&x);
    let forward = |t: f64, stop: bool| {
        let key = march_key(t * scale, BENCHMARK_R_COLL).map_err(|e| format!("{e:?}"))?;
        let key = engine::precision::MarchKey64 {
            stop_on_escape: stop,
            ..key
        };
        path_a::forward::<Defaults, A>(&c.z, &key)
            .map(|f| (f, key.loop_values.dt_macro))
            .map_err(|e| format!("{e:?}"))
    };
    let (at, _) = forward(t_max, false)?;
    let end = Ic {
        m: at.start.m,
        r: at.state.r,
        p: at.state.p,
    };
    let lightest = tiers::lightest(start.m);
    let (i, j) = ((lightest + 1) % 3, (lightest + 2) % 3);
    let (stopped, dt) = forward(BURRAU_HORIZON, true)?;
    let (live, _) = forward(BURRAU_HORIZON, false)?;
    Ok(Sp1 {
        occupant,
        t_max,
        outcome: tiers::outcome(&end),
        pair_energy: pair_energy(&end, i, j),
        escaper_energy: tiers::e_rel(&end, lightest),
        label_at_t_max: (sd_state(at.state.packed_a), sd_detail(at.state.packed_a)),
        first_label: (
            sd_state(stopped.state.packed_a),
            sd_detail(stopped.state.packed_a),
            f64::from(tm_t_end_step(stopped.state.times)) * dt / scale,
        ),
        label: tiers::burrau_live(&stopped.state, &live.state, dt, scale, start.m),
        stopped: stopped.state,
        live: live.state,
        dt_macro: dt,
    })
}

/// SP-1 by the occupant's name in a fixture: `"heggie-rk4"` (the default occupant) or `"yoshida4"`.
pub fn sp1_named(occupant: &str, t_max: f64) -> Result<Sp1, String> {
    match occupant {
        "heggie-rk4" => sp1::<DefaultOccupant>("Heggie × RK4", t_max),
        "yoshida4" => sp1::<Yoshida4>("Yoshida-4 unregularised", t_max),
        other => Err(format!(
            "SP-1 refused: unknown occupant `{other}` (heggie-rk4, yoshida4)"
        )),
    }
}

// ── SP-2 ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Table 2's path under the workspace root.
pub const TABLE2: &str = "docs/incoming/2026-10-29_burrau_pzb2018/table2/table2_reconciled.csv";

/// Which of the paper's two runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sp2Run {
    /// Table 1's start.
    Unperturbed,
    /// Table 1's start with mass 3's `x` moved by `10⁻¹⁰`.
    Perturbed,
}

impl Sp2Run {
    /// The CSV's word.
    pub fn word(self) -> &'static str {
        match self {
            Self::Unperturbed => "unperturbed",
            Self::Perturbed => "perturbed",
        }
    }
}

/// One row of Table 2: a body's state at a time, in S&P's units (`G = 1`, masses 3, 4, 5).
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The run.
    pub run: Sp2Run,
    /// The time, 100 or 200.
    pub t: f64,
    /// The body, 0 for mass 3, 1 for mass 4, 2 for mass 5.
    pub body: usize,
    /// The position.
    pub r: [f64; 2],
    /// The velocity.
    pub v: [f64; 2],
    /// `x, y, vx, vy` as printed, every digit kept: the comparison reads them at the reference's precision.
    pub printed: [String; 4],
}

/// Table 2 from `text` (the CSV): 12 rows, each run at 100 and 200 for each body; `Err` names the first row it cannot
/// read or what is missing.
pub fn table2(text: &str) -> Result<Vec<Row>, String> {
    let mut lines = text.lines();
    if lines.next() != Some("run,t,mass,x,y,vx,vy") {
        return Err("Table 2: the header is not `run,t,mass,x,y,vx,vy`".into());
    }
    let rows: Vec<Row> = lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            let bad = || format!("Table 2: cannot read the row `{l}`");
            if f.len() != 7 {
                return Err(bad());
            }
            let run = match f[0] {
                "unperturbed" => Sp2Run::Unperturbed,
                "perturbed" => Sp2Run::Perturbed,
                _ => return Err(bad()),
            };
            let body = match f[2] {
                "3" => 0,
                "4" => 1,
                "5" => 2,
                _ => return Err(bad()),
            };
            let n: Vec<f64> = f[3..7]
                .iter()
                .map(|s| s.parse::<f64>().map_err(|_| bad()))
                .collect::<Result<_, _>>()?;
            if n.iter().any(|x| !x.is_finite()) {
                return Err(bad());
            }
            Ok(Row {
                run,
                t: f[1].parse().map_err(|_| bad())?,
                body,
                r: [n[0], n[1]],
                v: [n[2], n[3]],
                printed: [f[3], f[4], f[5], f[6]].map(str::to_owned),
            })
        })
        .collect::<Result<_, _>>()?;
    for run in [Sp2Run::Unperturbed, Sp2Run::Perturbed] {
        for t in [100.0, 200.0] {
            for body in 0..3 {
                let n = rows
                    .iter()
                    .filter(|r| r.run == run && r.t == t && r.body == body)
                    .count();
                if n != 1 {
                    return Err(format!(
                        "Table 2: {n} rows for the {} run at t = {t}, body {body}; one expected",
                        run.word()
                    ));
                }
            }
        }
    }
    Ok(rows)
}

/// Table 2's state for `run` at `t`, as printed, in the reference's phase-space order (`x₀ y₀ x₁ y₁ x₂ y₂` then the
/// velocities).
fn table_state(rows: &[Row], run: Sp2Run, t: f64) -> [String; 12] {
    let mut y: [String; 12] = Default::default();
    for r in rows.iter().filter(|r| r.run == run && r.t == t) {
        y[2 * r.body] = r.printed[0].clone();
        y[2 * r.body + 1] = r.printed[1].clone();
        y[6 + 2 * r.body] = r.printed[2].clone();
        y[7 + 2 * r.body] = r.printed[3].clone();
    }
    y
}

/// A decimal state as f64s, for the largest coordinate difference (reported).
fn f64s(y: &[String; 12]) -> [f64; 12] {
    std::array::from_fn(|k| y[k].parse().unwrap_or(f64::NAN))
}

/// An output as a phase-space point.
fn point(o: &Output) -> [f64; 12] {
    std::array::from_fn(|k| {
        if k < 6 {
            o.r[k / 2][k % 2]
        } else {
            o.v[(k - 6) / 2][k % 2]
        }
    })
}

/// SP-2's ladder for the blocking test: rungs of 16, 20 and 24 digits, accepted at the first two agreeing to `10⁻²`
/// in the scaled distance (gross breakage only; the criteria scale with the ladder's own `δ`, so a finer ladder only
/// tightens them). `--propose` runs [`SP2_PROPOSE_LADDER`].
pub const SP2_LADDER: Ladder = Ladder {
    first: 16,
    step: 4,
    rungs: 3,
    agree: 2,
    max_steps: 2_000_000,
    fixed_bits: None,
};

/// The ladder `--propose` measures the digits on: rungs of 20 to 36 digits, agreeing to `10⁻¹²`.
pub const SP2_PROPOSE_LADDER: Ladder = Ladder {
    first: 20,
    step: 4,
    rungs: 5,
    agree: 12,
    max_steps: 4_000_000,
    fixed_bits: None,
};

/// The perturbation: `x` of body 0 (mass 3) moved by `+10⁻¹⁰`.
pub const SP2_NUDGE: Nudge = Nudge {
    body: 0,
    axis: 0,
    numerator: 1,
    exponent: 10,
};

/// What an SP-2 run reverses and how its start is perturbed: the test's arms and the controls' single changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sp2Options {
    /// Reverse every velocity at `t = 100` (off only in the reversibility control).
    pub reverse: bool,
    /// The perturbation the start carries: [`SP2_NUDGE`] for the perturbed run, `None` for the unperturbed one (and
    /// the perturbed run's control).
    pub nudge: Option<Nudge>,
    /// The perturbation as an f64 start instead, `x = 1.0000000001` rounded to f64 (`--propose`'s comparison).
    pub f64_start: bool,
}

impl Sp2Options {
    /// The paper's own run: reversed at 100, the perturbed run nudged exactly.
    pub fn of(run: Sp2Run) -> Self {
        Self {
            reverse: true,
            nudge: (run == Sp2Run::Perturbed).then_some(SP2_NUDGE),
            f64_start: false,
        }
    }
}

/// One SP-2 comparison: a distance and the bound it is judged against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Check {
    /// What is compared.
    pub what: &'static str,
    /// The scaled distance.
    pub distance: f64,
    /// The largest absolute difference in any coordinate (reported).
    pub absolute: f64,
    /// The bound (the module's note).
    pub bound: f64,
}

impl Check {
    /// Whether the distance is within the bound.
    pub fn met(&self) -> bool {
        self.distance <= self.bound
    }

    /// The digits of agreement, `−log₁₀` of the scaled distance.
    pub fn digits(&self) -> f64 {
        -self.distance.log10()
    }
}

/// An SP-2 run: the converged ladder and its three checks.
#[derive(Clone, Debug)]
pub struct Sp2 {
    /// The run.
    pub run: Sp2Run,
    /// The options it ran under.
    pub options: Sp2Options,
    /// The converged ladder's rungs: digits, steps, evaluations.
    pub rungs: Vec<(u32, u64, u64)>,
    /// `δ(100)` and `δ(200)`: the accepted rung's scaled distance from the rung before it at each time.
    pub delta: [f64; 2],
    /// The accepted rung's energy error at 100 and 200.
    pub energy_error: [f64; 2],
    /// Against Table 2 at `t = 100`.
    pub at_100: Check,
    /// Against the start at `t = 200`: the reversibility check.
    pub reversibility: Check,
    /// Against Table 2 at `t = 200`: the trajectory check.
    pub trajectory: Check,
}

impl Sp2 {
    /// The verdict: every check within its bound, in the order t = 100, reversibility, trajectory; `Err` names the
    /// first that is not.
    pub fn verdict(&self) -> Result<(), String> {
        for c in [&self.at_100, &self.reversibility, &self.trajectory] {
            if !c.met() {
                return Err(format!(
                    "SP-2, the {} run: the {} check fails: scaled distance {:.3e} > its bound {:.3e} (largest \
                     coordinate difference {:.3e})",
                    self.run.word(),
                    c.what,
                    c.distance,
                    c.bound,
                    c.absolute
                ));
            }
        }
        Ok(())
    }

    /// The report's lines.
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![format!(
            "SP-2, the {} run (reversed at 100: {}; nudge {:?}; f64 start {}): rungs {:?} (digits, steps, \
             evaluations); δ(100) {:.3e}, δ(200) {:.3e}; energy error {:.3e} at 100, {:.3e} at 200",
            self.run.word(),
            self.options.reverse,
            self.options.nudge,
            self.options.f64_start,
            self.rungs,
            self.delta[0],
            self.delta[1],
            self.energy_error[0],
            self.energy_error[1]
        )];
        for c in [&self.at_100, &self.reversibility, &self.trajectory] {
            out.push(format!(
                "  {}: scaled {:.3e} ({:.1} digits), largest coordinate difference {:.3e}, bound {:.3e}: {}",
                c.what,
                c.distance,
                c.digits(),
                c.absolute,
                c.bound,
                if c.met() { "met" } else { "NOT met" }
            ));
        }
        out
    }
}

/// The largest absolute coordinate difference between two points.
fn absolute(a: &[f64; 12], b: &[f64; 12]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// SP-2 for `run` on `ladder` under `options`, Table 2 read from `table` (the CSV's text); `Err` where the table
/// cannot be read or the ladder does not converge.
pub fn sp2(run: Sp2Run, ladder: &Ladder, options: Sp2Options, table: &str) -> Result<Sp2, String> {
    let rows = table2(table)?;
    let published = tiers::burrau();
    let mut start = from_published(&published);
    if options.f64_start && run == Sp2Run::Perturbed {
        start.r[0][0] = 1.000_000_000_1;
    }
    let extras = Extras {
        nudge: if options.f64_start {
            None
        } else {
            options.nudge
        },
        reverse_at: options.reverse.then_some(100.0),
    };
    let c: Converged = brutus::converge_with(&start, &[100.0, 200.0], ladder, &extras)
        .map_err(|e| format!("SP-2, the {} run: the reference: {e}", run.word()))?;
    let n = c.runs.len();
    let (fine, coarse) = (&c.runs[n - 1], &c.runs[n - 2]);
    let at = |k: usize| point(&fine.outputs[k]);
    let delta = [0, 1].map(|k| brutus::disagreement_at(coarse, fine, k));
    // The run's own start as decimals: Table 1's, with the perturbed x exact (`1.0000000001`), or, for the f64 start,
    // that f64's exact expansion.
    let mut origin: [String; 12] = Default::default();
    for i in 0..3 {
        origin[2 * i] = format!("{}", published.r[i][0]);
        origin[2 * i + 1] = format!("{}", published.r[i][1]);
        origin[6 + 2 * i] = "0".into();
        origin[7 + 2 * i] = "0".into();
    }
    if run == Sp2Run::Perturbed {
        origin[0] = if options.f64_start {
            format!("{:.80}", start.r[0][0])
        } else {
            "1.0000000001".into()
        };
    }
    let t100 = table_state(&rows, run, 100.0);
    let t200 = table_state(&rows, run, 200.0);
    let distance = |k: usize, b: &[String; 12]| -> Result<f64, String> {
        let b: [&str; 12] = std::array::from_fn(|j| b[j].as_str());
        brutus::scaled_to_decimal(fine, k, &b)
    };
    // The table's own departure from the start at 200 (about 10⁻¹³), read in f64, which resolves it to 10⁻¹⁶.
    let table_return = {
        let o: [f64; 12] = f64s(&origin);
        let t: [f64; 12] = f64s(&t200);
        let m = published.m;
        brutus::scaled_f64(&t, &o, &m, published.g * (m[0] + m[1] + m[2]))
    };
    let check = |what, k: usize, b: &[String; 12], bound| -> Result<Check, String> {
        Ok(Check {
            what,
            distance: distance(k, b)?,
            absolute: absolute(&at(k), &f64s(b)),
            bound,
        })
    };
    Ok(Sp2 {
        run,
        options,
        rungs: c
            .runs
            .iter()
            .map(|r| (r.rung.digits, r.accepted, r.evaluations))
            .collect(),
        delta,
        energy_error: [fine.outputs[0].energy_error, fine.outputs[1].energy_error],
        at_100: check("t = 100 against Table 2", 0, &t100, delta[0])?,
        reversibility: check(
            "t = 200 reversibility (against the start)",
            1,
            &origin,
            delta[1],
        )?,
        trajectory: check(
            "t = 200 trajectory (against Table 2)",
            1,
            &t200,
            delta[1] + table_return,
        )?,
    })
}

/// Table 2's text under `root`.
pub fn table2_text(root: &Path) -> Result<String, String> {
    std::fs::read_to_string(root.join(TABLE2)).map_err(|e| format!("{TABLE2}: {e}"))
}

// ── PL-8a ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// PL-8a's window, in S&P's units: to `t ≈ 60`.
pub const PL8A_WINDOW: f64 = 60.0;

/// PL-8a's sampling interval, in S&P's units.
pub const PL8A_SAMPLE: f64 = 0.5;

/// PL-8a's parting threshold, proposed (a calibration): the chord, relative to the system's size and speed, at `10⁻²`,
/// a hundredth of the system, past which the two runs are no longer the same orbit at a figure's resolution. The
/// parting time moves by `ln 10/λ` per decade of it (`--propose` prints each decade's).
pub const PL8A_PARTING: f64 = 1e-2;

/// f64's horizon, in S&P's units: `ln(1/ε)/λ ≈ 52` at `λ ≈ 0.7` (dd_validation_orbits §1.5).
pub const PL8A_F64_HORIZON: f64 = 52.0;

/// PL-8a's reference ladder: rungs from 20 digits, accepted at the first two agreeing to `10⁻⁸` over the window, six
/// decades below the parting threshold.
pub const PL8A_LADDER: Ladder = Ladder {
    first: 20,
    step: 4,
    rungs: 3,
    agree: 8,
    max_steps: 2_000_000,
    fixed_bits: None,
};

/// The width a march runs at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Width {
    /// f64.
    F64,
    /// f32: the f64 start rounded to f32.
    F32,
}

/// One width's chord over the window.
#[derive(Clone, Debug)]
pub struct Chord {
    /// The width.
    pub width: Width,
    /// `(t in S&P's units, chord)` at each sample.
    pub samples: Vec<(f64, f64)>,
}

impl Chord {
    /// The first sample time whose chord exceeds `threshold`; `None` where none does.
    pub fn parting(&self, threshold: f64) -> Option<f64> {
        self.samples
            .iter()
            .find(|(_, c)| c.is_nan() || *c > threshold)
            .map(|(t, _)| *t)
    }
}

/// PL-8a's measurement: the reference's convergence and the chords.
#[derive(Clone, Debug)]
pub struct Pl8a {
    /// The reference's rungs' digits and its last disagreement over the window.
    pub reference: (Vec<u32>, f64),
    /// The run judged as f64's (the f64 march, or in the control the f32 one in its place).
    pub judged: Chord,
    /// The f32 march.
    pub f32: Chord,
}

impl Pl8a {
    /// The verdict (the module's note): the judged run does not part before [`PL8A_F64_HORIZON`]; the f32 run parts
    /// in the window and before it. `Err` names what failed.
    pub fn verdict(&self) -> Result<(), String> {
        let judged = self.judged.parting(PL8A_PARTING);
        if let Some(t) = judged.filter(|&t| t < PL8A_F64_HORIZON) {
            return Err(format!(
                "PL-8a: the run judged as f64's parts from the reference at {t} of S&P's units, before f64's horizon \
                 {PL8A_F64_HORIZON} (chord > {PL8A_PARTING:e})"
            ));
        }
        let f32 = self.f32.parting(PL8A_PARTING).ok_or_else(|| {
            format!(
                "PL-8a: the f32 run does not part by {PL8A_WINDOW}, so its control cannot fire (chord ≤ \
                 {PL8A_PARTING:e} throughout)"
            )
        })?;
        if judged.is_some_and(|t| t <= f32) {
            return Err(format!(
                "PL-8a: the f32 run parts at {f32}, not before the f64 run ({judged:?})"
            ));
        }
        Ok(())
    }

    /// The report's lines.
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![format!(
            "PL-8a: the reference's rungs {:?}, their last disagreement over the window {:.3e}; parting threshold \
             {PL8A_PARTING:e}, f64's horizon {PL8A_F64_HORIZON}",
            self.reference.0, self.reference.1
        )];
        for c in [&self.judged, &self.f32] {
            out.push(format!(
                "  {:?}: parts at {:?} of S&P's units; chord at 10, 20, …, 60: {}",
                c.width,
                c.parting(PL8A_PARTING),
                c.samples
                    .iter()
                    .filter(|(t, _)| (t / 10.0).fract().abs() < 1e-9)
                    .map(|(t, ch)| format!("{t}: {ch:.2e}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        out
    }
}

/// The march's state as the reference's phase-space point.
fn march_point<R: Copy + Into<f64>>(m: [f64; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> [f64; 12] {
    std::array::from_fn(|k| {
        if k < 6 {
            r[k / 2][k % 2].into()
        } else {
            let (i, c) = ((k - 6) / 2, k % 2);
            p[i][c].into() / m[i]
        }
    })
}

/// PL-8a's samples: each a sync boundary `k` near `j·`[`PL8A_SAMPLE`] of S&P's units, its time `k·Δ_sync` and the
/// macro-step count `n` that lands the march mid-interval after it, so its state is boundary `k`'s at either width.
fn samples(dt: f64, scale: f64) -> Result<Vec<(u32, f64, u32)>, String> {
    let delta: f64 = delta_sync::<f64>();
    let mut out: Vec<(u32, f64, u32)> = Vec::new();
    let count = (PL8A_WINDOW / PL8A_SAMPLE).round() as u32;
    for j in 1..=count {
        let k = (f64::from(j) * PL8A_SAMPLE * scale / delta).floor() as u32;
        if out.last().is_some_and(|s| s.0 == k) {
            continue;
        }
        let n = ((f64::from(k) + 0.5) * delta / dt).round() as u32;
        let at64 = boundary_at_or_before::<f64>(f64::from(n) * dt, delta);
        let at32 = boundary_at_or_before::<f32>(n as f32 * dt as f32, delta_sync::<f32>());
        if at64 != k || at32 != k {
            return Err(format!(
                "PL-8a: n = {n} lands on boundary {at64} at f64 and {at32} at f32, not {k}"
            ));
        }
        out.push((k, f64::from(k) * delta, n));
    }
    Ok(out)
}

/// PL-8a on `ladder`, the f64 arm marched at `judged` (f64, or f32 in the control) and the f32 arm at f32.
pub fn pl8a(ladder: &Ladder, judged: Width) -> Result<Pl8a, String> {
    let start = tiers::burrau();
    let x = unit_state(&start)?;
    let scale = burrau_time_scale();
    let c = path_a::check1::<Defaults>(&x);
    let key = march_key(PL8A_WINDOW * scale, BENCHMARK_R_COLL).map_err(|e| format!("{e:?}"))?;
    let dt = key.loop_values.dt_macro;
    let at = samples(dt, scale)?;
    let times: Vec<f64> = at.iter().map(|s| s.1).collect();
    let reference = brutus::converge(&from_ic(&c.decoded), &times, ladder)
        .map_err(|e| format!("PL-8a: the reference: {e}"))?;
    let rungs: Vec<u32> = reference.runs.iter().map(|r| r.rung.digits).collect();
    let last = *reference.disagreement.last().unwrap_or(&f64::INFINITY);
    let refr = reference.accepted();
    let m = c.decoded.m;
    let gm = m[0] + m[1] + m[2];
    let x64 = c.decoded;
    let x32 = Ic {
        m: x64.m.map(|v| v as f32),
        r: x64.r.map(|a| a.map(|v| v as f32)),
        p: x64.p.map(|a| a.map(|v| v as f32)),
    };
    let chord = |width: Width| -> Chord {
        let samples = at
            .iter()
            .enumerate()
            .map(|(i, &(_, t, n))| {
                let y = match width {
                    Width::F64 => {
                        let u = WrapperUniforms::<f64> {
                            horizon_steps: n,
                            ..key.loop_values
                        };
                        let s = integrate_detecting::<DefaultOccupant, f64>(&x64, &u, key.radius);
                        march_point(m, &s.r, &s.p)
                    }
                    Width::F32 => {
                        let u = WrapperUniforms::<f32> {
                            dt_macro: dt as f32,
                            horizon_steps: n,
                            n_max: key.loop_values.n_max,
                            thresholds: key.loop_values.thresholds,
                        };
                        let s = integrate_detecting::<DefaultOccupant, f32>(&x32, &u, key.radius);
                        march_point(m, &s.r, &s.p)
                    }
                };
                let o = point(&refr.outputs[i]);
                // In S&P's units, to the half unit it was sampled near.
                let ts = (t / scale / PL8A_SAMPLE).round() * PL8A_SAMPLE;
                (ts, brutus::scaled_f64(&y, &o, &m, gm))
            })
            .collect();
        Chord { width, samples }
    };
    Ok(Pl8a {
        reference: (rungs, last),
        judged: chord(judged),
        f32: chord(Width::F32),
    })
}

// ── The gate ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// `burrau-smoke`: SP-1 on its inputs; SP-2 and PL-8a under `--target`.
pub struct BurrauSmokeGate;

/// An SP-1 input.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    description: String,
    occupant: String,
    t_max: f64,
}

impl Gate for BurrauSmokeGate {
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
        if config.threshold.value != BURRAU_WINDOW[1] {
            return Err(format!(
                "refused: gate.json's threshold {} is not the t_end window's upper edge {} (tiers::BURRAU_WINDOW)",
                config.threshold.value, BURRAU_WINDOW[1]
            ));
        }
        let run = sp1_named(&input.occupant, input.t_max)?;
        let verdict = run.verdict();
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
            scatter: run.lines(),
            regions: Some(Regions {
                count: RegionCount::Recorded(1),
                minimum: config.min_regions.clone(),
            }),
            negative_results: vec![match verdict {
                Ok(()) => "none: SP-1 met".to_owned(),
                Err(e) => e,
            }],
        })
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        let usage = || {
            format!(
                "gate {NAME}: unrecognised flags `{}`; usage: --target SP-1|SP-2|PL-8a [--propose]",
                args.join(" ")
            )
        };
        let (target, propose) = match args {
            ["--target", t] => (*t, false),
            ["--target", t, "--propose"] => (*t, true),
            _ => return Err(usage()),
        };
        let text = match (target, propose) {
            ("SP-1", false) => {
                let r = sp1::<DefaultOccupant>("Heggie × RK4", SP1_T_MAX)?;
                let lines = r.lines().join("\n");
                r.verdict().map(|()| lines)?
            }
            ("SP-1", true) => propose_sp1()?,
            ("SP-2", false) => {
                let table = table2_text(root)?;
                let mut out = Vec::new();
                for run in [Sp2Run::Unperturbed, Sp2Run::Perturbed] {
                    let r = sp2(run, &SP2_LADDER, Sp2Options::of(run), &table)?;
                    out.extend(r.lines());
                    r.verdict()?;
                }
                out.join("\n")
            }
            ("SP-2", true) => propose_sp2(root)?,
            ("PL-8a", false) => {
                let r = pl8a(&PL8A_LADDER, Width::F64)?;
                let lines = r.lines().join("\n");
                r.verdict().map(|()| lines)?
            }
            ("PL-8a", true) => propose_pl8a()?,
            (other, _) => {
                return Err(format!(
                    "gate {NAME}: unknown target `{other}` (SP-1, SP-2, PL-8a)"
                ))
            }
        };
        println!("{text}");
        Ok(())
    }
}

// ── The proposals (R-71: measured, stated, confirmed by the owner; they assert nothing) ─────────────────────────

/// SP-1's window proposal: the measured times beside the literature's.
pub fn propose_sp1() -> Result<String, String> {
    let r = sp1::<DefaultOccupant>("Heggie × RK4", SP1_T_MAX)?;
    let start = tiers::burrau();
    let x = unit_state(&start)?;
    let scale = burrau_time_scale();
    let c = path_a::check1::<Defaults>(&x);
    let key = march_key(BURRAU_HORIZON * scale, BENCHMARK_R_COLL).map_err(|e| format!("{e:?}"))?;
    let dt = key.loop_values.dt_macro;
    let samples: Vec<(f64, Ic<f64>)> = tiers::ejection_samples()
        .into_iter()
        .map(|ts| {
            let mut k = key;
            k.loop_values.horizon_steps = (ts * scale / dt).round() as u32;
            let f = path_a::forward::<Defaults, DefaultOccupant>(&c.z, &k)
                .map_err(|e| format!("{e:?}"))?;
            Ok((
                ts,
                Ic {
                    m: f.start.m,
                    r: f.state.r,
                    p: f.state.p,
                },
            ))
        })
        .collect::<Result<_, String>>()?;
    let onset = tiers::ejection_time(&samples, tiers::lightest(start.m));
    Ok(format!(
        "SP-1's t_end window (a proposal, asserting nothing)\n\
         - t_max: {SP1_T_MAX} of S&P's units (range {SP1_T_MAX_RANGE:?}); the default horizon is unchanged.\n\
         - the lightest body unbound for good from {onset:?} of S&P's units (sampled every 0.5; the ejection onset).\n\
         - the default rule's first escape label: {:.3} of S&P's units, naming body {}.\n\
         - the literature: Szebehely & Peters 1967, disruption near 60 (dd_validation_orbits §1.5); Portegies Zwart & \
           Boekholt 2018, escape at t ≈ 63.4.\n\
         - proposed: the label's window stays {BURRAU_WINDOW:?} on the march to {BURRAU_HORIZON} (its lower edge ten \
           units under the literature's disruption, its upper edge the horizon); SP-1 at t_max reads the class and the \
           escaper structurally, since the label lags the onset by {:.1} units.\n{}",
        r.first_label.2,
        r.first_label.1,
        r.first_label.2 - onset.unwrap_or(f64::NAN),
        r.lines().join("\n")
    ))
}

/// SP-2's proposal: the digits of agreement on a finer ladder, per check and run, and the f64 start beside the exact
/// one.
pub fn propose_sp2(root: &Path) -> Result<String, String> {
    let table = table2_text(root)?;
    let mut out = vec![format!(
        "SP-2's agreement (a proposal, asserting nothing): ladder {SP2_PROPOSE_LADDER:?}"
    )];
    for run in [Sp2Run::Unperturbed, Sp2Run::Perturbed] {
        let r = sp2(run, &SP2_PROPOSE_LADDER, Sp2Options::of(run), &table)?;
        out.extend(r.lines());
    }
    let f64_start = Sp2Options {
        f64_start: true,
        ..Sp2Options::of(Sp2Run::Perturbed)
    };
    let r = sp2(Sp2Run::Perturbed, &SP2_PROPOSE_LADDER, f64_start, &table)?;
    out.push("the perturbed run from the f64 start x = 1.0000000001 instead:".into());
    out.extend(r.lines());
    Ok(out.join("\n"))
}

/// PL-8a's proposal: the parting time at each threshold decade at both widths, and the chord's growth rate.
pub fn propose_pl8a() -> Result<String, String> {
    let r = pl8a(&PL8A_LADDER, Width::F64)?;
    let mut out = vec!["PL-8a's parting threshold (a proposal, asserting nothing)".to_owned()];
    out.extend(r.lines());
    for e in 1..=10 {
        let th = 10f64.powi(-e);
        out.push(format!(
            "  threshold 1e-{e}: f64 parts at {:?}, f32 at {:?}",
            r.judged.parting(th),
            r.f32.parting(th)
        ));
    }
    for c in [&r.judged, &r.f32] {
        // The growth rate: a least-squares line through ln(chord) over the samples where the chord is between 10⁻¹²
        // and 10⁻², past the start's transient.
        let pts: Vec<(f64, f64)> = c
            .samples
            .iter()
            .filter(|(t, ch)| *t >= 20.0 && *ch > 1e-12 && *ch < 1e-2)
            .map(|(t, ch)| (*t, ch.ln()))
            .collect();
        let n = pts.len() as f64;
        let (sx, sy) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
        let (mx, my) = (sx / n, sy / n);
        let num: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
        let den: f64 = pts.iter().map(|p| (p.0 - mx) * (p.0 - mx)).sum();
        out.push(format!(
            "  {:?}: the chord's growth rate over t ≥ 20, chord in (1e-12, 1e-2): {:.3} per unit of S&P's time ({} \
             samples)",
            c.width,
            num / den,
            pts.len()
        ));
    }
    Ok(out.join("\n"))
}
