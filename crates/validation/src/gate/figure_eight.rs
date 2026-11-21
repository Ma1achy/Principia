//! The figure-eight closure gate, `figure-eight-closure` (dd_validation_orbits §0, §1.2, §3; TASK-M3-28): the
//! Chenciner–Montgomery figure-eight (`T = 6.32591398`, `E = −1.2871419918`, `L_z = 0`, `Σp = 0`) integrated one period by
//! a regularised occupant across an `η` ladder, its closure `|dr|` reported per rung with the fitted convergence order,
//! which must not fall below the occupant's stated order. The order check is what caught §0's first-order error: energy
//! held to `10⁻¹²` while the orbit missed closure by `6·10⁻³`, a pure phase error no energy diagnostic sees.
//!
//! **The start** is the figure-eight as a pixel decodes it: Simó's digits relabelled so the middle body is in the inner
//! pair ([`tiers::figure_eight`]), through Path A's CHECK 1, `D(E(x))`, refused where the encode clamps. Its period in
//! the sim's units is the published `T` times the start's time scale ([`tiers::time_scale`]).
//!
//! **The march** is the occupant's own direct march at f64 (the comparison harness's [`compare::run`]), its step factor
//! `η` a value, every other step-control value [`StepControl::TEST`]'s, on a sync grid that lands on the period. The plan
//! is unclear here (the sync quantum is one fixed `Δ_sync = 1/8`, and the period is no multiple of it); chose
//! `Δ_sync = T/n`, `n = ⌈T/(1/8)⌉`, because a closure is read at the period, and a grid that stops short of it would read
//! the shortfall as an error.
//!
//! **Two quantities per rung.** `|dr|`, the closure, is `maxᵢ‖rᵢ(T) − rᵢ(0)‖`; it is the requirement's number and the
//! threshold judges it. The order is fitted to the successive differences `d_k = maxᵢ‖rᵢ(T; η_k) − rᵢ(T; η_{k+1})‖`
//! between neighbouring rungs, which fall as `η_k^p` for an occupant of order `p`. The plan is unclear here (it fits the
//! order to `|dr|`); chose the differences because two floors hide the order otherwise, both measured on the first run
//! of this gate: `|dr|` holds the published digits' own non-closure, `5.0·10⁻⁸` (the RK45 reference's own closure), so
//! its slope flattens there; and the error against the RK45 reference flattens at `2.6·10⁻¹¹`, the reference's own
//! error, which the fourth-order occupants pass below already at `η = 0.04`. The differences have no floor but
//! round-off, which measured `1.6·10⁻¹³` to `5.4·10⁻¹³` at the finest rungs of five occupants. The error against the
//! reference is reported beside them, never fitted.
//!
//! **The ladder's step control.** Each rung runs with the predictive step limit off (`f = 0`). The plan is unclear here;
//! chose this because the limit caps the step at coarse `η`, so there the ladder is not a ladder in step size (measured
//! under Heggie × RK4: 1529 steps at every `η` from 1.28 to 0.32 with the limit on, and a local order of 2.8 between
//! 0.16 and 0.08 against 4.0 with it off). The order is the stepper's under its regularisation; the limit is a cap on
//! it. The closure `|dr|` is judged on a separate run at [`ETA_GATE`] under the shipping step control, limit on.
//!
//! **What was fixed when.** Fixed before any measurement: the order tolerance [`ORDER_TOLERANCE`], the threshold's
//! rule, the five criteria's form below, and the closure judged at [`ETA_GATE`]. Chosen after the first run and a probe,
//! with the probe's differences on these rungs already printed (as `az-final-step`'s ladder was chosen knowing its
//! first run): the ladder [`LADDER`], the limit off on it, the fit to the differences and [`DIFFERENCE_FLOOR`]. The
//! first run's criterion, the order fitted to the error against the reference, resolved no fourth-order occupant, as
//! above. `--propose` prints the evidence for each choice: the default occupant's ladder with the limit on, one rung
//! finer than the ladder per occupant (its difference reads the round-off floor), and whether the gate's run at
//! [`ETA_GATE`] is the ladder's last rung bit for bit (where it is, the limit binds no step there, and the order read
//! with it off is the shipping step control's).
//!
//! **Pass criteria:**
//! - every run ends marching on the period's boundary (no budget exhaustion, no non-finite state);
//! - a difference is *resolved* when it is at least [`DIFFERENCE_FLOOR`]; at least [`MIN_RESOLVED`] differences are
//!   resolved, so that the fit has a residual and can fail;
//! - the fitted order `p̂`, the least-squares slope of `ln d_k` on `ln η_k` over the resolved differences, is at least the
//!   occupant's stated order (`Arm::profile().order`) less the input's `order_tolerance`;
//! - `|dr|` at the gate's `η` ([`ETA_GATE`], the field backend's), under the shipping step control, is at most the
//!   input's `closure_bound`, itself at most the gate's threshold, which is the ceiling over every occupant. The order
//!   check reads convergence, not convergence to the right orbit: `|dr|` is the gate's one judged tie to the published
//!   orbit, so it is bound per occupant, not only by the loosest occupant's ceiling.
//!
//! Reported beside them, never judged: each halving's local order `log₂(d_k/d_{k+1})`, the error against the reference,
//! the steps per rung and their ratio per halving, and the reference's own closure and uncertainty. The steps do not
//! double per halving at coarse `η`: each sync interval's landing retakes add a count that does not scale with `η`
//! (with the landing off, the steps are `⌈1/η⌉` per interval and their ratio is about 2).
//!
//! **The proposal** (`--propose`, [`propose`]) measures every input's ladder and proposes, by a rule fixed before the
//! run: the order tolerance [`ORDER_TOLERANCE`], the same for every occupant and not read from the data; each
//! occupant's closure bound, one decade above its own `|dr|` at [`ETA_GATE`], rounded up to a decade; and the threshold,
//! the same rule on the worst `|dr|` over the occupants whose order passes. An occupant whose order falls short is
//! reported with a one-variable measurement of where its error comes from (the sync boundary count varied at fixed
//! `η`), and is an expected failure only with that measurement beside it.

