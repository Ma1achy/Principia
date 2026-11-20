//! The `pair-table` gate (`cargo xtask gate pair-table`): the design's measurement on every stepper × regularisation
//! pair under each time transformation, the criterion that reads one cell "measured good" or "measured bad", and the
//! comparison of the measured verdicts with the kernel's pair table (`kernel::regularisation::pairs`) and with the
//! table in integrator_contract Part 2b § "Every stepper × regularisation pair runs; the pairs measured not to work
//! carry a warning".
//!
//! **The measurement** ([`measure`]) is the gap sweep's (`docs/incoming/2026-10-21_gap_sweep_integrator/`, design §1
//! and `evidence/heggie_stepper_probe.rs`), taken on each regularisation's own system through the kernel's own steps
//! (`AzStepper`, `HgStepper`, `LhStepper::regularised_step`): the figure-eight, registered once at `t = 0`, marched at
//! a **fixed fictitious step with no step control**, no sync grid and no re-registration, so the stepper alone is
//! measured on the regularised equations of motion.
//!
//! - **The fictitious period.** RK4 of the same regularisation at the fixed step [`PERIOD_SEARCH_STEP`] until the
//!   state's clock reaches the orbit's period; `τ_end` is that fictitious time rounded to the nearest 1/64.
//! - **Order.** The state error at `τ_end`, the largest component difference over the whole regularised state (the
//!   clock among its components), against RK4 at `τ_end/`[`REFERENCE_DIVISIONS`], over the halving ladder
//!   `τ_end/`[`LADDER`]. The fitted order is `log₂(e_first/e_last)/(n − 1)` over the rungs kept. A rung is kept while
//!   its error is finite and at least [`RESOLVED_FACTOR`] times the reference's own self-difference (the reference
//!   against RK4 at twice its step), counted from the coarsest; the fit stops at the first rung that is not, since an
//!   error at the reference's own error measures the reference, not the pair. With fewer than two rungs kept the
//!   order is not resolved ([`Cell::order`] is `None`), and says nothing.
//! - **Energy.** The regularised energy's relative error `|E − E₀|/|E₀|` after every step of a march of
//!   [`ORBITS`]`·τ_end` at `h = τ_end/`[`ENERGY_DIVISIONS`]: its largest value, its last, and the orbit at which it
//!   first was not below the bound.
//!
//! - **The form's order, beside the criterion** ([`Cell::form_order`]). Where a pair's error over a whole orbit is of
//!   order 1 the one-orbit ladder is not in its asymptotic range, and its slope is not the order of the pair's form.
//!   So the order a pair's profile states is read on a second, short ladder: the same state error over the horizon
//!   `τ_end/`[`SHORT_HORIZON`], at `16, 32, 64, 128, 256` steps ([`SHORT_LADDER`]), against RK4 at
//!   [`SHORT_REFERENCE`] steps, a rung kept by the same rule. Where the short ladder does not resolve an order (a
//!   high-order pair's error there is at the reference's), the one-orbit order is read. It enters no verdict.
//! - **Aarseth–Zare's other arms** ([`az_arms`]), printed and deciding nothing: Aarseth–Zare × RK4 with the reference
//!   body held at 256 and 512 steps an orbit, and at 128 with the reference body re-chosen by its rule after every
//!   step. They show what the held-reference verdict is made of.
//!
//! **The criterion** ([`verdict`]), deciding one cell, a pair under one time transformation, and combining nothing
//! across time transformations. A cell is **measured bad** when either holds, and **measured good** otherwise:
//!
//! 1. the largest energy error over the 100 orbits is not below [`ENERGY_BOUND`] (`!(e < bound)`, so an energy that
//!    is not finite reads bad);
//! 2. the fitted order, where resolved, is not at least [`ORDER_FLOOR`]: the error does not fall under halving.
//!
//! The plan is unclear here (it asks for "the bounds that make a pair good or bad" and gives none); chose these
//! because the energy bound is the design's own mark for a broken energy (its probe records the first step at which
//! `|ΔE/E|` is not below `1e-2`), and because a pair's order alone cannot make it bad: Heggie × KDK's measured order,
//! about 1, is the order its form has on a non-separable system, and Euler's is 1 by construction. Applied to the
//! design's numbers it reads Heggie × RK4 good (order 3.99, energy `5e-7` to `3e-6`) and Heggie × KDK bad (order
//! 0.8–1.2, energy `1e78` and beyond). The bounds are written here, before the first run, and are not fitted to it.
//!
//! **The comparison** ([`check`]): the kernel's table must hold one row per regularisation and time transformation,
//! mark every regularised cell with its measured verdict and `none`'s cells not measured (the `none` pairs are
//! separable, each the stepper's own form, and are not measured as pairs), give every measured-bad cell a warning and
//! no other cell one, keep note 4's warning (Heggie × the generalised leapfrog, the design's measurement, that
//! stepper not being built), and agree cell by cell with the contract's table, which must carry each warning's text.

