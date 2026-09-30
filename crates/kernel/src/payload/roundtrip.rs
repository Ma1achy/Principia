//! The descriptor parity check, `roundtrip_ctl` (pitfalls §9; dd_generation_root §5 test 2): pack → unpack → repack,
//! comparing whole raw `packed_a` words, never the unpacked fields. A check that compares unpacked fields cannot see a
//! contaminated bit the unpack masks off, so it passes on a wrong answer (pitfalls §9); comparing raw words, it can
//! fail on a contaminated value in any bit, the reserved bits 10–15 included.

use super::{
    pa_d_min, pack_packed_a, sd_detail, sd_dmin_pair, sd_last_symbol, sd_saturated, sd_state,
};

/// `packed_a`'s fields, as the generated accessors unpack them (payload §2): the `sample_descriptor` in bits 0–9 and
/// `d_min` in bits 16–31.
#[derive(Clone, Copy, Debug)]
pub struct PackedA {
    pub state: u32,
    pub detail: u32,
    pub saturated: bool,
    pub dmin_pair: u32,
    pub last_symbol: u32,
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
            d_min: pa_d_min(w),
        }
    }

    /// The raw word, by the generated packer (reserved bits zero; `d_min` under R-271).
    pub fn pack(&self) -> u32 {
        pack_packed_a(
            self.state,
            self.detail,
            self.saturated,
            self.dmin_pair,
            self.last_symbol,
            self.d_min,
        )
    }
}

/// Whether `observed` is `expected`, bit for bit: `expected` packed, `observed` unpacked and repacked, and both raw
/// words compared — `observed` against the packing of `expected`, and the repacking against `observed`. A bit of
/// `observed` that no accessor reads is lost by the repacking, so it fails the second comparison even where the fields
/// agree (pitfalls §9).
pub fn roundtrip_ctl(expected: &PackedA, observed: u32) -> bool {
    let packed = expected.pack();
    let repacked = PackedA::unpack(observed).pack();
    observed == packed && repacked == observed
}
