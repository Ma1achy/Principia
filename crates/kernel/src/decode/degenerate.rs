//! The `DEGENERATE(reason)` label (dd_decoder §2, "From totality"; chart_reference §0.7): a decode-time terminal label
//! with a narrow, enumerated cause set. An invalid decode is tagged, never dropped, and every pixel carries a label.
//!
//! **The closed set** (dd_decoder §2, "The `DEGENERATE` cause set"; R-72, REQ-DEC-038): five reasons, each raised by one
//! block, each recorded in payload §2's `sample_descriptor` as `state = decode_failed` with the `detail` code
//! [`Degenerate::detail`] gives:
//!
//! | reason | raised by | `detail` |
//! |---|---|---|
//! | `M01_TINY` | the mass block, `M₀₁ < ε_M01` (dd_decoder §3.1) | 2, invalid mass construction |
//! | `NO_QUALIFYING_SEED` | the invariant construction, no seed with `‖w⁽²⁾‖²_m > ε_w` (§3.4, R-82) | 1, degenerate configuration |
//! | `INVARIANT_INFEASIBLE` | the invariant construction, `K* < L_z²/(2I)` (chart_reference §2.2) | 3, other |
//! | `ENERGY_INFEASIBLE` | `η_E`, `E* < U` (§3.7) | 3, other |
//! | `ENERGY_UNSCALABLE` | `η_E`, the rest start at `E* > U` or the collision pole (§3.7) | 3, other |
//!
//! No reason writes `detail` 0, a non-finite decode output: the decode of a finite input is finite, and that is
//! asserted, not tagged (`kernel/tests/decode_totality.rs`). [`ALL`] lists the set, and [`Degenerate::detail`]'s match
//! is exhaustive, so a reason added to the type fails the build until it has its code.

use crate::payload::STATE_DECODE_FAILED;

/// `decode_failed`'s `detail` 0: a non-finite decode output (payload §2).
pub const DETAIL_NON_FINITE: u32 = 0;
/// `decode_failed`'s `detail` 1: a degenerate configuration (payload §2).
pub const DETAIL_DEGENERATE_CONFIGURATION: u32 = 1;
/// `decode_failed`'s `detail` 2: an invalid mass construction, a mass → 0, the mass-simplex boundary (payload §2).
pub const DETAIL_INVALID_MASS: u32 = 2;
/// `decode_failed`'s `detail` 3: other (payload §2).
pub const DETAIL_OTHER: u32 = 3;

/// Why a decode is `DEGENERATE`. Its discriminant is a `u32`: the kernel's SPIR-V has no 8-bit integers (no
/// `OpCapability Int8`), and with more than one reason the default discriminant would be a `u8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Degenerate {
    /// `M01_TINY`: the inner pair's mass `M₀₁ = m₀ + m₁` is below the mass block's ε (dd_decoder §3.1;
    /// REQ-DEC-011), [`crate::decode::mass::m01_tiny`].
    M01Tiny,
    /// `ENERGY_INFEASIBLE`: under an energy target, `E* < U`, so `η_E` is not real (dd_decoder §3.7; REQ-DEC-041),
    /// [`crate::decode::energy_norm`].
    EnergyInfeasible,
    /// `ENERGY_UNSCALABLE`: under an energy target, no finite `η_E` exists: the rest start `K₀ = 0` at `E* > U`, or the
    /// collision pole `U = −∞` (dd_decoder §3.7; REQ-DEC-041), [`crate::decode::energy_norm`].
    EnergyUnscalable,
    /// `INVARIANT_INFEASIBLE`: on a chart built by the invariant-momentum construction, `K* < L_z²/(2I)`, so no momentum
    /// realises the targets (chart_reference §2.2; REQ-DEC-040), [`crate::decode::invariant::Outcome::Infeasible`].
    InvariantInfeasible,
    /// `NO_QUALIFYING_SEED`: the construction's targets are feasible but no seed has `‖w⁽²⁾‖²_m > ε_w` (dd_decoder
    /// §3.4; R-82), [`crate::decode::invariant::Outcome::Degenerate`].
    NoQualifyingSeed,
}

/// Every reason, the closed set, in the order of the type.
pub const ALL: [Degenerate; 5] = [
    Degenerate::M01Tiny,
    Degenerate::EnergyInfeasible,
    Degenerate::EnergyUnscalable,
    Degenerate::InvariantInfeasible,
    Degenerate::NoQualifyingSeed,
];

impl Degenerate {
    /// The reason's `decode_failed` `detail` code (the module's table).
    #[inline]
    pub fn detail(self) -> u32 {
        match self {
            Degenerate::M01Tiny => DETAIL_INVALID_MASS,
            Degenerate::NoQualifyingSeed => DETAIL_DEGENERATE_CONFIGURATION,
            Degenerate::InvariantInfeasible
            | Degenerate::EnergyInfeasible
            | Degenerate::EnergyUnscalable => DETAIL_OTHER,
        }
    }

    /// The reason as `(state, detail)` of payload §2's `sample_descriptor`: `decode_failed` (5) and [`Self::detail`].
    #[inline]
    pub fn label(self) -> (u32, u32) {
        (STATE_DECODE_FAILED, self.detail())
    }

    /// The reason's name, as dd_decoder §2 writes it.
    pub fn name(self) -> &'static str {
        match self {
            Degenerate::M01Tiny => "M01_TINY",
            Degenerate::EnergyInfeasible => "ENERGY_INFEASIBLE",
            Degenerate::EnergyUnscalable => "ENERGY_UNSCALABLE",
            Degenerate::InvariantInfeasible => "INVARIANT_INFEASIBLE",
            Degenerate::NoQualifyingSeed => "NO_QUALIFYING_SEED",
        }
    }
}
