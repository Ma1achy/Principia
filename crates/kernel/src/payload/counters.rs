//! The `d_min` packer's telemetry counters (R-281, R-288; telemetry §2): `dmin_nan_unset`, the packs that stored a NaN
//! as the unset value, and `dmin_negative_floored`, the packs that clamped a negative value to the floor. Both are
//! atomic u32 counts, incremented in release builds as well as debug ones. They belong to the frame, never to global
//! state: the packer's caller passes in a frame's pair and reads it back, and the kernel holds no mutable static
//! (R-294). The GPU's per-frame buffer and its readback with the telemetry readback are TASK-M5-28's (R-294).

use core::sync::atomic::{AtomicU32, Ordering};

/// A frame's `d_min` counter pair (R-288, R-294). The packer increments one with a relaxed `fetch_add` per NaN or
/// negative input it stores; the caller that owns the frame passes it to every pack and reads it back after.
#[derive(Debug, Default)]
pub struct DminCounters {
    /// The `d_min` packs that stored a NaN as the unset value, `0x7c00` (telemetry §2's `dmin_nan_unset`).
    pub dmin_nan_unset: AtomicU32,
    /// The `d_min` packs that clamped a negative value to the floor, `0x0001` (telemetry §2's
    /// `dmin_negative_floored`).
    pub dmin_negative_floored: AtomicU32,
}

impl DminCounters {
    /// A pair at zero, for a new frame.
    pub const fn new() -> Self {
        DminCounters {
            dmin_nan_unset: AtomicU32::new(0),
            dmin_negative_floored: AtomicU32::new(0),
        }
    }

    /// The frame's counts, `(dmin_nan_unset, dmin_negative_floored)`, as its caller reads them back (R-294).
    pub fn read(&self) -> (u32, u32) {
        (
            self.dmin_nan_unset.load(Ordering::Relaxed),
            self.dmin_negative_floored.load(Ordering::Relaxed),
        )
    }
}