use std::fmt::Write as _;
use std::path::Path;

use serde::Deserialize;

use kernel::generated::links::Defaults;
use kernel::occupant::{DebugOccupant, Occupant, Selection, State};
use kernel::regularisation::az::End;
use kernel::regularisation::pairs::{Choice, HeggieTime, LogHMode};
use kernel::regularisation::sync::delta_sync;

use crate::compare::{self, Arm, Discretisation, StepControl};
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::ground_truth::{path_a, tiers};
use crate::import::unit_state;
use crate::pair_table::{label, stepper_name};

/// The gate's name.
pub const NAME: &str = "figure-eight-closure";

/// The `η` ladder, coarse to fine, halving, ending at the gate's `η`.
pub const LADDER: [f64; 6] = [0.32, 0.16, 0.08, 0.04, 0.02, 0.01];

/// The gate's `η`, at which `|dr|` is judged: the field backend's (`engine::field::heggie_params`).
pub const ETA_GATE: f64 = 0.01;

/// The least difference between neighbouring rungs the fit reads, in the canonical gauge's length (`I = 1`, so every
/// `‖rᵢ‖ < 1`): about twenty times the round-off floor the differences reach at the finest rungs, `5.4·10⁻¹³` at worst
/// over five occupants (the module's note).
pub const DIFFERENCE_FLOOR: f64 = 1e-11;

/// The fewest resolved differences a fit is read from: three, so the fit has a residual.
pub const MIN_RESOLVED: usize = 3;

/// The order tolerance proposed for every occupant, fixed before the run: half an order, so a fourth-order occupant
/// passes at `p̂ ≥ 3.5` and §0's first-order error, and a third-order one, fail.
pub const ORDER_TOLERANCE: f64 = 0.5;

/// An occupant named in an input: a stepper under a regularisation, with the landing on or off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Named {
    /// The stepper.
    pub stepper: Selection,
    /// The regularisation, with its time transformation.
    pub regularisation: Choice,
    /// Whether the landing is on: the clamp, the secant's retakes and the bisection's halvings (§0.1's fix and the
    /// rule in force). Off, none of them, so each interval's last step overshoots its boundary by up to a step (§0's
    /// error): only in a control.
    pub landing: bool,
}

