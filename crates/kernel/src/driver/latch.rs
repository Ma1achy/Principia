//! The monotone latches and their f16 packing (payload §1, the precision rule; payload §2; render_contract Part 5,
//! "Unpack layer"; REQ-PAY-041, REQ-PAY-044, REQ-PAY-045, REQ-PAY-050, REQ-INT-045).
//!
//! **What they are** (payload §2, "Drift latches are ABSOLUTE maxima"): `d_min = min_t |separation|`,
//! `dE_max = max_t |ΔE(t)|` and `dLz_max = max_t |ΔL_z(t)|`, each a monotone latch over the whole trajectory, `t = 0`
//! included for `d_min`. [`Latches`] holds the three as local f32s and packs them to the payload's f16 halves only when
//! persistent state is written ([`Latches::persist`]): payload §1's semantics (a), one requantisation per persist
//! boundary. At a resume the latches reload from the payload ([`Latches::reload`]) and go on latching, so a sample
//! resumed across any number of persist boundaries stores the f16 of the one f32 latch over its whole run: f16
//! rounding and the clamp are monotone and leave an f16 value where it is, so latching the reloaded f16 value of a
//! segment's latch and then rounding gives what rounding the segment's own latch would have.
//!
//! **`d_min` is latched as its square.** The collision detector forms each pair's position-quantised `d²` once per
//! step and compares `d²`, never `d` (`crate::detect::collision`'s note), and its `dmin_pair` latch holds the least
//! `d²` met. [`Latches`] takes that `d²` ([`Latches::separation_squared`]) and forms `d_min = √d²` once, at
//! persist: `√` is monotone, so the least `√` is the `√` of the least `d²`. A reloaded `d_min` is squared back exactly:
//! an f16 value has at most 11 significant bits, so its square is an f32 value, and its `√` is it again.
//!
//! **What is packed** (payload §1, "Pack requires clamp/canonicalise first"; R-271): each value is clamped to
//! binary16's finite range, ±65504, before the generated packers convert it ([`crate::payload::set_d_min`],
//! [`crate::payload::pack_packed_b`]), so every half of a valid state is finite. A `sim_failed` or `decode_failed`
//! sample, and a reserved state code, which a decoder reads as untrusted ([`crate::payload::sd_is_failed`]), holds the
//! defined failed-state values whatever its latches held: 0.0 in `dE_max` and `dLz_max`, and `d_min` its unset value,
//! f16 `+∞` (bits `0x7C00`), written by [`crate::payload::set_d_min_unset`], never left to the generated struct's
//! `Default`, whose zero `packed_a` would read `d_min` as `0x0000`, a value R-271 says never appears. A valid sample's
//! `d_min` is unset only before its first observation; a start already collided is a collision at `t = 0` whose least
//! separation is observed on the IC and stored, never the unset value (R-933).
//!
//! **Storage never holds NaN** (payload §1; R-79). A NaN `d²` never enters the `d_min` latch (it is not a
//! separation), and a NaN drift maximum, which the march marks `sim_failed` before it persists
//! (`super::wrapper::monitored`), packs as the failed-state 0.0 on a state not so marked, never as f16 NaN.
//!
//! **Control flow reads the live f32** (payload §1, R-669; REQ-INT-045). The latches are for display and the read-time
//! predicates: no branch of the march reads an unpacked f16 latch. The march's decisions read the live measurement (the
//! detector's `d²`, the monitor's drifts), and [`Latches`]' accessors give the live f32 values.

use super::monitor::{latched, magnitude};
use crate::payload::{
    clamp_f16, pa_d_min, pa_d_min_is_unset, pack_packed_b, pb_dE_max, pb_dLz_max, sd_is_failed,
    set_d_min, set_d_min_unset, DminCounters,
};
use crate::real::{IeeeBits, RealOps};

/// The three monotone latches as local f32s (the module's note): the least `d²` met, with whether any was met, and the
/// two drift maxima.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Latches {
    /// The least separation squared met so far; meaningful only where `seen`.
    d2_min: f32,
    /// Whether a separation has been met: `d_min` is unset before it.
    seen: bool,
    /// `max|ΔE|` so far.
    de_max: f32,
    /// `max|ΔL_z|` so far.
    dlz_max: f32,
}

