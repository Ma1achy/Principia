//! The `DEGENERATE(reason)` label (dd_decoder §2, "From totality"): a decode-time terminal label with a narrow,
//! enumerated cause set. An invalid decode is tagged, never dropped. This holds the members written so far, the mass
//! block's and `η_E`'s (dd_decoder §3.7); TASK-M2-24 writes the full closed set.

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
}