use std::sync::OnceLock;

use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{CapabilityProfile, DebugOccupant, Occupant, Selection, State};
use kernel::regularisation::az::{self, AzStepper};
use kernel::regularisation::heggie::{self, HgRegistered, HgStepper};
use kernel::regularisation::logh::{self, LhRegistered, LhStepper};
use kernel::regularisation::pairs::{Choice, HeggieTime, LogHMode, PairTable, Status, STEPPERS};
use serde::Deserialize;

use crate::az::{figure_eight, orbit, reference_tiny};
use crate::gate::report::Verdict;
use crate::gate::{MeasurementConfig, MeasurementControl, MeasurementGate};

/// The gate's name.
pub const NAME: &str = "pair-table";

/// The contract the table is written in.
pub const CONTRACT: &str = include_str!("../../../docs/contracts/principia_integrator_contract.md");

/// The fixed fictitious step of the RK4 march that finds the fictitious length of one period (the design's `1e-4`).
pub const PERIOD_SEARCH_STEP: f64 = 1e-4;

/// The halving ladder: the order runs' steps are `τ_end` over each (the design's).
pub const LADDER: [f64; 5] = [64.0, 128.0, 256.0, 512.0, 1024.0];

/// The RK4 reference's step is `τ_end` over this (the design's: a sixteenth of the ladder's finest).
pub const REFERENCE_DIVISIONS: f64 = 16384.0;

/// A rung's error is resolved while it is at least this many times the reference's self-difference.
pub const RESOLVED_FACTOR: f64 = 100.0;

/// The energy run's step is `τ_end` over this (the design's).
pub const ENERGY_DIVISIONS: f64 = 128.0;

/// The energy run's length, in orbits (the design's).
pub const ORBITS: f64 = 100.0;

/// The criterion's energy bound: a largest relative energy error over the run that is not below it reads bad.
pub const ENERGY_BOUND: f64 = 1e-2;

/// The criterion's order floor: a resolved fitted order that is not at least it reads bad.
pub const ORDER_FLOOR: f64 = 0.5;

/// The short ladder's horizon is `τ_end` over this.
pub const SHORT_HORIZON: f64 = 64.0;

/// The short ladder: the number of steps over its horizon at each rung.
pub const SHORT_LADDER: [f64; 5] = [16.0, 32.0, 64.0, 128.0, 256.0];

/// The short ladder's RK4 reference takes this many steps over its horizon.
pub const SHORT_REFERENCE: f64 = 4096.0;

/// The most steps the period search may take: twenty times the most any regularisation here needs. A guard against
/// a clock that never reaches the period, not a calibration.
const PERIOD_SEARCH_CAP: usize = 10_000_000;

// ── The measurement ────────────────────────────────────────────────────────────────────────────────────────────

/// One cell's measurement: a stepper under a regularisation and one time transformation.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// The stepper.
    pub stepper: Selection,
    /// The state error at `τ_end` at each rung of [`LADDER`].
    pub errors: [f64; 5],
    /// How many rungs the fit keeps, counted from the coarsest.
    pub kept: usize,
    /// The fitted order over the kept rungs; `None` where fewer than two are kept.
    pub order: Option<f64>,
    /// The state error over the short horizon at each rung of [`SHORT_LADDER`].
    pub short_errors: [f64; 5],
    /// The fitted order over the short ladder's kept rungs; `None` where fewer than two are kept.
    pub short_order: Option<f64>,
    /// The order the pair's composed profile states.
    pub profile_order: u32,
    /// The flow evaluations a step takes, by the pair's composed profile.
    pub profile_evals: u32,
    /// The largest `|E − E₀|/|E₀|` over the energy run; NaN once any step's energy is not a number.
    pub energy_max: f64,
    /// `|E − E₀|/|E₀|` at the energy run's end.
    pub energy_last: f64,
    /// The orbit (fictitious time over `τ_end`) at which the energy error first was not below [`ENERGY_BOUND`].
    pub broke_at: Option<f64>,
    /// The physical time the energy run's clock reached.
    pub t_reached: f64,
}

impl Cell {
    /// The order of the pair's form, as measured: the short ladder's where it resolves one, the one-orbit ladder's
    /// otherwise (the module's note).
    pub fn form_order(&self) -> Option<f64> {
        self.short_order.or(self.order)
    }
}

/// One row's measurement: a regularisation under one time transformation.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The regularisation and its time transformation.
    pub choice: Choice,
    /// The fictitious time at which the search's clock reached the period.
    pub tau_period: f64,
    /// `tau_period` rounded to the nearest 1/64: the order runs' length, and a hundredth of the energy run's.
    pub tau_end: f64,
    /// The reference's self-difference: RK4 at `τ_end/16384` against RK4 at `τ_end/8192`, at `τ_end`.
    pub reference_difference: f64,
    /// The cells, in the table's column order ([`STEPPERS`]).
    pub cells: [Cell; 5],
}

