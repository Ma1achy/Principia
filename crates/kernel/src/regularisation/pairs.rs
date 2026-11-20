//! The pair table of integrator_contract Part 2b § "Every stepper × regularisation pair runs; the pairs measured not to
//! work carry a warning (R-561)": one cell per stepper × regularisation × time transformation, each measured good,
//! measured bad or not measured, a measured-bad cell carrying its warning.
//!
//! **No pair is forbidden** (R-561). [`Pair::select`] takes any stepper with any regularisation and returns the pair;
//! it has no failing case, and a measured-bad pair is selected like any other, with its warning to show. A pair's
//! status is its cell's for the time transformation in force ([`Choice`]), never looked up by the pair alone
//! (REQ-INT-017).
//!
//! [`PairTable::part_2b`] is the contract's table as measured: every regularised cell measured good or measured bad by
//! the contract's criterion (`cargo xtask gate pair-table` takes the measurement and fails when this table or the
//! contract's differs from it), each with its evidence line, a measured-bad cell with its warning; `none`'s cells not
//! measured, being each stepper's own form. A status changes only by that measurement. It is host-side interface
//! data, read by the selector, the status line and the export sidecar, not by a step.

use super::heggie::Heggie;
use super::{Composed, Kind};
use crate::occupant::rk4::Rk4;
use crate::occupant::{DebugOccupant, Occupant, Selection};

/// Heggie's time transformations (Part 2b; R-161).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeggieTime {
    /// Eq. 20, `dτ = dt/(R₁R₂R₃)`.
    Eq20,
    /// Eq. 22 at n = 3/2, the default.
    Eq22,
}

/// logH's time modes (Part 2b; R-162).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogHMode {
    /// The logH mode.
    LogH,
    /// The TTL mode (Mikkola & Aarseth 2002, as the reference set cites it).
    Ttl,
}

/// A regularisation with the time transformation in force: a row of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// `none`.
    None,
    /// Aarseth–Zare.
    AarsethZare,
    /// Heggie, under one of its time transformations.
    Heggie(HeggieTime),
    /// logH, in one of its time modes.
    LogH(LogHMode),
}

impl Choice {
    /// Every row, in the table's order.
    pub const ALL: [Choice; 6] = [
        Choice::None,
        Choice::AarsethZare,
        Choice::Heggie(HeggieTime::Eq20),
        Choice::Heggie(HeggieTime::Eq22),
        Choice::LogH(LogHMode::LogH),
        Choice::LogH(LogHMode::Ttl),
    ];

    /// The regularisation the row is of.
    #[inline]
    pub const fn kind(self) -> Kind {
        match self {
            Choice::None => Kind::None,
            Choice::AarsethZare => Kind::AarsethZare,
            Choice::Heggie(_) => Kind::Heggie,
            Choice::LogH(_) => Kind::LogH,
        }
    }

    /// The row's index in the table.
    #[inline]
    const fn row(self) -> usize {
        match self {
            Choice::None => 0,
            Choice::AarsethZare => 1,
            Choice::Heggie(HeggieTime::Eq20) => 2,
            Choice::Heggie(HeggieTime::Eq22) => 3,
            Choice::LogH(LogHMode::LogH) => 4,
            Choice::LogH(LogHMode::Ttl) => 5,
        }
    }
}

/// Every stepper, in the table's column order: Euler (debug), KDK, Yoshida-4, Yoshida-6, RK4.
pub const STEPPERS: [Selection; 5] = [
    Selection::Debug(DebugOccupant::Euler),
    Selection::Production(Occupant::Kdk),
    Selection::Production(Occupant::Yoshida4),
    Selection::Production(Occupant::Yoshida6),
    Selection::Production(Occupant::Rk4),
];

/// The stepper's column in the table.
#[inline]
const fn column(stepper: Selection) -> usize {
    match stepper {
        Selection::Debug(DebugOccupant::Euler) => 0,
        Selection::Production(Occupant::Kdk) => 1,
        Selection::Production(Occupant::Yoshida4) => 2,
        Selection::Production(Occupant::Yoshida6) => 3,
        Selection::Production(Occupant::Rk4) => 4,
    }
}

/// A cell's status (R-561).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not measured.
    NotMeasured,
    /// Measured to work.
    MeasuredGood,
    /// Measured not to work: the pair still runs, and carries this warning, which states what was measured.
    MeasuredBad {
        /// The warning, as the contract gives it.
        warning: &'static str,
    },
}

impl Status {
    /// The warning a measured-bad cell carries; none for any other.
    #[inline]
    pub const fn warning(self) -> Option<&'static str> {
        match self {
            Status::MeasuredBad { warning } => Some(warning),
            Status::NotMeasured | Status::MeasuredGood => None,
        }
    }
}

