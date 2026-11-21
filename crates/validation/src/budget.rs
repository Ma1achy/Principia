//! The regularised integration floor's calibrations (integrator_contract Part 3, Part 6; R-564, R-604; TASK-M3-41):
//! **`step-budget`**, a measurement (R-649), and its `--propose`, the step budget's rate (REQ-INT-103) and its floor
//! `N_min` (REQ-INT-108), each an R-71 calibration the human confirms at the M3 gate.
//!
//! **What is measured** (written before any run): Heggie × RK4, the default occupant, at f32, the GPU's precision, at
//! the CPU field's step-control values (`engine::field`: `η = 0.01`, `Δ_sync` the register's, `TINY = 10⁻³⁷`,
//! `f = 0.02`, R-998's landing), on a [`SIDE`] × [`SIDE`] grid of cell centres over each target's window (fixtures/
//! case_matrix.toml): `burrau_nu_k`, `latent_mixed_h3`, `config_stability` and the clean control `latent_shape`. Each
//! copy marches to `t = 50` with no terminal (no collision radius, no escape stop) and under a measuring budget of
//! [`CAP_RATE`] steps per unit time, far above any the proposal could make, so its step counts are those of the march
//! itself, not of where an event or the budget stopped it (pitfalls §3: a run stopped early is its own discriminator).
//! At each sync boundary `t_k = k·Δ_sync` the copy's running step total `N(t_k)` (every stepper call and landing
//! retake, the budget's own count) is read, and the boundary whose interval holds the copy's first close encounter
//! (a pair nearer than `r_close`, the register's 0.01) is noted.
//!
//! **Per copy, at each horizon `H` of 13 and 50:** its mean rate `N(H)/H`, and its step total at its earliest close
//! encounter, read at the end of the interval that holds it (an upper bound, by at most one interval's steps); each
//! reported as p50, p99 and max over the target's copies.
//!
//! **The proposal**: the rate first, the sustained one, the greatest mean rate `N(H)/H` over every copy and both
//! horizons, rounded up to two significant figures; then `N_min` what the rate leaves uncovered, the least power of two
//! at or above the greatest `N(t_k)` over the boundaries where `N(t_k) > rate·t_{k−1}`. The budget is read every step,
//! and within an interval the steps grow while the allowance grows with `t`, so `N(t_k) ≤ max(N_min, rate·t_{k−1})` at
//! every boundary holds every step between, and no copy runs out before its first close encounter is passed. The plan
//! is unclear here (it asks for a rate and a floor that protects early encounters, not how the two share the count);
//! chose the rate as the sustained count and the floor as what it leaves, because a floor taken first, from the
//! encounters alone, covered whole runs on these targets (a first encounter at a running total of 4.1e5) and left no
//! rate to propose, a constant cap where R-564 asks for a rate.
//! **Its check**, written before any run: every copy re-marched under the proposed `(rate, N_min)`, through the
//! occupant's own every-step budget, runs out nowhere (zero exhausted); and under half the proposed rate at least one
//! copy runs out, so the check can fire.
//!
//! **`config_stability` is not measured:** its window is prin-rs's ten-component GLSL latent (`z_glsl`, fixtures/
//! slices.toml), which no decoder here takes; the report says so rather than standing another window in for it. The
//! plan is unclear here (it names the target); chose to report it unmeasured, because a substituted window would be a
//! different target under its name.
//!
//! A copy that ran out of even the measuring budget, or whose decode is degenerate, is counted and reported, never
//! dropped silently.

use std::fmt::Write as _;
use std::path::Path;
use std::thread;

use serde::Deserialize;

use kernel::chart::burrau::{decode as burrau_decode, Burrau};
use kernel::decode::{self, Latent};
use kernel::driver::advance::Cadence;
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::regularisation::az::{End, ExplicitTest};
use kernel::regularisation::heggie::{self as hg, HgParams, HgStanding, DEFAULT_TIME};
use kernel::regularisation::sync::delta_sync;

use crate::gate::report::Verdict;
use crate::gate::{MeasurementConfig, MeasurementControl, MeasurementGate};

/// The gate's name.
pub const STEP_BUDGET_NAME: &str = "step-budget";

/// The copies per axis of each target's grid: `SIDE²` copies a target. Fixed before any run.
pub const SIDE: usize = 8;

/// The horizons the rates are read at, the targets' `t_max` (fixtures/case_matrix.toml).
pub const HORIZONS: [f64; 2] = [13.0, 50.0];

/// The measuring budget's rate: 25 times R-564's starting value, so no copy the proposal could clear runs out of it.
pub const CAP_RATE: f64 = 1.0e6;

