//! The pair table of integrator_contract Part 2b § "Every stepper × regularisation pair runs; the pairs measured not to
//! work carry a warning (R-561)": one cell per stepper × regularisation × time transformation, each measured good,
//! measured bad or not measured, a measured-bad cell carrying its warning.
//!
//! **No pair is forbidden** (R-561). [`Pair::select`] takes any stepper with any regularisation and returns the pair;
//! it has no failing case, and a measured-bad pair is selected like any other, with its warning to show. A pair's
//! status is its cell's for the time transformation in force ([`Choice`]), never looked up by the pair alone
//! (REQ-INT-017).
//!
//! [`PairTable::part_2b`] is the contract's table as it stands. A status changes only by a measurement recorded there
//! (TASK-M3-39 measures the table). It is host-side interface data, read by the selector, the status line and the
//! export sidecar, not by a step.

use super::Kind;
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
    /// The TTL mode (Mikkola–Tanikawa).
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

/// The pair table: a [`Status`] per [`Choice`] (row) and stepper (column).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PairTable {
    cells: [[Status; 5]; 6],
}

impl PairTable {
    /// A table with no pair measured.
    #[inline]
    pub const fn not_measured() -> Self {
        PairTable {
            cells: [[Status::NotMeasured; 5]; 6],
        }
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

    /// Part 2b's table as the contract gives it: Aarseth–Zare × RK4 and Heggie × RK4 (both time transformations)
    /// measured good, Heggie × KDK (both) measured bad with [`HEGGIE_KDK_WARNING`], every other cell not measured.
    pub const fn part_2b() -> Self {
        let kdk = Selection::Production(Occupant::Kdk);
        let rk4 = Selection::Production(Occupant::Rk4);
        let bad = Status::MeasuredBad {
            warning: HEGGIE_KDK_WARNING,
        };
        Self::not_measured()
            .with(rk4, Choice::AarsethZare, Status::MeasuredGood)
            .with(kdk, Choice::Heggie(HeggieTime::Eq20), bad)
            .with(rk4, Choice::Heggie(HeggieTime::Eq20), Status::MeasuredGood)
            .with(kdk, Choice::Heggie(HeggieTime::Eq22), bad)
            .with(rk4, Choice::Heggie(HeggieTime::Eq22), Status::MeasuredGood)
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

impl Pair {
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