/// The warning of Heggie × KDK, under both of Heggie's time transformations (Part 2b's table, note 2).
pub const HEGGIE_KDK_WARNING: &str =
    "Heggie's Hamiltonian is not separable, so a leapfrog step is first order here \
     and can diverge (figure-eight: order about 1, diverged within 100 orbits).";

/// The warning of Heggie × Yoshida-4 and × Yoshida-6, under both of Heggie's time transformations.
pub const HEGGIE_YOSHIDA_WARNING: &str =
    "Heggie's Hamiltonian is not separable, so a Yoshida step, a sequence of leapfrog steps, is first order here \
     and can diverge (figure-eight: order about 1, the energy error past 1e-2 within a tenth of an orbit).";

/// The warning of Aarseth–Zare × KDK, × Yoshida-4 and × Yoshida-6.
pub const AARSETH_ZARE_LEAPFROG_WARNING: &str =
    "Aarseth–Zare's Hamiltonian is not separable, so a leapfrog or Yoshida step is first order here and can diverge \
     (figure-eight, reference body held: order about 1, the energy error past 1e-2 within a quarter of an orbit).";

/// The warning of Aarseth–Zare × RK4: what the fixed-step measurement found, and what it did not measure.
pub const AARSETH_ZARE_RK4_WARNING: &str =
    "With its reference body held and its step fixed, Aarseth–Zare × RK4 loses the figure-eight at 128 steps an \
     orbit (fourth order; the energy error past 1e-2 at orbit 14, not finite by 100). It holds 100 orbits at 512 \
     steps an orbit, and at 128 with the reference body re-chosen after every step; the occupant re-chooses it at \
     every sync boundary and controls its step, which this did not measure.";

/// The warning of Euler under every regularisation and time transformation.
pub const EULER_WARNING: &str =
    "Explicit Euler is first order and its energy error grows without bound (figure-eight, fixed fictitious step: \
     order about 1, the energy error past 1e-2 within a fifth of an orbit).";

/// The evidence line of `none`'s cells: why they are not measured.
pub const NONE_EVIDENCE: &str =
    "separable, so the pair is the stepper's own form; not measured as a pair";

/// The warning of Heggie × the generalised (Störmer–Verlet) leapfrog, the table's note 4: the design's measurement,
/// kept as its record. That stepper is not built and is no column of the table.
pub const GENERALISED_LEAPFROG_WARNING: &str =
    "does not keep Heggie's Eq. (9); the energy breaks once its residual reaches ~2e-2";