impl Named {
    /// The occupant's name, as the pair table writes it.
    pub fn describe(&self) -> String {
        let clamp = if self.landing { "" } else { ", landing off" };
        format!(
            "{} × {}{clamp}",
            label(self.regularisation),
            stepper_name(self.stepper)
        )
    }

    /// Its arm at `eta` on `d`: [`StepControl::TEST`] with `η` replaced, and the landing turned off where it is off.
    pub fn arm(&self, eta: f64, d: Discretisation) -> Arm {
        let on = StepControl::TEST;
        Arm {
            stepper: self.stepper,
            regularisation: self.regularisation,
            step_control: StepControl {
                eta,
                clamp: self.landing && on.clamp,
                secant_tries: if self.landing { on.secant_tries } else { 0 },
                bisect_tries: if self.landing { on.bisect_tries } else { 0 },
                ..on
            },
            discretisation: d,
        }
    }

    /// The occupant's stated order: its composed capability profile's.
    pub fn order(&self) -> u32 {
        self.arm(
            ETA_GATE,
            Discretisation {
                delta_sync: 1.0,
                horizon: 1.0,
            },
        )
        .profile()
        .order
    }
}

/// The stepper an input names: `euler`, `kdk`, `yoshida4`, `yoshida6` or `rk4`.
pub fn stepper(name: &str) -> Result<Selection, String> {
    Ok(match name {
        "euler" => Selection::Debug(DebugOccupant::Euler),
        "kdk" => Selection::Production(Occupant::Kdk),
        "yoshida4" => Selection::Production(Occupant::Yoshida4),
        "yoshida6" => Selection::Production(Occupant::Yoshida6),
        "rk4" => Selection::Production(Occupant::Rk4),
        other => {
            return Err(format!(
                "refused: no stepper `{other}` (euler, kdk, yoshida4, yoshida6, rk4)"
            ))
        }
    })
}

/// The regularisation an input names: `heggie-eq22`, `heggie-eq20`, `aarseth-zare`, `logh` or `ttl`. `none` is refused:
/// its step control is the wrapper's substep law, which takes no `η`.
pub fn regularisation(name: &str) -> Result<Choice, String> {
    Ok(match name {
        "heggie-eq22" => Choice::Heggie(HeggieTime::Eq22),
        "heggie-eq20" => Choice::Heggie(HeggieTime::Eq20),
        "aarseth-zare" => Choice::AarsethZare,
        "logh" => Choice::LogH(LogHMode::LogH),
        "ttl" => Choice::LogH(LogHMode::Ttl),
        other => {
            return Err(format!(
                "refused: no regularisation `{other}` (heggie-eq22, heggie-eq20, aarseth-zare, logh, ttl); `none` has \
                 no η ladder"
            ))
        }
    })
}

/// An input: the occupant and its order tolerance.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The stepper's name ([`stepper`]).
    pub stepper: String,
    /// The regularisation's name ([`regularisation`]).
    pub regularisation: String,
    /// Whether the landing is on ([`Named::landing`]); on unless the input says otherwise.
    #[serde(default = "on")]
    pub landing: bool,
    /// The order tolerance (REQ-VAL-133's calibration).
    pub order_tolerance: f64,
    /// The closure bound at [`ETA_GATE`] (REQ-VAL-133's calibration), at most the gate's threshold.
    pub closure_bound: f64,
    /// Whether the input is a control, which must fail; the proposal measures the others.
    #[serde(default)]
    pub control: bool,
}

fn on() -> bool {
    true
}

impl Input {
    /// The occupant it names.
    pub fn named(&self) -> Result<Named, String> {
        Ok(Named {
            stepper: stepper(&self.stepper)?,
            regularisation: regularisation(&self.regularisation)?,
            landing: self.landing,
        })
    }
}

/// The figure-eight's start, as a pixel decodes it, and its period in the sim's units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Start {
    /// `D(E(x))`.
    pub state: State<f64>,
    /// The period.
    pub period: f64,
}