/// The measuring budget's floor.
pub const CAP_N_MIN: u32 = 10_000_000;

/// A target's window.
#[derive(Clone, Copy, Debug)]
pub enum Window {
    /// `latent_axes(gallery_z0, a, b)` over `[−half, half]²` about the origin.
    Latent { a: usize, b: usize, half: f64 },
    /// The `(ν, K)` chart, `burrau_family`, over its unit square's `[0.05, 0.95]²`.
    Burrau,
    /// A window no decoder here takes: the reason.
    Undecodable(&'static str),
}

/// A target: its case's name and window.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub name: &'static str,
    pub window: Window,
}

/// `gallery_z0` (fixtures/case_matrix.toml): `z_α, z_β, z_q[4], z_μ[2]`, the latent's block order.
pub const GALLERY_Z0: [f64; 8] = [0.35, -0.45, 0.25, -0.15, 0.40, 0.05, 0.20, -0.30];

/// The budget's worst targets and the clean control, as the task names them.
pub const TARGETS: [Target; 4] = [
    Target {
        name: "burrau_nu_k",
        window: Window::Burrau,
    },
    Target {
        name: "latent_mixed_h3",
        window: Window::Latent {
            a: 0,
            b: 4,
            half: 3.0,
        },
    },
    Target {
        name: "config_stability",
        window: Window::Undecodable(
            "prin-rs's ten-component GLSL latent (`z_glsl`, fixtures/slices.toml), which no decoder here takes",
        ),
    },
    Target {
        name: "latent_shape",
        window: Window::Latent {
            a: 0,
            b: 1,
            half: 1.5,
        },
    },
];

/// The `(ν, K)` chart of `burrau_nu_k` (fixtures/case_matrix.toml): `ν ∈ [0.05, 0.95]`, `K_max = 4`, `γ_K = 1.5`.
fn burrau_chart() -> Burrau<f32> {
    Burrau {
        nu_range: [0.05, 0.95],
        k_max: 4.0,
        gamma_k: 1.5,
    }
}

/// Cell `i` of `SIDE` across `[lo, hi]` at the fraction `at` of its width: 0.5 its centre, the measured grid's.
fn cell_at(i: usize, lo: f64, hi: f64, at: f64) -> f64 {
    lo + (hi - lo) * (i as f64 + at) / SIDE as f64
}

/// The held-out grid's place in each cell: a quarter of the way across, so no copy is one the proposal measured.
pub const HELD_OUT: f64 = 0.25;

/// The copies of `target`'s grid at f32: each decoded state, or `None` for a degenerate decode; empty for a window no
/// decoder takes.
pub fn copies(target: &Target) -> Vec<Option<State<f32>>> {
    copies_at(target, 0.5)
}

/// [`copies`] with each copy at the fraction `at` of its cell's width.
pub fn copies_at(target: &Target, at: f64) -> Vec<Option<State<f32>>> {
    let mut out = Vec::new();
    for j in 0..SIDE {
        for i in 0..SIDE {
            let state = match target.window {
                Window::Latent { a, b, half } => {
                    let mut z = GALLERY_Z0.map(|x| x as f32);
                    z[a] = cell_at(i, -half, half, at) as f32;
                    z[b] = cell_at(j, -half, half, at) as f32;
                    let d = decode::decode::<Defaults, f32>(&Latent::new(z));
                    d.degenerate.is_none().then_some(State {
                        m: d.ic.m,
                        r: d.ic.r,
                        p: d.ic.p,
                    })
                }
                Window::Burrau => {
                    let chart = burrau_chart();
                    let (d, _) = burrau_decode::<f32, f32>(
                        &chart,
                        cell_at(i, 0.05, 0.95, at) as f32,
                        cell_at(j, 0.05, 0.95, at) as f32,
                    );
                    d.degenerate.is_none().then_some(State {
                        m: d.ic.m,
                        r: d.ic.r,
                        p: d.ic.p,
                    })
                }
                Window::Undecodable(_) => return Vec::new(),
            };
            out.push(state);
        }
    }
    out
}

/// The CPU field's Heggie values at f32 under the budget `rate`, `n_min` (the module's note).
pub fn params(rate: f64, n_min: u32) -> HgParams<f32> {
    HgParams {
        eta: 0.01,
        delta_sync: delta_sync::<f32>(),
        tiny: 1e-37,
        predictive_f: 0.02,
        step_budget_rate: rate as f32,
        n_min,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        time: DEFAULT_TIME,
    }
}

/// A cadence that never ends the march and notes whether any kept step's state had a pair nearer than `r_close`.
struct Close {
    r_close: f32,
    seen: bool,
}