/// A cell's verdict under the criterion.
#[derive(Clone, Debug, PartialEq)]
pub enum Measured {
    /// Measured good.
    Good,
    /// Measured bad, with which of the criterion's two conditions it met.
    Bad(String),
}

impl Measured {
    /// Whether the verdict is measured bad.
    pub fn is_bad(&self) -> bool {
        matches!(self, Measured::Bad(_))
    }
}

/// The criterion on a fitted order (`None` where not resolved) and a largest energy error (the module's note).
pub fn verdict_of(order: Option<f64>, energy_max: f64) -> Measured {
    // `!(x < bound)` and `!(x >= floor)`, never `x >= bound` or `x < floor`: a NaN reads bad in both.
    let energy_broke = !matches!(
        energy_max.partial_cmp(&ENERGY_BOUND),
        Some(std::cmp::Ordering::Less)
    );
    if energy_broke {
        return Measured::Bad(format!(
            "the largest energy error over {ORBITS} orbits, {energy_max:.3e}, is not below {ENERGY_BOUND:e}"
        ));
    }
    match order {
        Some(o)
            if !matches!(
                o.partial_cmp(&ORDER_FLOOR),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ) =>
        {
            Measured::Bad(format!(
                "the fitted order, {o:.2}, is not at least {ORDER_FLOOR}: the error does not fall under halving"
            ))
        }
        _ => Measured::Good,
    }
}

/// The criterion on a measured cell.
pub fn verdict(cell: &Cell) -> Measured {
    verdict_of(cell.order, cell.energy_max)
}

/// The fitted order over `errors`, the rungs kept counted from the coarsest while finite and at least `floor`: how
/// many are kept, and `log₂(e_first/e_last)/(n − 1)` where at least two are.
pub fn fit(errors: &[f64; 5], floor: f64) -> (usize, Option<f64>) {
    let kept = errors
        .iter()
        .take_while(|e| e.is_finite() && **e >= floor)
        .count();
    let order = (kept >= 2).then(|| (errors[0] / errors[kept - 1]).log2() / (kept - 1) as f64);
    (kept, order)
}

/// A regularised system as the measurement reads it: its registered state, one step of each stepper, its regularised
/// energy, its clock, its state as a flat list, and each pair's composed profile.
struct System<'a, Q, P> {
    q: Q,
    p: P,
    step: &'a dyn Fn(Selection, &Q, &P, f64) -> (Q, P),
    energy: &'a dyn Fn(&Q, &P) -> f64,
    clock: &'a dyn Fn(&Q) -> f64,
    flat: &'a dyn Fn(&Q, &P) -> Vec<f64>,
    profile: &'a dyn Fn(Selection) -> CapabilityProfile,
}

/// RK4, the search's and the reference's stepper.
const RK4: Selection = Selection::Production(Occupant::Rk4);

/// The largest component difference of two flat states; NaN where any component of either is.
fn difference(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).fold(0.0, |worst: f64, (x, y)| {
        let d = (x - y).abs();
        if d.is_nan() || worst.is_nan() {
            f64::NAN
        } else {
            worst.max(d)
        }
    })
}