/// The start through Path A's CHECK 1 (the module's note); refused where the encode clamps.
pub fn start() -> Result<Start, String> {
    let s = tiers::figure_eight();
    let x = unit_state(&s)?;
    let c = path_a::check1::<Defaults>(&x);
    if c.clamped {
        return Err("refused: the figure-eight's encode clamped".to_owned());
    }
    Ok(Start {
        state: State {
            m: c.decoded.m,
            r: c.decoded.r,
            p: c.decoded.p,
        },
        period: tiers::figure_eight_period() * tiers::time_scale(&s, c.lambda),
    })
}

/// The sync grid that lands on `period` with `n` boundaries: `Δ_sync = T/n`, the horizon `n·Δ_sync`. `None` where the
/// grid's boundary count is not `n`.
pub fn grid(period: f64, n: u32) -> Option<Discretisation> {
    let delta_sync = period / f64::from(n);
    let d = Discretisation {
        delta_sync,
        horizon: f64::from(n) * delta_sync,
    };
    (d.boundaries() == Some(n)).then_some(d)
}

/// The default boundary count on `period`: `⌈T/Δ_sync⌉`, the kernel's `Δ_sync`.
pub fn boundaries(period: f64) -> u32 {
    (period / delta_sync::<f64>()).ceil() as u32
}

/// The largest position distance of the three bodies, `maxᵢ‖aᵢ − bᵢ‖`.
pub fn dr(a: &State<f64>, b: &State<f64>) -> f64 {
    (0..3)
        .map(|i| (a.r[i][0] - b.r[i][0]).hypot(a.r[i][1] - b.r[i][1]))
        .fold(0.0, f64::max)
}

/// One run on the period's grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rung {
    /// `η`.
    pub eta: f64,
    /// The state it ended at.
    pub state: State<f64>,
    /// The closure `|dr|`.
    pub closure: f64,
    /// The error at the period against the reference, reported.
    pub error: f64,
    /// The steps taken.
    pub steps: u32,
    /// How the march ended.
    pub end: End,
    /// The boundary it ended on.
    pub boundary: u32,
}

/// The reference at the period.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Truth {
    /// Its state, where it reached the period.
    pub state: Option<State<f64>>,
    /// Its uncertainty there.
    pub uncertainty: f64,
    /// Its own closure `|dr|`: the published digits' non-closure, which no occupant removes.
    pub closure: f64,
}

/// The reference on the start's grid `d`.
pub fn truth(start: &Start, d: &Discretisation) -> Truth {
    let k = d.boundaries().expect("the grid has boundaries");
    let r = compare::reference(&start.state, d, k);
    Truth {
        state: r.state,
        uncertainty: r.uncertainty,
        closure: r.state.map_or(f64::NAN, |s| dr(&s, &start.state)),
    }
}

/// The run of `arm` from `start`, against `truth`.
pub fn rung(arm: &Arm, start: &Start, truth: &Truth) -> Rung {
    let t = compare::run(arm, &start.state);
    Rung {
        eta: arm.step_control.eta,
        state: t.state,
        closure: dr(&t.state, &start.state),
        error: truth.state.map_or(f64::NAN, |s| dr(&t.state, &s)),
        steps: t.steps,
        end: t.end,
        boundary: t.boundary,
    }
}

/// The least-squares slope of `y` on `x`; NaN for fewer than two points or no spread in `x`.
pub fn slope(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;
    if points.len() < 2 {
        return f64::NAN;
    }
    let (mx, my) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), &(x, y)| (a + x / n, b + y / n));
    let (sxy, sxx) = points.iter().fold((0.0, 0.0), |(a, b), &(x, y)| {
        (a + (x - mx) * (y - my), b + (x - mx) * (x - mx))
    });
    if sxx > 0.0 {
        sxy / sxx
    } else {
        f64::NAN
    }
}

/// The successive differences of a ladder, `(η_k, d_k)` with `d_k = maxᵢ‖rᵢ(T; η_k) − rᵢ(T; η_{k+1})‖`.
pub fn differences(rungs: &[Rung]) -> Vec<(f64, f64)> {
    rungs
        .windows(2)
        .map(|w| (w[0].eta, dr(&w[0].state, &w[1].state)))
        .collect()
}