impl Cadence<f32> for Close {
    fn after_step(&mut self, s: &mut State<f32>, _: &mut Geom<f32>) {
        let d = |i: usize, j: usize| {
            let (x, y) = (s.r[i][0] - s.r[j][0], s.r[i][1] - s.r[j][1]);
            (x * x + y * y).sqrt()
        };
        if d(0, 1).min(d(0, 2)).min(d(1, 2)) < self.r_close {
            self.seen = true;
        }
    }

    fn done(&self) -> bool {
        false
    }
}

/// One copy's march, read at its sync boundaries.
#[derive(Clone, Debug, Default)]
pub struct Record {
    /// `(t_k, N(t_k))` at each boundary landed on, `k ≥ 1`.
    pub boundaries: Vec<(f64, u32)>,
    /// The boundary that ends the interval holding the first close encounter, as an index into `boundaries`.
    pub first_close: Option<usize>,
    /// How the march ended: [`End::Marching`] where it reached `t = 50`.
    pub end: Option<End>,
}

/// `state`'s march to `t = 50` under `params`, boundary by boundary (the module's note).
pub fn march(state: &State<f32>, params: &HgParams<f32>) -> Record {
    let last = (HORIZONS[1] / f64::from(params.delta_sync)).round() as u32;
    let mut close = Close {
        r_close: ledger::constants::R_CLOSE.number() as f32,
        seen: false,
    };
    let mut out = hg::march::<Rk4, f32, _, _>(state, 1, params, &mut close, &mut ExplicitTest);
    let mut rec = Record::default();
    let mut k = 1;
    loop {
        rec.boundaries.push((f64::from(out.t), out.count.steps));
        if close.seen && rec.first_close.is_none() {
            rec.first_close = Some(rec.boundaries.len() - 1);
        }
        if out.count.end != End::Marching || k >= last {
            rec.end = Some(out.count.end);
            return rec;
        }
        k += 1;
        let from = HgStanding {
            q: out.q,
            p: out.p,
            boundary: out.boundary,
            count: out.count,
        };
        out = hg::march_on::<Rk4, f32, _, _>(
            &out.system,
            &out.state,
            from,
            k,
            params,
            &mut close,
            &mut ExplicitTest,
        );
    }
}