impl<Q: Copy, P: Copy> System<'_, Q, P> {
    /// The state after `n` steps `h` of `stepper` from the registered state.
    fn march(&self, stepper: Selection, h: f64, n: usize) -> (Q, P) {
        let (mut q, mut p) = (self.q, self.p);
        for _ in 0..n {
            (q, p) = (self.step)(stepper, &q, &p, h);
        }
        (q, p)
    }

    /// The fictitious time at which RK4 at [`PERIOD_SEARCH_STEP`] first has the clock at or past `period`.
    fn tau_period(&self, period: f64) -> Result<f64, String> {
        let (mut q, mut p) = (self.q, self.p);
        let mut steps = 0usize;
        while (self.clock)(&q) < period {
            if steps == PERIOD_SEARCH_CAP || !(self.clock)(&q).is_finite() {
                return Err(format!(
                    "the period search's clock stands at {} after {steps} steps",
                    (self.clock)(&q)
                ));
            }
            (q, p) = (self.step)(RK4, &q, &p, PERIOD_SEARCH_STEP);
            steps += 1;
        }
        Ok(steps as f64 * PERIOD_SEARCH_STEP)
    }

    /// The row's measurement over one `period` of the orbit.
    fn measure(&self, choice: Choice, period: f64) -> Result<Row, String> {
        let tau_period = self.tau_period(period)?;
        let tau_end = (tau_period * 64.0).round() / 64.0;
        let flat = |(q, p): (Q, P)| (self.flat)(&q, &p);
        let reference = flat(self.march(
            RK4,
            tau_end / REFERENCE_DIVISIONS,
            REFERENCE_DIVISIONS as usize,
        ));
        let coarser = flat(self.march(
            RK4,
            2.0 * tau_end / REFERENCE_DIVISIONS,
            (REFERENCE_DIVISIONS / 2.0) as usize,
        ));
        let reference_difference = difference(&reference, &coarser);
        let e0 = (self.energy)(&self.q, &self.p);
        let short = tau_end / SHORT_HORIZON;
        let short_reference =
            flat(self.march(RK4, short / SHORT_REFERENCE, SHORT_REFERENCE as usize));
        let short_coarser = flat(self.march(
            RK4,
            2.0 * short / SHORT_REFERENCE,
            (SHORT_REFERENCE / 2.0) as usize,
        ));
        let short_floor = RESOLVED_FACTOR * difference(&short_reference, &short_coarser);
        let cells = STEPPERS.map(|stepper| {
            let short_errors = SHORT_LADDER.map(|n| {
                difference(
                    &flat(self.march(stepper, short / n, n as usize)),
                    &short_reference,
                )
            });
            let (_, short_order) = fit(&short_errors, short_floor);
            let errors = LADDER.map(|n| {
                difference(
                    &flat(self.march(stepper, tau_end / n, n as usize)),
                    &reference,
                )
            });
            let (kept, order) = fit(&errors, RESOLVED_FACTOR * reference_difference);
            let h = tau_end / ENERGY_DIVISIONS;
            let steps = (ORBITS * ENERGY_DIVISIONS) as usize;
            let (mut q, mut p) = (self.q, self.p);
            let (mut energy_max, mut energy_last, mut broke_at) = (0.0f64, 0.0f64, None);
            for k in 0..steps {
                (q, p) = (self.step)(stepper, &q, &p, h);
                energy_last = (((self.energy)(&q, &p) - e0) / e0.abs()).abs();
                energy_max = if energy_last.is_nan() || energy_max.is_nan() {
                    f64::NAN
                } else {
                    energy_max.max(energy_last)
                };
                if broke_at.is_none() && verdict_of(None, energy_last).is_bad() {
                    broke_at = Some((k + 1) as f64 / ENERGY_DIVISIONS);
                }
            }
            let profile = (self.profile)(stepper);
            Cell {
                stepper,
                errors,
                kept,
                order,
                short_errors,
                short_order,
                profile_order: profile.order,
                profile_evals: profile.force_evals,
                energy_max,
                energy_last,
                broke_at,
                t_reached: (self.clock)(&q),
            }
        });
        Ok(Row {
            choice,
            tau_period,
            tau_end,
            reference_difference,
            cells,
        })
    }
}

/// `$call` with the stepper type `$stepper` selects named `S`.
macro_rules! by_stepper {
    ($stepper:expr, $S:ident => $call:expr) => {
        match $stepper {
            Selection::Debug(DebugOccupant::Euler) => {
                type $S = Euler;
                $call
            }
            Selection::Production(Occupant::Kdk) => {
                type $S = Kdk;
                $call
            }
            Selection::Production(Occupant::Yoshida4) => {
                type $S = Yoshida4;
                $call
            }
            Selection::Production(Occupant::Yoshida6) => {
                type $S = Yoshida6;
                $call
            }
            Selection::Production(Occupant::Rk4) => {
                type $S = Rk4;
                $call
            }
        }
    };
}

/// The figure-eight at `t = 0` and its period.
fn fixture() -> Result<(State<f64>, f64), String> {
    let o = orbit()?;
    Ok((figure_eight::<f64>(&o), o.period))
}

/// The design's measurement on the row `choice`, a regularisation under one time transformation: each of the five
/// steppers' order and energy (the module's note). `none` has no regularised system and is refused.
pub fn measure(choice: Choice) -> Result<Row, String> {
    let (s0, period) = fixture()?;
    let tiny = reference_tiny::<f64>();
    match choice {
        Choice::None => Err(
            "`none` is not a regularised pair: each stepper's own form, not measured here"
                .to_owned(),
        ),
        Choice::AarsethZare => {
            let (system, q, p) = az::register(&s0, tiny);
            System {
                q,
                p,
                step: &|s, q, p, h| {
                    by_stepper!(s, S => <S as AzStepper>::regularised_step(&system, q, p, h))
                },
                energy: &|q, p| system.energy(q, p),
                clock: &|q| q.t,
                flat: &|q, p| {
                    [q.u1, q.u2, [q.t, 0.0], p.p1, p.p2]
                        .into_iter()
                        .flatten()
                        .collect()
                },
                profile: &|s| by_stepper!(s, S => <S as AzStepper>::pair_profile()),
            }
            .measure(choice, period)
        }
        Choice::Heggie(time) => {
            let HgRegistered { system, q, p } = heggie::register(&s0, tiny, time);
            System {
                q,
                p,
                step: &|s, q, p, h| {
                    by_stepper!(s, S => <S as HgStepper>::regularised_step(&system, q, p, h))
                },
                energy: &|q, p| system.energy(q, p),
                clock: &|q| q.t,
                flat: &|q, p| {
                    q.q.iter()
                        .chain(&p.p)
                        .flatten()
                        .copied()
                        .chain([q.t])
                        .collect()
                },
                profile: &|s| by_stepper!(s, S => <S as HgStepper>::pair_profile()),
            }
            .measure(choice, period)
        }
        Choice::LogH(mode) => {
            let LhRegistered { system, q, p } = logh::register(&s0, tiny, mode);
            System {
                q,
                p,
                step: &|s, q, p, h| {
                    by_stepper!(s, S => <S as LhStepper>::regularised_step(&system, q, p, h))
                },
                energy: &|q, p| system.energy(q, p),
                clock: &|q| q.t,
                flat: &|q, p| {
                    q.r.iter()
                        .chain(&p.p)
                        .flatten()
                        .copied()
                        .chain([q.t, p.w])
                        .collect()
                },
                profile: &|s| by_stepper!(s, S => <S as LhStepper>::pair_profile(mode)),
            }
            .measure(choice, period)
        }
    }
}

