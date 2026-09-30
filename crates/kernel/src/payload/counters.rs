//! The `d_min` packer's telemetry counters (R-281, R-288; telemetry §2): `dmin_nan_unset`, the packs that stored a NaN
//! as the unset value, and `dmin_negative_floored`, the packs that clamped a negative value to the floor. Both are
//! atomic u32 counts, incremented in release builds as well as debug ones. Binding them to the GPU dispatch and reading
//! them back on the profiler/telemetry readback belong to the tasks that build those (R-288).

use core::sync::atomic::AtomicU32;

/// The `d_min` packer's counter pair (R-288). The packer increments one with a relaxed `fetch_add` per NaN or negative
/// input it stores; the per-frame read and reset are the telemetry readback's.
#[derive(Debug, Default)]
pub struct DminCounters {
    /// The `d_min` packs that stored a NaN as the unset value, `0x7c00` (telemetry §2's `dmin_nan_unset`).
    pub dmin_nan_unset: AtomicU32,
    /// The `d_min` packs that clamped a negative value to the floor, `0x0001` (telemetry §2's
    /// `dmin_negative_floored`).
    pub dmin_negative_floored: AtomicU32,
}

impl DminCounters {
    /// A pair at zero.
    pub const fn new() -> Self {
        DminCounters {
            dmin_nan_unset: AtomicU32::new(0),
            dmin_negative_floored: AtomicU32::new(0),
        }
    }
}

/// The crate-level pair that [`super::set_d_min`] counts into, for callers that pass none.
pub static DMIN_COUNTERS: DminCounters = DminCounters::new();