/// `f` over `items` on the machine's threads, in order.
fn parallel<T: Sync, U: Send>(items: &[T], f: impl Fn(&T) -> U + Sync) -> Vec<U> {
    let n = thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = items.len().div_ceil(n).max(1);
    thread::scope(|s| {
        let handles: Vec<_> = items
            .chunks(chunk)
            .map(|c| s.spawn(|| c.iter().map(&f).collect::<Vec<U>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a worker"))
            .collect()
    })
}

/// A copy's running total at its earliest close encounter, read at the end of its interval, or `None` for none.
pub fn at_first_close(r: &Record) -> Option<u32> {
    r.first_close.map(|i| r.boundaries[i].1)
}

/// The copy's mean rate to `h`: `N(h)/h`, or `None` where it did not reach `h`.
pub fn mean_rate(r: &Record, h: f64) -> Option<f64> {
    r.boundaries
        .iter()
        .find(|(t, _)| (*t - h).abs() < 1e-6)
        .map(|&(t, n)| f64::from(n) / t)
}

/// What `rate` leaves `N_min` to cover in `r` (the module's note): the greatest `N(t_k)` over its boundaries where
/// `N(t_k) > rate·t_{k−1}`, `t_0 = 0`; 0 where the rate covers every boundary.
pub fn uncovered(r: &Record, rate: f64) -> f64 {
    let mut prev = 0.0;
    let mut worst: f64 = 0.0;
    for &(t, n) in &r.boundaries {
        if f64::from(n) > rate * prev {
            worst = worst.max(f64::from(n));
        }
        prev = t;
    }
    worst
}

/// The `q` quantile of `xs` (nearest rank), `NaN` for none.
pub fn quantile(xs: &[f64], q: f64) -> f64 {
    if xs.is_empty() {
        return f64::NAN;
    }
    let mut v = xs.to_vec();
    v.sort_by(f64::total_cmp);
    let i = ((q * v.len() as f64).ceil() as usize).clamp(1, v.len()) - 1;
    v[i]
}

/// `x` rounded up to two significant figures; 0 for `x ≤ 0`.
pub fn up_two_figures(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let e = x.log10().floor() - 1.0;
    let unit = 10f64.powf(e);
    (x / unit).ceil() * unit
}

/// One target's measurement: its records, and the copies that were degenerate.
pub struct Measured {
    pub target: Target,
    pub records: Vec<Record>,
    pub degenerate: usize,
}

/// Every target measured under the measuring budget.
pub fn measure() -> Vec<Measured> {
    let cap = params(CAP_RATE, CAP_N_MIN);
    TARGETS
        .iter()
        .map(|t| {
            let copies = copies(t);
            let states: Vec<State<f32>> = copies.iter().flatten().copied().collect();
            Measured {
                target: *t,
                degenerate: copies.len() - states.len(),
                records: parallel(&states, |s| march(s, &cap)),
            }
        })
        .collect()
}

/// `p50 / p99 / max` of `xs`, formatted.
fn stats(xs: &[f64]) -> String {
    format!(
        "{:.4e} / {:.4e} / {:.4e}",
        quantile(xs, 0.5),
        quantile(xs, 0.99),
        quantile(xs, 1.0)
    )
}

/// The copies of `m` that ran out under `params`, re-marched through the occupant's own every-step budget.
fn exhausted(m: &[Measured], params: &HgParams<f32>) -> usize {
    m.iter()
        .map(|t| {
            let states: Vec<State<f32>> = copies(&t.target).into_iter().flatten().collect();
            parallel(&states, |s| march(s, params).end == Some(End::Exhausted))
                .into_iter()
                .filter(|&e| e)
                .count()
        })
        .sum()
}

/// `--propose`'s report (the module's note).
pub fn propose() -> Result<String, String> {
    let measured = measure();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {STEP_BUDGET_NAME} --propose (REQ-INT-103, REQ-INT-108)\n\nHeggie × RK4 at f32, the CPU field's values, \
         {SIDE} × {SIDE} copies a target, each marched to t = 50 with no terminal under a measuring budget of \
         {CAP_RATE:e} steps per unit time (floor {CAP_N_MIN}). Running step totals read at each sync boundary.\n"
    );
    let mut closes: Vec<f64> = Vec::new();
    let mut over_cap = 0;
    for m in &measured {
        if let Window::Undecodable(why) = m.target.window {
            let _ = writeln!(
                out,
                "## {}\n\nNot measured: its window is {why}.\n",
                m.target.name
            );
            continue;
        }
        let capped = m
            .records
            .iter()
            .filter(|r| r.end != Some(End::Marching))
            .count();
        over_cap += capped;
        let close: Vec<f64> = m
            .records
            .iter()
            .filter_map(at_first_close)
            .map(f64::from)
            .collect();
        closes.extend(&close);
        let _ = writeln!(
            out,
            "## {}\n\n{} copies, {} degenerate decode(s), {} not reaching t = 50 under the measuring budget \
             (ended {:?}); {} with a close encounter (r < r_close).\n\n- running total at the earliest close \
             encounter, p50 / p99 / max: {}",
            m.target.name,
            m.records.len(),
            m.degenerate,
            capped,
            m.records
                .iter()
                .filter_map(|r| r.end)
                .filter(|e| *e != End::Marching)
                .collect::<Vec<_>>(),
            close.len(),
            stats(&close)
        );
        for h in HORIZONS {
            let rates: Vec<f64> = m.records.iter().filter_map(|r| mean_rate(r, h)).collect();
            let _ = writeln!(
                out,
                "- mean rate N(t)/t at t = {h}, p50 / p99 / max: {}",
                stats(&rates)
            );
        }
        out.push('\n');
    }
    let worst_close = closes.iter().copied().fold(0.0, f64::max);
    let marched = |m: &Measured| -> Vec<Record> {
        m.records
            .iter()
            .filter(|r| r.end == Some(End::Marching))
            .cloned()
            .collect()
    };
    let mut worst_mean: f64 = 0.0;
    for m in &measured {
        for h in HORIZONS {
            for r in marched(m) {
                worst_mean = worst_mean.max(mean_rate(&r, h).unwrap_or(0.0));
            }
        }
    }
    let rate = up_two_figures(worst_mean);
    let mut worst_burst: f64 = 0.0;
    let mut bursts_by_target = Vec::new();
    for m in &measured {
        let bursts: Vec<f64> = marched(m).iter().map(|r| uncovered(r, rate)).collect();
        worst_burst = bursts.iter().copied().fold(worst_burst, f64::max);
        if !bursts.is_empty() {
            bursts_by_target.push((m.target.name, bursts));
        }
    }
    let n_min = (worst_burst.max(1.0).ceil() as u32).next_power_of_two();
    for (name, bursts) in &bursts_by_target {
        let _ = writeln!(
            out,
            "{name}: the running total the rate leaves uncovered, max N(t_k) where N(t_k) > rate·t_(k−1), p50 / p99 / \
             max: {}",
            stats(bursts)
        );
    }
    for m in &measured {
        let cleared = marched(m)
            .iter()
            .filter_map(|r| {
                let i = r.first_close?;
                let t_prev = if i == 0 { 0.0 } else { r.boundaries[i - 1].0 };
                Some(f64::from(r.boundaries[i].1) <= (rate * t_prev).max(f64::from(n_min)))
            })
            .collect::<Vec<bool>>();
        if !cleared.is_empty() {
            let _ = writeln!(
                out,
                "{}: copies whose earliest close encounter's interval clears max(N_min, rate·t): {} of {}",
                m.target.name,
                cleared.iter().filter(|&&c| c).count(),
                cleared.len()
            );
        }
    }
    let _ = writeln!(
        out,
        "\n**Proposed rate = {rate:e} steps per unit time per copy** (the greatest mean rate N(H)/H, {worst_mean:.4e}, \
         rounded up to two figures); **proposed N_min = {n_min}** (the least power of two at or above the greatest \
         running total the rate leaves uncovered, {worst_burst:.0}). The greatest running total at an earliest close encounter is \
         {worst_close:.0}. {over_cap} copies that did not reach t = 50 under the measuring budget are outside the \
         proposal and listed above."
    );
    let at = exhausted(&measured, &params(rate, n_min));
    let half = exhausted(&measured, &params(rate / 2.0, n_min));
    let _ = writeln!(
        out,
        "\nCheck, through the occupant's every-step budget: at the proposed (rate, N_min) {at} copies run out \
         (criterion: 0) — {}; at half the rate {half} run out (criterion: at least 1, the check can fire) — {}.",
        if at == 0 { "pass" } else { "FAIL" },
        if half > 0 { "pass" } else { "FAIL" }
    );
    // A held-out grid, measured, no criterion (physics review 6003366749 of PR #32, note 2): the proposal's own copies
    // set the rate, so its check is in-sample; this reads the proposal on copies it never saw.
    let (mut held, mut out_of) = (0, 0);
    for t in &TARGETS {
        let states: Vec<State<f32>> = copies_at(t, HELD_OUT).into_iter().flatten().collect();
        out_of += states.len();
        held += parallel(&states, |s| {
            march(s, &params(rate, n_min)).end == Some(End::Exhausted)
        })
        .into_iter()
        .filter(|&e| e)
        .count();
    }
    let _ = writeln!(
        out,
        "\nHeld out (each copy a quarter of the way across its cell, never measured): at the proposal {held} of \
         {out_of} copies run out ({:.2}%), a measurement, no criterion.",
        100.0 * held as f64 / out_of.max(1) as f64
    );
    if at == 0 && half > 0 {
        Ok(out)
    } else {
        Err(out)
    }
}

/// The gate, a measurement (R-649).
pub struct StepBudgetGate;

/// A control input: whether the budget, `n_min` steps at rate 0, is meant to run out on Burrau's march.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    description: String,
    n_min: u32,
}