/// One of Aarseth–Zare × RK4's other arms: what it is, the largest energy error over its run, the orbit at which the
/// error first was not below [`ENERGY_BOUND`], and the physical time reached.
#[derive(Clone, Debug, PartialEq)]
pub struct AzArm {
    /// What the arm is.
    pub what: String,
    /// The largest `|E − E₀|/|E₀|`; NaN once any step's energy is not a number.
    pub energy_max: f64,
    /// The orbit (fictitious time over `τ_end`) at which the energy error first was not below the bound.
    pub broke_at: Option<f64>,
    /// The physical time reached.
    pub t_reached: f64,
}

/// Aarseth–Zare × RK4's other arms on the figure-eight over `100·τ_end`, `tau_end` the Aarseth–Zare row's (the
/// module's note): the reference body held at `τ_end/256` and `τ_end/512`, and at `τ_end/128` with the reference
/// body re-chosen by Aarseth–Zare's rule after every step (the state re-registered, which also re-freezes `E`; the
/// physical time is summed across the registrations). They decide no cell.
pub fn az_arms(tau_end: f64) -> Result<Vec<AzArm>, String> {
    let (s0, _) = fixture()?;
    let tiny = reference_tiny::<f64>();
    let arm = |divisions: f64, rechosen: bool| {
        let (mut system, mut q, mut p) = az::register(&s0, tiny);
        let e0 = system.energy(&q, &p);
        let h = tau_end / divisions;
        let (mut energy_max, mut broke_at, mut t) = (0.0f64, None, 0.0f64);
        for k in 0..(ORBITS * divisions) as usize {
            let before = q.t;
            (q, p) = <Rk4 as AzStepper>::regularised_step(&system, &q, &p, h);
            t += q.t - before;
            let e = ((system.energy(&q, &p) - e0) / e0.abs()).abs();
            energy_max = if e.is_nan() || energy_max.is_nan() {
                f64::NAN
            } else {
                energy_max.max(e)
            };
            if broke_at.is_none() && verdict_of(None, e).is_bad() {
                broke_at = Some((k + 1) as f64 / divisions);
            }
            if rechosen {
                (system, q, p) = az::register(&system.image(&q, &p), tiny);
            }
        }
        AzArm {
            what: format!(
                "RK4, the reference body {}, h = τ_end/{divisions}",
                if rechosen {
                    "re-chosen by the rule after every step"
                } else {
                    "held"
                }
            ),
            energy_max,
            broke_at,
            t_reached: t,
        }
    };
    Ok(vec![arm(256.0, false), arm(512.0, false), arm(128.0, true)])
}

/// The regularised rows, in the table's order: every row but `none`.
pub const REGULARISED: [Choice; 5] = [
    Choice::AarsethZare,
    Choice::Heggie(HeggieTime::Eq20),
    Choice::Heggie(HeggieTime::Eq22),
    Choice::LogH(LogHMode::LogH),
    Choice::LogH(LogHMode::Ttl),
];

/// The measurement of every regularised row, taken once a process.
pub fn measured() -> Result<&'static [Row], String> {
    static ROWS: OnceLock<Result<Vec<Row>, String>> = OnceLock::new();
    ROWS.get_or_init(|| REGULARISED.into_iter().map(measure).collect())
        .as_deref()
        .map_err(Clone::clone)
}

// ── The table as shown ─────────────────────────────────────────────────────────────────────────────────────────

/// A cell's mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Not measured.
    NotMeasured,
    /// Measured good.
    Good,
    /// Measured bad.
    Bad,
}

impl Mark {
    /// The mark's words, as the contract's table writes them.
    pub fn words(self) -> &'static str {
        match self {
            Mark::NotMeasured => "not measured",
            Mark::Good => "measured good",
            Mark::Bad => "measured bad",
        }
    }
}

