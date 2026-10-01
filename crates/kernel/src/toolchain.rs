//! The trivial kernel (canonical_spec §1 item 2; parity_contract §6): the generated pack/unpack of the packed words,
//! run in-kernel over a buffer. The one source is compiled to SPIR-V by rust-gpu and translated to WGSL by naga (`cargo
//! xtask build-kernel`), and compiled natively by rustc; `validation`'s `toolchain_trivial_kernel` runs both and
//! compares the words bit for bit (parity_contract Tier B).
//!
//! It is stateless (pitfalls §10): it certifies the toolchain and the compiled pack/unpack given identical inputs, not
//! parity along a trajectory.

pub use spirv_std::glam::UVec3;
use spirv_std::spirv;

use crate::payload::{
    pack_times, sd_detail, sd_dmin_pair, sd_last_symbol, sd_saturated, sd_state, set_detail,
    set_dmin_pair, set_last_symbol, set_saturated, set_state, tm_t_dmin_step, tm_t_end_step,
};

/// `w` read as `packed_a`: its `sample_descriptor` fields unpacked by the generated accessors and packed again over
/// zero (payload §2, §6), so bits 10–31, the reserved bits and `d_min`, come back zero.
#[inline]
pub fn descriptor_round_trip(w: u32) -> u32 {
    let d = set_state(0, sd_state(w));
    let d = set_detail(d, sd_detail(w));
    let d = set_saturated(d, sd_saturated(w));
    let d = set_dmin_pair(d, sd_dmin_pair(w));
    set_last_symbol(d, sd_last_symbol(w))
}

/// `w` read as `times`: its two exact step indices unpacked and packed again by the generated packer (payload §1, §6).
#[inline]
pub fn times_round_trip(w: u32) -> u32 {
    pack_times(tm_t_end_step(w), tm_t_dmin_step(w))
}

/// The word the kernel writes at index `i` for the input word `w`: an even index reads `w` as `packed_a`, an odd one
/// as `times`, so every buffer exercises both packed words' integer fields.
#[inline]
pub fn pack_unpack_word(i: u32, w: u32) -> u32 {
    if i & 1 == 0 {
        descriptor_round_trip(w)
    } else {
        times_round_trip(w)
    }
}

/// The compute entry point, one invocation per word, at the workgroup size of `validation`'s harness (64):
/// `output[i] = pack_unpack_word(i, input[i])`.
#[spirv(compute(threads(64)))]
pub fn pack_unpack(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] output: &mut [u32],
) {
    let i = id.x as usize;
    if i < input.len() {
        output[i] = pack_unpack_word(id.x, input[i]);
    }
}