impl Default for Latches {
    /// [`Latches::unset`].
    #[inline]
    fn default() -> Self {
        Self::unset()
    }
}

impl Latches {
    /// The latches of a sample before its first observation: `d_min` unset (the minimum of an empty set, R-271), both
    /// drift maxima 0.
    #[inline]
    pub fn unset() -> Self {
        Latches {
            d2_min: 0.0,
            seen: false,
            de_max: 0.0,
            dlz_max: 0.0,
        }
    }

    /// The latches a resumed sample continues from, reloaded from its persisted `packed_a` and `packed_b`: `d_min`
    /// unset where its bits are the unset value (tested by the bits, R-271), else its f16 value, squared back exactly
    /// (the module's note); `dE_max` and `dLz_max` their f16 values. A failed sample is finished and never resumed
    /// (payload §6, `sd_is_finished`).
    #[inline]
    pub fn reload(packed_a: u32, packed_b: u32) -> Self {
        let seen = !pa_d_min_is_unset(packed_a);
        let d = pa_d_min(packed_a);
        Latches {
            d2_min: if seen { d * d } else { 0.0 },
            seen,
            de_max: pb_dE_max(packed_b),
            dlz_max: pb_dLz_max(packed_b),
        }
    }

    /// Latches a separation squared `d2`, the detector's position-quantised `d²`: the least met so far is replaced
    /// where `d2` is strictly below it or none was met yet. A NaN `d2` is not a separation and is not latched (tested
    /// on the bits; gpu_determinism_note rule 4).
    #[inline]
    pub fn separation_squared(&mut self, d2: f32) {
        if !d2.nan_bits() && (!self.seen || d2 < self.d2_min) {
            self.d2_min = d2;
            self.seen = true;
        }
    }

    /// Latches the drifts `de = ΔE` and `dlz = ΔL_z`, signed: each maximum raised by the magnitude, a non-finite one
    /// kept from then on as the monitor keeps it ([`latched`]).
    #[inline]
    pub fn drift(&mut self, de: f32, dlz: f32) {
        self.de_max = latched(self.de_max, magnitude(de));
        self.dlz_max = latched(self.dlz_max, magnitude(dlz));
    }

    /// Whether `d_min` has been met: false before the first observation, when it is unset.
    #[inline]
    pub fn d_min_set(&self) -> bool {
        self.seen
    }

    /// The live `d_min`, `√` of the least `d²` met; `+∞`, the unset value's, before the first (R-271). The `+∞` is
    /// formed from its bits at run time ([`IeeeBits::or_pos_inf`]), never a non-finite constant, which a shader cannot
    /// hold (gpu_determinism_note rule 4).
    #[inline]
    pub fn d_min(&self) -> f32 {
        self.d2_min.square_root().or_pos_inf(!self.seen)
    }

    /// The live `dE_max`.
    #[inline]
    pub fn de_max(&self) -> f32 {
        self.de_max
    }

    /// The live `dLz_max`.
    #[inline]
    pub fn dlz_max(&self) -> f32 {
        self.dlz_max
    }

    /// `packed_a` and `packed_b` written at a persist boundary (the module's note), `packed_a`'s descriptor kept: for a
    /// failed state ([`sd_is_failed`]), `d_min` unset and both drift halves 0.0; otherwise `d_min` clamped to ±65504
    /// and packed by [`set_d_min`] (unset only where none was met), and the drifts clamped and packed by
    /// [`pack_packed_b`], a NaN one as 0.0. `counters` are the frame's `d_min` counters, which [`set_d_min`] counts
    /// into (R-288, R-294).
    #[inline]
    pub fn persist(&self, packed_a: u32, counters: &DminCounters) -> (u32, u32) {
        if sd_is_failed(packed_a) {
            (set_d_min_unset(packed_a), pack_packed_b(0.0, 0.0))
        } else {
            let a = if self.seen {
                set_d_min(packed_a, clamp_f16(self.d_min()), counters)
            } else {
                set_d_min_unset(packed_a)
            };
            (a, pack_packed_b(stored(self.de_max), stored(self.dlz_max)))
        }
    }
}

/// A drift maximum as a valid state stores it: itself, the packer clamping it, or 0.0 where it is NaN (R-79).
#[inline]
fn stored(x: f32) -> f32 {
    if x.nan_bits() {
        0.0
    } else {
        x
    }
}