/// A Burrau-like close-encounter state at f32, the control's march: the Pythagorean problem's masses and positions
/// scaled by 1/5, with small momenta, so not Burrau's problem itself; a deep encounter march to bind a budget.
fn burrau() -> State<f32> {
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f32; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    State { m, r, p }
}

impl MeasurementGate for StepBudgetGate {
    fn name(&self) -> &'static str {
        STEP_BUDGET_NAME
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let c = Control::deserialize(fixture).map_err(|e| format!("{name}: {e}"))?;
        let rec = march(&burrau(), &params(0.0, c.n_min));
        let ran_out = rec.end == Some(End::Exhausted);
        let steps = rec.boundaries.last().map_or(0, |b| b.1);
        Ok(MeasurementControl {
            description: c.description.clone(),
            verdict: if ran_out { Verdict::Pass } else { Verdict::Fail },
            markdown: format!(
                "# {STEP_BUDGET_NAME}: {name}\n\n{}\n\n- the Burrau-like state at f32 under a budget of {} steps at rate 0: {:?} after \
                 {steps} steps\n",
                c.description, c.n_min, rec.end
            ),
        })
    }

    fn flags(&self, _root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => propose().map(|s| print!("{s}")).map_err(|s| {
                print!("{s}");
                format!("gate {STEP_BUDGET_NAME}: the proposal's check failed")
            }),
            _ => Err(format!(
                "gate {STEP_BUDGET_NAME}: unrecognised flags `{}`; usage: --propose",
                args.join(" ")
            )),
        }
    }
}
