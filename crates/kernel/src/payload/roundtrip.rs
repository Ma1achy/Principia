//! The descriptor parity check, `roundtrip_ctl` (pitfalls §9; dd_generation_root §5 test 2): pack → unpack → repack,
//! comparing whole raw `packed_a` words, never the unpacked fields. A check that compares unpacked fields cannot see a
//! contaminated bit the unpack masks off, so it passes on a wrong answer (pitfalls §9); comparing raw words, it can
//! fail on a contaminated value in any bit, `I_max`'s bits 10–14 included.

use super::{
    pa_d_min, pack_packed_a, sd_I_max, sd_departed, sd_detail, sd_dmin_pair, sd_last_symbol,
    sd_saturated, sd_state, set_I_max, set_d_min_release, set_departed, set_detail, set_dmin_pair,
    set_last_symbol, set_saturated, set_state, DminCounters,
};

/// `packed_a`'s fields, as the generated accessors unpack them (payload §2): the `sample_descriptor` in bits 0–15, the
/// Chazy latch `I_max`'s level in 10–14 and the departed bit in 15, and `d_min` in bits 16–31.
#[derive(Clone, Copy, Debug)]
pub struct PackedA {
    pub state: u32,
    pub detail: u32,
    pub saturated: bool,
    pub dmin_pair: u32,
    pub last_symbol: u32,
    /// `I_max`'s stored level (`crate::chazy::i_max_code`).
    pub i_max: u32,
    pub departed: bool,
    pub d_min: f32,
}

impl PackedA {
    /// The fields of `w`, each read by its accessor (payload §6).
    pub fn unpack(w: u32) -> Self {
        PackedA {
            state: sd_state(w),
            detail: sd_detail(w),
            saturated: sd_saturated(w),
            dmin_pair: sd_dmin_pair(w),
            last_symbol: sd_last_symbol(w),
            i_max: sd_I_max(w),
            departed: sd_departed(w),
            d_min: pa_d_min(w),
        }
    }

    /// The raw word, by the generated packer (reserved bits zero; `d_min` canonicalised: a NaN or `+∞` as the unset
    /// value, a negative or a value below f16's least subnormal as the floor), counting into `counters`, the caller's
    /// frame pair.
    pub fn pack(&self, counters: &DminCounters) -> u32 {
        pack_packed_a(
            self.state,
            self.detail,
            self.saturated,
            self.dmin_pair,
            self.last_symbol,
            self.i_max,
            self.departed,
            self.d_min,
            counters,
        )
    }

    /// The raw word of fields unpacked from an observed word, written in bit order over zero as
    /// [`pack_packed_a`] writes them, with `d_min` through [`super::set_d_min_release`]. An observed NaN or negative
    /// `d_min` is a contaminated value for the check to report, not a store, so the repack must not trip
    /// [`super::set_d_min`]'s debug assertions. It writes `0x7c00` or `0x0001` there, which differ from the
    /// observed bits, so the check fails. Nor does it count: the repack is an observation, not a store, so it passes
    /// a scratch counter pair and drops it, and no frame's counters are touched.
    fn repack(&self, scratch: &DminCounters) -> u32 {
        let w = set_state(0, self.state);
        let w = set_detail(w, self.detail);
        let w = set_saturated(w, self.saturated);
        let w = set_dmin_pair(w, self.dmin_pair);
        let w = set_last_symbol(w, self.last_symbol);
        let w = set_I_max(w, self.i_max);
        let w = set_departed(w, self.departed);
        set_d_min_release(w, self.d_min, scratch)
    }
}

/// Whether `observed` is `expected`, bit for bit: `expected` packed, `observed` unpacked and repacked, and both raw
/// words compared — `observed` against the packing of `expected`, and the repacking against `observed`. A bit of
/// `observed` that no accessor reads is lost by the repacking, so it fails the second comparison even where the fields
/// agree (pitfalls §9).
///
/// The check is an observation, not a store, so it takes no frame's counters: both packs count into a scratch pair
/// local to the call, which is dropped.
pub fn roundtrip_ctl(expected: &PackedA, observed: u32) -> bool {
    let scratch = DminCounters::new();
    let packed = expected.pack(&scratch);
    let repacked = PackedA::unpack(observed).repack(&scratch);
    observed == packed && repacked == observed
}