/// The pair table: a [`Status`] and an evidence line per [`Choice`] (row) and stepper (column), and the table's note
/// 4, the one pair recorded that is not of the built steppers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PairTable {
    cells: [[Status; 5]; 6],
    evidence: [[&'static str; 5]; 6],
    generalised_leapfrog: Status,
}

impl PairTable {
    /// A table with no pair measured, no evidence and no note 4.
    #[inline]
    pub const fn not_measured() -> Self {
        PairTable {
            cells: [[Status::NotMeasured; 5]; 6],
            evidence: [[""; 5]; 6],
            generalised_leapfrog: Status::NotMeasured,
        }
    }

    /// The table with the cell of `stepper` under `regularisation` set to `status`, with `evidence`, the line that
    /// states what was measured (or why nothing was).
    #[inline]
    pub const fn measured(
        mut self,
        stepper: Selection,
        regularisation: Choice,
        status: Status,
        evidence: &'static str,
    ) -> Self {
        self.cells[regularisation.row()][column(stepper)] = status;
        self.evidence[regularisation.row()][column(stepper)] = evidence;
        self
    }

    /// The evidence line of the cell of `stepper` under `regularisation`: what was measured, or why nothing was.
    #[inline]
    pub const fn evidence(&self, stepper: Selection, regularisation: Choice) -> &'static str {
        self.evidence[regularisation.row()][column(stepper)]
    }

    /// The table with its note 4, Heggie × the generalised leapfrog, set to `status`.
    #[inline]
    pub const fn with_generalised_leapfrog(mut self, status: Status) -> Self {
        self.generalised_leapfrog = status;
        self
    }

    /// The table's note 4: the status of Heggie × the generalised (Störmer–Verlet) leapfrog, a pair of a stepper that
    /// is not built, recorded from the design's measurement.
    #[inline]
    pub const fn generalised_leapfrog(&self) -> Status {
        self.generalised_leapfrog
    }

    /// The table with the cell of `stepper` under `regularisation` set to `status`.
    #[inline]
    pub const fn with(
        mut self,
        stepper: Selection,
        regularisation: Choice,
        status: Status,
    ) -> Self {
        self.cells[regularisation.row()][column(stepper)] = status;
        self
    }

    /// The cell of `stepper` under `regularisation`.
    #[inline]
    pub const fn cell(&self, stepper: Selection, regularisation: Choice) -> Status {
        self.cells[regularisation.row()][column(stepper)]
    }

    /// Part 2b's table as measured (the contract's criterion; `cargo xtask gate pair-table`). On the figure-eight at a
    /// fixed fictitious step: Heggie × RK4 good under both time transformations, the default among them; logH × KDK,
    /// × Yoshida-4, × Yoshida-6 and × RK4 good in both time modes; every other regularised pair bad, with its
    /// warning: Euler everywhere, the explicit leapfrog and Yoshida compositions under Aarseth–Zare and Heggie, and
    /// Aarseth–Zare × RK4 with its reference body held. `none`'s cells are not measured. Each evidence line is that
    /// cell's measured order and energy error.
    pub const fn part_2b() -> Self {
        let euler = Selection::Debug(DebugOccupant::Euler);
        let kdk = Selection::Production(Occupant::Kdk);
        let yoshida4 = Selection::Production(Occupant::Yoshida4);
        let yoshida6 = Selection::Production(Occupant::Yoshida6);
        let rk4 = Selection::Production(Occupant::Rk4);
        let none = Status::NotMeasured;
        Self::not_measured()
            .measured(euler, Choice::None, none, NONE_EVIDENCE)
            .measured(kdk, Choice::None, none, NONE_EVIDENCE)
            .measured(yoshida4, Choice::None, none, NONE_EVIDENCE)
            .measured(yoshida6, Choice::None, none, NONE_EVIDENCE)
            .measured(rk4, Choice::None, none, NONE_EVIDENCE)
            .measured(euler, Choice::AarsethZare, Status::MeasuredBad { warning: EULER_WARNING }, "order 1.37 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.19, not finite by 100")
            .measured(kdk, Choice::AarsethZare, Status::MeasuredBad { warning: AARSETH_ZARE_LEAPFROG_WARNING }, "order 0.95 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.21, not finite by 100")
            .measured(yoshida4, Choice::AarsethZare, Status::MeasuredBad { warning: AARSETH_ZARE_LEAPFROG_WARNING }, "order 1.52 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.12, not finite by 100")
            .measured(yoshida6, Choice::AarsethZare, Status::MeasuredBad { warning: AARSETH_ZARE_LEAPFROG_WARNING }, "order 1.57 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.12, not finite by 100")
            .measured(rk4, Choice::AarsethZare, Status::MeasuredBad { warning: AARSETH_ZARE_RK4_WARNING }, "order 4.13 over one orbit; the energy error past 1e-2 at orbit 14.07, not finite by 100")
            .measured(euler, Choice::Heggie(HeggieTime::Eq20), Status::MeasuredBad { warning: EULER_WARNING }, "order 1.11 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.09, not finite by 100")
            .measured(kdk, Choice::Heggie(HeggieTime::Eq20), Status::MeasuredBad { warning: HEGGIE_KDK_WARNING }, "order 1.21 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.21, not finite by 100")
            .measured(yoshida4, Choice::Heggie(HeggieTime::Eq20), Status::MeasuredBad { warning: HEGGIE_YOSHIDA_WARNING }, "order 0.02 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.04, 7.769e299 at most by 100")
            .measured(yoshida6, Choice::Heggie(HeggieTime::Eq20), Status::MeasuredBad { warning: HEGGIE_YOSHIDA_WARNING }, "order not resolved over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.05, not finite by 100")
            .measured(rk4, Choice::Heggie(HeggieTime::Eq20), Status::MeasuredGood, "order 3.99 over one orbit, 4.00 on the short ladder; the energy error at most 5.026e-7 over 100 orbits")
            .measured(euler, Choice::Heggie(HeggieTime::Eq22), Status::MeasuredBad { warning: EULER_WARNING }, "order 1.14 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.09, not finite by 100")
            .measured(kdk, Choice::Heggie(HeggieTime::Eq22), Status::MeasuredBad { warning: HEGGIE_KDK_WARNING }, "order 0.77 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.21, 7.769e299 at most by 100")
            .measured(yoshida4, Choice::Heggie(HeggieTime::Eq22), Status::MeasuredBad { warning: HEGGIE_YOSHIDA_WARNING }, "order 0.28 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.04, 1.461e157 at most by 100")
            .measured(yoshida6, Choice::Heggie(HeggieTime::Eq22), Status::MeasuredBad { warning: HEGGIE_YOSHIDA_WARNING }, "order 0.31 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.05, 3.583e0 at most by 100")
            .measured(rk4, Choice::Heggie(HeggieTime::Eq22), Status::MeasuredGood, "order 3.99 over one orbit, 4.00 on the short ladder; the energy error at most 2.617e-6 over 100 orbits")
            .measured(euler, Choice::LogH(LogHMode::LogH), Status::MeasuredBad { warning: EULER_WARNING }, "order 0.79 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.02, 1.139e3 at most by 100")
            .measured(kdk, Choice::LogH(LogHMode::LogH), Status::MeasuredGood, "order 2.00 over one orbit, 2.00 on the short ladder; the energy error at most 3.376e-5 over 100 orbits")
            .measured(yoshida4, Choice::LogH(LogHMode::LogH), Status::MeasuredGood, "order 4.05 over one orbit, 4.00 on the short ladder; the energy error at most 5.594e-6 over 100 orbits")
            .measured(yoshida6, Choice::LogH(LogHMode::LogH), Status::MeasuredGood, "order 5.93 over one orbit; the energy error at most 2.228e-9 over 100 orbits")
            .measured(rk4, Choice::LogH(LogHMode::LogH), Status::MeasuredGood, "order 4.14 over one orbit, 4.01 on the short ladder; the energy error at most 3.308e-4 over 100 orbits")
            .measured(euler, Choice::LogH(LogHMode::Ttl), Status::MeasuredBad { warning: EULER_WARNING }, "order 1.16 over one orbit, 1.00 on the short ladder; the energy error past 1e-2 at orbit 0.02, 2.157e0 at most by 100")
            .measured(kdk, Choice::LogH(LogHMode::Ttl), Status::MeasuredGood, "order 2.00 over one orbit, 2.00 on the short ladder; the energy error at most 3.376e-5 over 100 orbits")
            .measured(yoshida4, Choice::LogH(LogHMode::Ttl), Status::MeasuredGood, "order 4.05 over one orbit, 4.00 on the short ladder; the energy error at most 5.594e-6 over 100 orbits")
            .measured(yoshida6, Choice::LogH(LogHMode::Ttl), Status::MeasuredGood, "order 5.93 over one orbit; the energy error at most 2.228e-9 over 100 orbits")
            .measured(rk4, Choice::LogH(LogHMode::Ttl), Status::MeasuredGood, "order 4.39 over one orbit, 4.00 on the short ladder; the energy error at most 6.927e-4 over 100 orbits")
            .with_generalised_leapfrog(Status::MeasuredBad {
                warning: GENERALISED_LEAPFROG_WARNING,
            })
    }
}