/// Whether a difference is resolved: finite and at least [`DIFFERENCE_FLOOR`].
pub fn is_resolved(d: f64) -> bool {
    d.is_finite() && d >= DIFFERENCE_FLOOR
}

/// The resolved differences of `differences`.
pub fn resolved(differences: &[(f64, f64)]) -> Vec<(f64, f64)> {
    differences
        .iter()
        .copied()
        .filter(|&(_, d)| is_resolved(d))
        .collect()
}

/// The fitted order over `differences`: the slope of `ln d` on `ln η`.
pub fn order(differences: &[(f64, f64)]) -> f64 {
    slope(
        &differences
            .iter()
            .map(|&(eta, d)| (eta.ln(), d.ln()))
            .collect::<Vec<_>>(),
    )
}

/// Each halving's local order, `log₂(d_k/d_{k+1})`.
pub fn local_orders(differences: &[(f64, f64)]) -> Vec<f64> {
    differences
        .windows(2)
        .map(|w| (w[0].1 / w[1].1).log2())
        .collect()
}

/// One occupant's ladder, measured.
#[derive(Clone, Debug, PartialEq)]
pub struct Ladder {
    /// The occupant.
    pub named: Named,
    /// Its stated order.
    pub stated: u32,
    /// The period.
    pub period: f64,
    /// The grid.
    pub grid: Discretisation,
    /// The reference.
    pub truth: Truth,
    /// The rungs, coarse to fine, the predictive limit off.
    pub rungs: Vec<Rung>,
    /// The run at [`ETA_GATE`] under the shipping step control, which the threshold judges.
    pub gate: Rung,
}

impl Ladder {
    /// The successive differences.
    pub fn differences(&self) -> Vec<(f64, f64)> {
        differences(&self.rungs)
    }

    /// The differences the fit reads.
    pub fn resolved(&self) -> Vec<(f64, f64)> {
        resolved(&self.differences())
    }

    /// The fitted order over the resolved differences.
    pub fn order(&self) -> f64 {
        order(&self.resolved())
    }

    /// Why a run cannot be read, one line each; empty when every one can.
    pub fn faults(&self) -> Vec<String> {
        let k = self.grid.boundaries().unwrap_or(0);
        self.rungs
            .iter()
            .map(|r| (r, "the ladder's rung"))
            .chain(std::iter::once((&self.gate, "the gate's run")))
            .filter(|(r, _)| !(r.end == End::Marching && r.boundary == k))
            .map(|(r, what)| {
                format!(
                    "{what} at η = {} ended {:?} on boundary {} of {k}: undetermined",
                    r.eta, r.end, r.boundary
                )
            })
            .collect()
    }

    /// The ladder's table and its fit, as markdown lines.
    pub fn table(&self) -> Vec<String> {
        let mut out = vec![
            format!(
                "{}: stated order {}; period {:.9} on {} boundaries of Δ_sync {:.9}; the RK45 reference's uncertainty \
                 {:.3e}, its own closure {:.3e} (the published digits')",
                self.named.describe(),
                self.stated,
                self.period,
                self.grid.boundaries().unwrap_or(0),
                self.grid.delta_sync,
                self.truth.uncertainty,
                self.truth.closure
            ),
            "| η | closure |dr| | error against the reference | steps | steps ratio | difference to the next | local \
             order | resolved |"
                .to_owned(),
            "|---|---|---|---|---|---|---|---|".to_owned(),
        ];
        let diffs = self.differences();
        let local = local_orders(&diffs);
        for (i, r) in self.rungs.iter().enumerate() {
            let ratio = i
                .checked_sub(1)
                .map(|j| format!("{:.3}", f64::from(r.steps) / f64::from(self.rungs[j].steps)))
                .unwrap_or_else(|| "–".to_owned());
            let (d, res) = diffs
                .get(i)
                .map(|&(_, d)| {
                    (
                        format!("{d:.3e}"),
                        if is_resolved(d) { "yes" } else { "no" },
                    )
                })
                .unwrap_or_else(|| ("–".to_owned(), "–"));
            let lo = i
                .checked_sub(1)
                .and_then(|j| local.get(j))
                .map(|l| format!("{l:.3}"))
                .unwrap_or_else(|| "–".to_owned());
            out.push(format!(
                "| {} | {:.3e} | {:.3e} | {} | {ratio} | {d} | {lo} | {res} |",
                r.eta, r.closure, r.error, r.steps
            ));
        }
        let res = self.resolved();
        out.push(format!(
            "fitted order over the {} resolved difference(s): {:.3}; the gate's run at η = {ETA_GATE}, the predictive \
             limit on: closure |dr| {:.3e}, error against the reference {:.3e}, {} steps",
            res.len(),
            order(&res),
            self.gate.closure,
            self.gate.error,
            self.gate.steps
        ));
        out
    }
}