/// A row of a pair table as a reader is shown it: its regularisation and time transformation, each cell's mark, and
/// each cell's warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShownRow {
    /// The regularisation and its time transformation.
    pub choice: Choice,
    /// The marks, in the table's column order.
    pub marks: [Mark; 5],
    /// The warnings, in the table's column order.
    pub warnings: [Option<String>; 5],
}

/// A pair table as a reader is shown it: its rows, and note 4's warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    /// The rows.
    pub rows: Vec<ShownRow>,
    /// The warning of note 4, Heggie × the generalised leapfrog.
    pub note_4: Option<String>,
}

/// The mark and warning of a status.
fn mark(status: Status) -> (Mark, Option<String>) {
    match status {
        Status::NotMeasured => (Mark::NotMeasured, None),
        Status::MeasuredGood => (Mark::Good, None),
        Status::MeasuredBad { warning } => (Mark::Bad, Some(warning.to_owned())),
    }
}

/// `table` as a reader is shown it.
pub fn shown(table: &PairTable) -> Shown {
    Shown {
        rows: Choice::ALL
            .into_iter()
            .map(|choice| {
                let cells = STEPPERS.map(|s| mark(table.cell(s, choice)));
                ShownRow {
                    choice,
                    marks: cells.clone().map(|(m, _)| m),
                    warnings: cells.map(|(_, w)| w),
                }
            })
            .collect(),
        note_4: mark(table.generalised_leapfrog()).1,
    }
}

/// A row's name, as the contract's table writes it.
pub fn label(choice: Choice) -> &'static str {
    match choice {
        Choice::None => "`none`",
        Choice::AarsethZare => "Aarseth–Zare",
        Choice::Heggie(HeggieTime::Eq20) => "Heggie (Eq. 20)",
        Choice::Heggie(HeggieTime::Eq22) => "Heggie (Eq. 22, the default)",
        Choice::LogH(LogHMode::LogH) => "logH (logH mode)",
        Choice::LogH(LogHMode::Ttl) => "logH (TTL mode)",
    }
}

/// A stepper's name, as the contract's table writes it.
pub fn stepper_name(stepper: Selection) -> &'static str {
    match stepper {
        Selection::Debug(DebugOccupant::Euler) => "Euler",
        Selection::Production(Occupant::Kdk) => "KDK",
        Selection::Production(Occupant::Yoshida4) => "Yoshida-4",
        Selection::Production(Occupant::Yoshida6) => "Yoshida-6",
        Selection::Production(Occupant::Rk4) => "RK4",
    }
}

/// The header of the contract's pair table.
const CONTRACT_HEADER: &str =
    "| regularisation (time transformation) \\ stepper | Euler (debug) | KDK | Yoshida-4 | Yoshida-6 | RK4 |";

/// The pair table written in `contract` (the integrator contract's text): each row's regularisation and time
/// transformation and its five marks, in the order written. Refused where the table is not found, a row's name is
/// none of the table's, or a cell holds none of the three marks.
pub fn contract_table(contract: &str) -> Result<Vec<(Choice, [Mark; 5])>, String> {
    let mut lines = contract.lines().skip_while(|l| l.trim() != CONTRACT_HEADER);
    lines
        .next()
        .ok_or("the contract's pair table is not found (its header line)")?;
    let mut rows = Vec::new();
    for line in lines
        .skip(1)
        .take_while(|l| l.trim_start().starts_with('|'))
    {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        let [name, marks @ ..] = cells.as_slice() else {
            return Err(format!("the contract's pair table: an empty row `{line}`"));
        };
        let choice = Choice::ALL
            .into_iter()
            .find(|c| label(*c) == *name)
            .ok_or(format!(
                "the contract's pair table: no row is named `{name}`"
            ))?;
        let marks: Vec<Mark> = marks
            .iter()
            .map(|cell| {
                [Mark::NotMeasured, Mark::Good, Mark::Bad]
                    .into_iter()
                    .find(|m| cell.contains(m.words()))
                    .ok_or(format!(
                        "the contract's pair table: `{cell}` in row `{name}` is none of the three marks"
                    ))
            })
            .collect::<Result<_, _>>()?;
        let marks: [Mark; 5] = marks.try_into().map_err(|_| {
            format!("the contract's pair table: row `{name}` does not have five cells")
        })?;
        rows.push((choice, marks));
    }
    Ok(rows)
}

