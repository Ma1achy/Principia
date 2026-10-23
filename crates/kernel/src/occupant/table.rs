//! The occupant table of integrator_contract Part 2 and Part 2a ("The table gains two rows"): each occupant's profile
//! and default binding, and the tier defaults (REQ-INT-014, REQ-INT-015).
//!
//! | Occupant | Order | Force evals | Symplectic | Step-reversible | Run-reversible | Owns time mapping | Default |
//! |---|---|---|---|---|---|---|---|
//! | Euler | 1 | 1 | ✗ | ✗ | ✗ | ✗ | debug only |
//! | KDK leapfrog | 2 | 1 | ✓ | ✓ | ✗ | ✗ | Potato–Low |
//! | Yoshida-4 | 4 | 3 | ✓ | ✓ | ✗ | ✗ | Medium–High |
//! | Yoshida-6 | 6 | 7 | ✓ | ✓ | ✗ | ✗ | Ultra–Extreme |
//! | RK4 | 4 | 4 | ✗ | ✗ | ✗ | ✗ | validation cross-check |
//! | Aarseth–Zare + RK4 | 4 | 4 | ✗ | ✗ | ✗ | ✓ | close-encounter regimes |
//! | Aarseth–Zare + time-transformed leapfrog | 2 | 1 | ✓ | ✓ (measured) | ✗ | ✓ | allowed future occupant |
//!
//! The five steppers are built here, and each row reads its occupant's own [`Stepper::profile`]. Aarseth–Zare + RK4 is
//! a regularised composition TASK-M3-07 builds (Part 2b); its row is the table's profile until then, and a
//! regularised occupant's `step_reversible` is the one dd_integrator §5 test 3 measures (R-500). Aarseth–Zare +
//! time-transformed leapfrog is an allowed future occupant, not a v1 requirement, and is not built (R-162, R-168).
//!
//! Euler is in no tier's default and in no production list: [`default_occupant`] returns an [`Occupant`], which has
//! no Euler, and Euler's row is bound [`Binding::DebugOnly`] (REQ-INT-015).

use super::euler::Euler;
use super::kdk::Kdk;
use super::rk4::Rk4;
use super::yoshida4::Yoshida4;
use super::yoshida6::Yoshida6;
use super::{CapabilityProfile, DebugOccupant, Occupant, Selection, Stepper};

/// The quality tiers, Potato to Extreme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Potato,
    Low,
    Medium,
    High,
    Ultra,
    Extreme,
}

impl Tier {
    /// Every tier, in order.
    pub const ALL: [Tier; 6] = [
        Tier::Potato,
        Tier::Low,
        Tier::Medium,
        Tier::High,
        Tier::Ultra,
        Tier::Extreme,
    ];
}

/// An occupant's default binding, the table's last column (the user overrides it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// The default of the tiers `from` to `to`, both included.
    Tiers { from: Tier, to: Tier },
    /// The validation cross-check (RK4: fixed, non-symplectic).
    ValidationCrossCheck,
    /// A debug occupant, in no tier's default (Euler).
    DebugOnly,
    /// Close-encounter regimes, a validation-grade stepper (Aarseth–Zare + RK4).
    CloseEncounter,
    /// An allowed future occupant, not a v1 requirement (R-168).
    AllowedFuture,
}

/// Whether a row's occupant is built, and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Built {
    /// A stepper built here, as its selection.
    Stepper(Selection),
    /// A regularised composition, built with the regularisation axis (Part 2b; TASK-M3-07).
    Regularised,
    /// Not built (R-162, R-168).
    NotBuilt,
}

/// One row of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// The occupant's name as the table gives it.
    pub name: &'static str,
    /// Its capability profile.
    pub profile: CapabilityProfile,
    /// Its default binding.
    pub binding: Binding,
    /// Whether it is built.
    pub built: Built,
}

/// The table, Part 2's five rows then Part 2a's two: interface data, the contract's table, not register constants
/// (REQ-SYS-082).
pub fn table() -> [Row; 7] {
    [
        Row {
            name: "Euler",
            profile: Euler::profile(),
            binding: Binding::DebugOnly,
            built: Built::Stepper(Selection::Debug(DebugOccupant::Euler)),
        },
        Row {
            name: "KDK leapfrog",
            profile: Kdk::profile(),
            binding: Binding::Tiers {
                from: Tier::Potato,
                to: Tier::Low,
            },
            built: Built::Stepper(Selection::Production(Occupant::Kdk)),
        },
        Row {
            name: "Yoshida-4",
            profile: Yoshida4::profile(),
            binding: Binding::Tiers {
                from: Tier::Medium,
                to: Tier::High,
            },
            built: Built::Stepper(Selection::Production(Occupant::Yoshida4)),
        },
        Row {
            name: "Yoshida-6",
            profile: Yoshida6::profile(),
            binding: Binding::Tiers {
                from: Tier::Ultra,
                to: Tier::Extreme,
            },
            built: Built::Stepper(Selection::Production(Occupant::Yoshida6)),
        },
        Row {
            name: "RK4",
            profile: Rk4::profile(),
            binding: Binding::ValidationCrossCheck,
            built: Built::Stepper(Selection::Production(Occupant::Rk4)),
        },
        Row {
            name: "Aarseth–Zare + RK4",
            profile: CapabilityProfile {
                order: 4,
                force_evals: 4,
                symplectic: false,
                step_reversible: false,
                run_reversible: false,
                owns_time_mapping: true,
            },
            binding: Binding::CloseEncounter,
            built: Built::Regularised,
        },
        Row {
            name: "Aarseth–Zare + time-transformed leapfrog",
            profile: CapabilityProfile {
                order: 2,
                force_evals: 1,
                symplectic: true,
                step_reversible: true,
                run_reversible: false,
                owns_time_mapping: true,
            },
            binding: Binding::AllowedFuture,
            built: Built::NotBuilt,
        },
    ]
}

/// The stepper a tier binds by default (Part 2's last column): KDK for Potato and Low, Yoshida-4 for Medium and High,
/// Yoshida-6 for Ultra and Extreme. Never Euler, which [`Occupant`] does not hold.
#[inline]
pub const fn default_occupant(tier: Tier) -> Occupant {
    match tier {
        Tier::Potato | Tier::Low => Occupant::Kdk,
        Tier::Medium | Tier::High => Occupant::Yoshida4,
        Tier::Ultra | Tier::Extreme => Occupant::Yoshida6,
    }
}