/// `named`'s arm for the ladder: its arm at `eta` on `d`, the predictive limit off (the module's note).
pub fn ladder_arm(named: &Named, eta: f64, d: Discretisation) -> Arm {
    let mut arm = named.arm(eta, d);
    arm.step_control.predictive_f = 0.0;
    arm
}

/// `named`'s ladder on [`LADDER`] from `start`, on the default grid.
pub fn ladder(named: &Named, start: &Start) -> Result<Ladder, String> {
    ladder_on(named, start, boundaries(start.period), &LADDER)
}

/// `named`'s ladder on `etas` from `start` on a grid of `n` boundaries, and its gate run.
pub fn ladder_on(named: &Named, start: &Start, n: u32, etas: &[f64]) -> Result<Ladder, String> {
    let grid = grid(start.period, n).ok_or_else(|| {
        format!(
            "refused: a grid of {n} boundaries does not land on the period {}",
            start.period
        )
    })?;
    let truth = truth(start, &grid);
    let rungs = etas
        .iter()
        .map(|&eta| rung(&ladder_arm(named, eta, grid), start, &truth))
        .collect();
    Ok(Ladder {
        named: *named,
        stated: named.order(),
        period: start.period,
        grid,
        truth,
        rungs,
        gate: rung(&named.arm(ETA_GATE, grid), start, &truth),
    })
}

/// The gate's judgement of `ladder` under the closure bound `threshold` and the order tolerance `tolerance`: the
/// verdict, and the negative-results lines.
pub fn judge(ladder: &Ladder, threshold: f64, tolerance: f64) -> (Verdict, Vec<String>) {
    let faults = ladder.faults();
    let res = ladder.resolved();
    let p = order(&res);
    let floor = f64::from(ladder.stated) - tolerance;
    let enough = res.len() >= MIN_RESOLVED;
    let order_ok = enough && p >= floor;
    let closure = ladder.gate.closure;
    let closure_ok = closure <= threshold;
    let mut lines = vec![
        if enough {
            format!(
                "{} difference(s) resolved (at least {MIN_RESOLVED}): each at least {DIFFERENCE_FLOOR:e}",
                res.len()
            )
        } else {
            format!(
                "only {} difference(s) resolved (at least {DIFFERENCE_FLOOR:e}), fewer than {MIN_RESOLVED}: the order is \
                 undetermined",
                res.len()
            )
        },
        if order_ok {
            format!(
                "the fitted order {p:.3} is at least the stated order {} less the tolerance {tolerance} ({floor})",
                ladder.stated
            )
        } else {
            format!(
                "the fitted order {p:.3} is below the stated order {} less the tolerance {tolerance} ({floor})",
                ladder.stated
            )
        },
        if closure_ok {
            format!("the closure |dr| {closure:.3e} at η = {ETA_GATE} is within the bound {threshold:e}")
        } else {
            format!(
                "the closure |dr| {closure:.3e} at η = {ETA_GATE} exceeds the bound {threshold:e}"
            )
        },
    ];
    let ok = faults.is_empty() && order_ok && closure_ok;
    lines.extend(faults);
    (if ok { Verdict::Pass } else { Verdict::Fail }, lines)
}

/// The gate.
pub struct FigureEightClosureGate;