/// `text` with each run of white space one space, so a sentence wrapped over lines reads as written on one.
fn unwrapped(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every way `table` fails the gate against the measurement `rows` and the contract's text (the module's note, "The
/// comparison"); empty where it passes.
pub fn check(table: &Shown, rows: &[Row], contract: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let have: Vec<Choice> = table.rows.iter().map(|r| r.choice).collect();
    if have != Choice::ALL {
        let missing: Vec<&str> = Choice::ALL
            .into_iter()
            .filter(|c| !have.contains(c))
            .map(label)
            .collect();
        findings.push(format!(
            "the table has {} rows, not one per regularisation and time transformation (6): missing {}",
            have.len(),
            if missing.is_empty() {
                "none, but out of order or repeated".to_owned()
            } else {
                missing.join(", ")
            }
        ));
    }
    for row in &table.rows {
        let measured = rows.iter().find(|r| r.choice == row.choice);
        for (j, stepper) in STEPPERS.into_iter().enumerate() {
            let name = format!("{} × {}", label(row.choice), stepper_name(stepper));
            let want = match measured {
                Some(m) if verdict(&m.cells[j]).is_bad() => Mark::Bad,
                Some(_) => Mark::Good,
                None => Mark::NotMeasured,
            };
            if row.marks[j] != want {
                findings.push(format!(
                    "{name}: the table says {}, the measurement {} (a status to change in the table and the \
                     contract)",
                    row.marks[j].words(),
                    want.words()
                ));
            }
            match (&row.warnings[j], row.marks[j]) {
                (Some(w), Mark::Bad) if !w.trim().is_empty() => {}
                (_, Mark::Bad) => {
                    findings.push(format!("{name}: measured bad with no warning text"))
                }
                (Some(_), _) => {
                    findings.push(format!("{name}: carries a warning and is not measured bad"))
                }
                (None, _) => {}
            }
        }
    }
    if !table
        .note_4
        .as_deref()
        .is_some_and(|w| !w.trim().is_empty())
    {
        findings.push(
            "note 4's warning, Heggie × the generalised leapfrog (the design's measurement), is not in the table"
                .to_owned(),
        );
    }
    let text = unwrapped(contract);
    match contract_table(contract) {
        Err(e) => findings.push(e),
        Ok(written) => {
            let shown: Vec<(Choice, [Mark; 5])> =
                table.rows.iter().map(|r| (r.choice, r.marks)).collect();
            if written != shown {
                for (choice, marks) in &written {
                    let Some((_, ours)) = shown.iter().find(|(c, _)| c == choice) else {
                        findings.push(format!(
                            "the contract's table has a row {} the table does not",
                            label(*choice)
                        ));
                        continue;
                    };
                    for (j, stepper) in STEPPERS.into_iter().enumerate() {
                        if marks[j] != ours[j] {
                            findings.push(format!(
                                "{} × {}: the contract's table says {}, the table {}",
                                label(*choice),
                                stepper_name(stepper),
                                marks[j].words(),
                                ours[j].words()
                            ));
                        }
                    }
                }
                if written.len() != shown.len() {
                    findings.push(format!(
                        "the contract's table has {} rows, the table {}",
                        written.len(),
                        shown.len()
                    ));
                }
            }
        }
    }
    for warning in table
        .rows
        .iter()
        .flat_map(|r| r.warnings.iter().flatten())
        .chain(&table.note_4)
        .filter(|w| !w.trim().is_empty())
    {
        if !text.contains(&unwrapped(warning)) {
            findings.push(format!(
                "the contract does not carry the warning \"{warning}\""
            ));
        }
    }
    findings.dedup();
    findings
}

// ── The report ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The measurement's report: each row's fictitious period and reference, and each cell's rung errors, fitted order,
/// energy, margin to the bound and verdict.
pub fn report(rows: &[Row]) -> String {
    let mut out = format!(
        "The figure-eight, registered once at t = 0, at a fixed fictitious step with no step control. Order: the \
         state error at τ_end against RK4 at τ_end/{REFERENCE_DIVISIONS}, over τ_end/64 … τ_end/1024, a rung kept \
         while its error is at least {RESOLVED_FACTOR} × the reference's self-difference. Energy: the largest \
         |ΔE/E₀| of the regularised energy over {ORBITS} orbits at h = τ_end/{ENERGY_DIVISIONS}. The criterion, \
         one cell: bad when the largest energy error is not below {ENERGY_BOUND:e}, or the fitted order, where \
         resolved, is not at least {ORDER_FLOOR}.\n"
    );
    for row in rows {
        out.push_str(&format!(
            "\n## {}\n\nτ for one period {:.4}, τ_end {}; the reference's self-difference {:.3e}\n\n\
             | stepper | error at τ_end/64, /128, /256, /512, /1024 | rungs kept | order | short ladder's order | \
             profile's order, evaluations | max \\|ΔE/E₀\\| | last | bound over max | first not below the bound \
             (orbit) | t reached | verdict |\n|---|---|---|---|---|---|---|---|---|---|---|---|\n",
            label(row.choice),
            row.tau_period,
            row.tau_end,
            row.reference_difference
        ));
        for cell in &row.cells {
            let errors: Vec<String> = cell.errors.iter().map(|e| format!("{e:.3e}")).collect();
            let v = verdict(cell);
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {}, {} | {:.3e} | {:.3e} | {:.3e} | {} | {} | {} |\n",
                stepper_name(cell.stepper),
                errors.join(", "),
                cell.kept,
                cell.order
                    .map_or("not resolved".to_owned(), |o| format!("{o:.2}")),
                cell.short_order
                    .map_or("not resolved".to_owned(), |o| format!("{o:.2}")),
                cell.profile_order,
                cell.profile_evals,
                cell.energy_max,
                cell.energy_last,
                ENERGY_BOUND / cell.energy_max,
                cell.broke_at
                    .map_or("never".to_owned(), |o| format!("{o:.2}")),
                time(cell.t_reached),
                match &v {
                    Measured::Good => "**measured good**".to_owned(),
                    Measured::Bad(why) => format!("**measured bad**: {why}"),
                }
            ));
        }
        if row.choice == Choice::AarsethZare {
            out.push_str(
                "\nAarseth–Zare × RK4's other arms, over 100·τ_end; they decide no cell:\n\n| arm | max \\|ΔE/E₀\\| | \
                 first not below the bound (orbit) | t reached |\n|---|---|---|---|\n",
            );
            match az_arms(row.tau_end) {
                Ok(arms) => {
                    for arm in arms {
                        out.push_str(&format!(
                            "| {} | {:.3e} | {} | {} |\n",
                            arm.what,
                            arm.energy_max,
                            arm.broke_at
                                .map_or("never".to_owned(), |o| format!("{o:.2}")),
                            time(arm.t_reached)
                        ));
                    }
                }
                Err(e) => out.push_str(&format!("not measured: {e}\n")),
            }
        }
    }
    out
}

