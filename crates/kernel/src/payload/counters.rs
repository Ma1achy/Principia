//! The `d_min` packer's telemetry counters (R-281, R-288; telemetry §2): `dmin_nan_unset`, the packs that stored a NaN
//! as the unset value, and `dmin_negative_floored`, the packs that clamped a negative value to the floor. Both are
//! atomic u32 counts, incremented in release builds as well as debug ones. They belong to the frame, never to global
//! state: the packer's caller passes in a frame's pair and reads it back, and the kernel holds no mutable static
//! (R-294). The fields are private and reached only through the methods below; workers share `&DminCounters` (R-300).
//! The GPU's per-frame buffer and its readback with the telemetry readback are TASK-M5-28's (R-294).

use core::sync::atomic::{AtomicU32, Ordering};

/// A frame's `d_min` counter pair (R-288, R-294, R-300). The packer increments one with a relaxed `fetch_add` per NaN
/// or negative input it stores; the caller that owns the frame passes `&DminCounters` to every pack, reads it back
/// after with [`DminCounters::read`], and resets it with [`DminCounters::reset`] before reusing it for a new frame.
#[derive(Debug, Default)]
pub struct DminCounters {
    /// The `d_min` packs that stored a NaN as the unset value, `0x7c00` (telemetry §2's `dmin_nan_unset`).
    dmin_nan_unset: AtomicU32,
    /// The `d_min` packs that clamped a negative value to the floor, `0x0001` (telemetry §2's
    /// `dmin_negative_floored`).
    dmin_negative_floored: AtomicU32,
}

impl DminCounters {
    /// A pair at zero, for a new frame.
    pub const fn new() -> Self {
        DminCounters {
            dmin_nan_unset: AtomicU32::new(0),
            dmin_negative_floored: AtomicU32::new(0),
        }
    }

    /// Counts one `d_min` pack that stored a NaN as the unset value (`dmin_nan_unset`). Takes `&self`, so workers
    /// sharing the frame's `&DminCounters` count into it concurrently (R-300).
    #[inline]
    pub fn increment_nan_unset(&self) {
        self.dmin_nan_unset.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts one `d_min` pack that clamped a negative value to the floor (`dmin_negative_floored`), as
    /// [`DminCounters::increment_nan_unset`] does for its own counter (R-300).
    #[inline]
    pub fn increment_negative_floored(&self) {
        self.dmin_negative_floored.fetch_add(1, Ordering::Relaxed);
    }

    /// The frame's counts, `(dmin_nan_unset, dmin_negative_floored)`, as its caller reads them back (R-294, R-300).
    pub fn read(&self) -> (u32, u32) {
        (
            self.dmin_nan_unset.load(Ordering::Relaxed),
            self.dmin_negative_floored.load(Ordering::Relaxed),
        )
    }

    /// Both counts back to zero, for the frame's owner to reuse the pair for its next frame (R-300). Takes `&mut
    /// self`: a reset needs the owner's exclusive borrow, so it cannot run while a worker still holds the shared
    /// `&DminCounters` and a frame's counts cannot be lost to a reset part-way through it.
    pub fn reset(&mut self) {
        *self.dmin_nan_unset.get_mut() = 0;
        *self.dmin_negative_floored.get_mut() = 0;
    }
}