impl Gate for FigureEightClosureGate {
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
        if !(input.order_tolerance.is_finite() && input.order_tolerance >= 0.0) {
            return Err(format!(
                "refused: the order tolerance {} is not finite and non-negative",
                input.order_tolerance
            ));
        }
        let ceiling = config.threshold.value;
        if !(input.closure_bound > 0.0 && input.closure_bound <= ceiling) {
            return Err(format!(
                "refused: the closure bound {} is not positive and at most the gate's threshold {ceiling:e}",
                input.closure_bound
            ));
        }
        let named = input.named()?;
        let start = start()?;
        let ladder = ladder(&named, &start)?;
        let (verdict, negative) = judge(&ladder, input.closure_bound, input.order_tolerance);
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description,
            verdict,
            threshold: config.threshold.clone(),
            scatter: ladder.table(),
            regions: Some(Regions {
                count: RegionCount::Recorded(ladder.resolved().len() as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--propose"] => propose(root).map(|s| print!("{s}")),
            _ => Err(format!(
                "gate {NAME}: unrecognised flags `{}`; usage: --propose",
                args.join(" ")
            )),
        }
    }
}

/// The occupants the proposal measures: every input of the gate's `gate.json` under `root` that is expected to pass or
/// to fail as a finding, its controls (`"control": true`) left out.
fn occupants(root: &Path) -> Result<Vec<(String, Named)>, String> {
    let dir = root.join("fixtures/gates").join(NAME);
    let config = crate::gate::config(root, NAME)?;
    let mut out = Vec::new();
    for file in &config.inputs {
        let path = dir.join(file);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(o) = v.as_object_mut() {
            o.remove("expected");
        }
        let input: Input =
            serde_json::from_value(v).map_err(|e| format!("{}: {e}", path.display()))?;
        if !input.control {
            out.push((file.clone(), input.named()?));
        }
    }
    Ok(out)
}

/// The threshold the proposal's rule gives from the worst passing `|dr|`: one decade above it, rounded up to a decade.
pub fn proposed_threshold(worst: f64) -> f64 {
    10f64.powf(worst.log10().ceil() + 1.0)
}

/// REQ-VAL-133's proposal (the module's note): every occupant's ladder, its fit, its closure bound, one rung finer and
/// the gate's run against the ladder's last rung; the default occupant's ladder with the predictive limit on and off;
/// the threshold and the tolerance; for an occupant whose order falls short, its ladder again with the sync boundary
/// count halved and doubled, one variable.
pub fn propose(root: &Path) -> Result<String, String> {
    let start = start()?;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# REQ-VAL-133: the figure-eight closure gate's threshold and order tolerance, proposed ({NAME} --propose)\n"
    );
    let _ = writeln!(
        out,
        "The rule, fixed before the run: the order tolerance is {ORDER_TOLERANCE} for every occupant, not read from the \
         data; each occupant's closure bound is one decade above its own closure |dr| at η = {ETA_GATE}, rounded up to \
         a decade; the threshold, the ceiling over them, is the same rule on the worst |dr| over the occupants whose \
         order passes. The order is fitted to the successive differences between rungs at \
         least {DIFFERENCE_FLOOR:e}, at least {MIN_RESOLVED} of them, the predictive limit off on the ladder.\n"
    );
    let mut worst = 0.0f64;
    let mut short = Vec::new();
    for (file, named) in occupants(root)? {
        let l = ladder(&named, &start)?;
        let _ = writeln!(out, "## {file}\n");
        for line in l.table() {
            let _ = writeln!(out, "{line}");
        }
        let (_, lines) = judge(&l, f64::INFINITY, ORDER_TOLERANCE);
        for line in lines {
            let _ = writeln!(out, "- {line}");
        }
        let last = l.rungs.last().expect("a ladder has rungs");
        let same = last.state == l.gate.state && last.steps == l.gate.steps;
        let finer = rung(
            &ladder_arm(&named, last.eta / 2.0, l.grid),
            &start,
            &l.truth,
        );
        let _ = writeln!(
            out,
            "- the gate's run at η = {ETA_GATE}, the limit on, is {} the ladder's last rung, the limit off{}",
            if same { "bit for bit" } else { "not" },
            if same {
                ": the limit binds no step there"
            } else {
                ": the limit binds there"
            }
        );
        let _ = writeln!(
            out,
            "- one rung finer, η = {}: its difference from η = {} is {:.3e}, {:.3}× the floor {DIFFERENCE_FLOOR:e} \
             (below it, the ladder's last difference is at round-off)",
            finer.eta,
            last.eta,
            dr(&finer.state, &last.state),
            dr(&finer.state, &last.state) / DIFFERENCE_FLOOR
        );
        let res = l.resolved();
        let passes = res.len() >= MIN_RESOLVED
            && l.faults().is_empty()
            && l.order() >= f64::from(l.stated) - ORDER_TOLERANCE;
        let _ = writeln!(
            out,
            "- proposed closure bound: {:e}",
            proposed_threshold(l.gate.closure)
        );
        if passes {
            worst = worst.max(l.gate.closure);
        } else {
            short.push((file, named));
        }
        let _ = writeln!(out);
    }
    // The evidence for the ladder's limit off: the default occupant's ladder with the limit on, coarser rungs added.
    let default = Named {
        stepper: Selection::Production(Occupant::Rk4),
        regularisation: Choice::Heggie(HeggieTime::Eq22),
        landing: true,
    };
    let n = boundaries(start.period);
    let grid = grid(start.period, n).ok_or("refused: the default grid")?;
    let truth = truth(&start, &grid);
    let etas = [1.28, 0.64, 0.32, 0.16, 0.08, 0.04, 0.02, 0.01];
    let _ = writeln!(
        out,
        "## Why the ladder runs with the predictive limit off: {}, the limit on and off\n",
        default.describe()
    );
    let _ = writeln!(
        out,
        "| η | steps, limit on | steps, limit off | difference to the next, on | difference, off | local order, on | \
         local order, off |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|");
    let on: Vec<Rung> = etas
        .iter()
        .map(|&e| rung(&default.arm(e, grid), &start, &truth))
        .collect();
    let off: Vec<Rung> = etas
        .iter()
        .map(|&e| rung(&ladder_arm(&default, e, grid), &start, &truth))
        .collect();
    let (d_on, d_off) = (differences(&on), differences(&off));
    let (l_on, l_off) = (local_orders(&d_on), local_orders(&d_off));
    for k in 0..etas.len() {
        let d = |v: &[(f64, f64)]| v.get(k).map_or("–".to_owned(), |x| format!("{:.3e}", x.1));
        let lo = |v: &[f64]| {
            k.checked_sub(1)
                .and_then(|j| v.get(j))
                .map_or("–".to_owned(), |x| format!("{x:.3}"))
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} |",
            etas[k],
            on[k].steps,
            off[k].steps,
            d(&d_on),
            d(&d_off),
            lo(&l_on),
            lo(&l_off)
        );
    }
    let _ = writeln!(out);
    for (file, named) in short {
        let _ = writeln!(
            out,
            "## {file}: the order falls short; the sync boundary count varied at η = {ETA_GATE}, one variable, the shipping step control\n"
        );
        let _ = writeln!(
            out,
            "| boundaries | fitted order | resolved differences | local orders | closure |dr| at η = {ETA_GATE} |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|");
        let n = boundaries(start.period);
        for m in [n / 2, n, 2 * n] {
            match ladder_on(&named, &start, m.max(1), &LADDER) {
                Ok(l) => {
                    let local: Vec<String> = local_orders(&l.differences())
                        .iter()
                        .map(|x| format!("{x:.2}"))
                        .collect();
                    let _ = writeln!(
                        out,
                        "| {m} | {:.3} | {} | {} | {:.3e} |",
                        l.order(),
                        l.resolved().len(),
                        local.join(", "),
                        l.gate.closure
                    );
                }
                Err(e) => {
                    let _ = writeln!(out, "| {m} | {e} | | | |");
                }
            }
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(
        out,
        "**Proposed:** the order tolerance {ORDER_TOLERANCE} for every occupant; each occupant's closure bound as \
         listed; the threshold, their ceiling, {:e} (the worst passing |dr| at η = {ETA_GATE} is {worst:.3e}).",
        proposed_threshold(worst)
    );
    Ok(out)
}