/// A physical time as the report prints it: to two places, or in exponent form where it is not an orbit's scale.
fn time(t: f64) -> String {
    if t.abs() < 1e6 {
        format!("{t:.2}")
    } else {
        format!("{t:.3e}")
    }
}

// ── The gate ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A `pair-table` input: which table is judged.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    description: String,
    /// `kernel`, the kernel's own table, or a control's: `heggie-kdk-good`, `empty-warning`, `no-note-4`,
    /// `one-heggie-row`, `one-logh-row`.
    table: String,
}

/// The table an input names: the kernel's, or the kernel's with a control's change.
pub fn input_table(which: &str) -> Result<Shown, String> {
    let kernel = PairTable::part_2b();
    let kdk = Selection::Production(Occupant::Kdk);
    let mut table = shown(&kernel);
    match which {
        "kernel" => {}
        "heggie-kdk-good" => {
            table = shown(&kernel.with(
                kdk,
                Choice::Heggie(HeggieTime::Eq22),
                Status::MeasuredGood,
            ));
        }
        "empty-warning" => {
            table = shown(&kernel.with(
                kdk,
                Choice::Heggie(HeggieTime::Eq22),
                Status::MeasuredBad { warning: "" },
            ));
        }
        "no-note-4" => table.note_4 = None,
        "one-heggie-row" => table
            .rows
            .retain(|r| r.choice != Choice::Heggie(HeggieTime::Eq20)),
        "one-logh-row" => table
            .rows
            .retain(|r| r.choice != Choice::LogH(LogHMode::Ttl)),
        other => {
            return Err(format!(
                "no table `{other}` (kernel, heggie-kdk-good, empty-warning, no-note-4, one-heggie-row, \
                 one-logh-row)"
            ))
        }
    }
    Ok(table)
}

/// The `pair-table` gate: a measurement, each of whose inputs is a table judged against it. The kernel's own table
/// must pass; each control's must fail.
pub struct PairTableGate;

impl MeasurementGate for PairTableGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let input = Input::deserialize(fixture).map_err(|e| format!("{name}: {e}"))?;
        let table = input_table(&input.table).map_err(|e| format!("{name}: {e}"))?;
        let rows = measured()?;
        let findings = check(&table, rows, CONTRACT);
        let mut markdown = format!("# pair-table: {name}\n\n{}\n\n", input.description);
        if findings.is_empty() {
            markdown.push_str(
                "The table holds the measured verdicts and is the contract's, cell by cell.\n\n",
            );
        } else {
            for f in &findings {
                markdown.push_str(&format!("- {f}\n"));
            }
            markdown.push('\n');
        }
        if input.table == "kernel" {
            markdown.push_str(&report(rows));
            println!("{markdown}");
        }
        Ok(MeasurementControl {
            description: input.description,
            verdict: if findings.is_empty() {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            markdown,
        })
    }

    /// `--measure`: prints the measurement's report, asserting nothing.
    fn flags(&self, _root: &std::path::Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--measure"] => {
                println!("{}", report(measured()?));
                Ok(())
            }
            _ => Err(format!(
                "gate {NAME}: no flags `{}` (--measure)",
                args.join(" ")
            )),
        }
    }
}