/// A selected pair: a stepper and a regularisation with its time transformation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    /// The stepper.
    pub stepper: Selection,
    /// The regularisation, with the time transformation in force.
    pub regularisation: Choice,
}

/// The default occupant as the kernel runs it: RK4 under Heggie with the values of `C`, [`Pair::DEFAULT`]'s type
/// where `C`'s time transformation is Eq. 22. The plan is unclear here (a "default occupant config" with no values
/// to give it); chose to leave the type open in its configuration, because Heggie's step-control values are
/// calibrations with no register entry yet, and a number written here would be one the register does not hold.
pub type DefaultOccupant<C> = Composed<Rk4, Heggie<C>>;

impl Pair {
    /// The default occupant: Heggie × RK4 under Eq. 22 at n = 3/2 ([`super::heggie::DEFAULT_TIME`]), the pair every
    /// Heggie result was measured with, at every tier (Part 2b). Every other pair is selectable ([`Pair::select`]).
    pub const DEFAULT: Pair = Pair::select(
        Selection::Production(Occupant::Rk4),
        Choice::Heggie(super::heggie::DEFAULT_TIME),
    );

    /// Selects `stepper` with `regularisation`. Every pair is selectable, `none` and the measured-bad pairs included:
    /// there is no refusal (R-561).
    #[inline]
    pub const fn select(stepper: Selection, regularisation: Choice) -> Self {
        Pair {
            stepper,
            regularisation,
        }
    }

    /// The pair's status in `table`: its cell under the time transformation in force.
    #[inline]
    pub const fn status(self, table: &PairTable) -> Status {
        table.cell(self.stepper, self.regularisation)
    }

    /// The warning the pair carries in `table`: a measured-bad pair's, and none otherwise.
    #[inline]
    pub const fn warning(self, table: &PairTable) -> Option<&'static str> {
        self.status(table).warning()
    }

    /// Whether the composed occupant owns its time mapping: whenever the regularisation is not `none` (Part 2a; R-19).
    #[inline]
    pub const fn owns_time_mapping(self) -> bool {
        self.regularisation.kind().owns_time_mapping()
    }
}
