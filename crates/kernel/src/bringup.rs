//! The kernel's one debug mode, colour_composition Appendix A's bring-up mode (R-75): instead of physics, the kernel
//! writes a known pattern, a function of the sample's index alone, into the existing `SimState` slots (no new buffer),
//! for when payload writes are too broken for the fragment presets to run. It is a baked variant, selected by its type
//! and pre-built (lowering Part 3, "Compute side"), never a dispatch-flag bit (R-41): nothing here reads a flags word.
//! It is on the sim key (colour_composition §0; RQ-221), which the engine carries.
//!
//! **The pattern** (colour_composition Appendix A, REQ-TOOL-123), for the sample at buffer index `i`, `j = i mod 2¹⁶`:
//! - each real slot of `SimStateFTLE`, numbered `k` in the ledger's member order — `r` 0–5 (`r[b][c]` is `2b + c`), `p`
//!   6–11, `r_sh` 12–17, `p_sh` 18–23, `S` 24, `theta` 25, `mean_y` 26, `C_ty` 27, `E_0` 28, `Lz_0` 29, `closure_min`
//!   30 — holds `32·i + k`, an integer exact in f32 for `i < 2¹⁹`, so the f32 and the f64 instantiations write the same
//!   values;
//! - `packed_a`, through the generated setters: `state = i mod 6`, `detail = ⌊i/2⌋ mod 4`, `saturated = ⌊i/8⌋ mod 2 =
//!   1`, `dmin_pair = ⌊i/16⌋ mod 4`, `last_symbol = ⌊i/64⌋ mod 4`, `d_min` unset (R-271), the reserved bits 10–15 zero;
//! - `packed_b = 0`: `dE_max` and `dLz_max` both +0;
//! - `times = pack_times(j, 65535 − j)`; `total_substeps = i`; `closure_step = j`, `_reserved = 0`.
//!
//! The word buffer, `ICDescriptor` and `RenderQuad` are not written. [`pattern`] is the native writer, generic over the
//! kernel's `Real`, run at f64 on the CPU; [`write_pattern`] the GPU entry point, at f32, writing the same values as
//! `SimStateFTLE`'s words. Both take each value from the same functions, monomorphised at their `Real`.

use core::mem::{offset_of, size_of};

pub use spirv_std::glam::UVec3;
use spirv_std::spirv;

use crate::payload::{
    pack_times, set_d_min_unset, set_detail, set_dmin_pair, set_last_symbol, set_saturated,
    set_state, PayloadReal, SimStateFTLE, SimStateFTLEOf,
};

/// A baked kernel variant, selected by its type (lowering Part 3, "Compute side"): each is its own pre-built kernel,
/// never a branch on a dispatch-flag bit (R-41).
pub trait Variant {
    /// The variant's name, as the sim key carries it.
    const NAME: &'static str;
    /// Whether it is a debug mode.
    const DEBUG: bool;
}

/// colour_composition Appendix A's bring-up mode, the kernel's one debug mode (R-75).
#[derive(Clone, Copy, Debug)]
pub struct BringUp;

impl Variant for BringUp {
    const NAME: &'static str = "bring_up";
    const DEBUG: bool = true;
}

/// The kernel's baked variants, as `(name, debug)`. The physics kernel is not built yet (M1), so the bring-up mode is
/// the only one; UV, DECODE and ROUNDTRIP are fragment presets, never kernel variants (R-75).
pub const VARIANTS: &[(&str, bool)] = &[(BringUp::NAME, BringUp::DEBUG)];

/// A `Real` the pattern is written at: the payload's, with the exact conversion of a pattern integer.
pub trait PatternReal: PayloadReal {
    /// `n` as this type: exact for every `n < 2²⁴`, as every pattern value is.
    fn from_u32(n: u32) -> Self;
}

impl PatternReal for f32 {
    #[inline]
    fn from_u32(n: u32) -> Self {
        n as f32
    }
}

impl PatternReal for f64 {
    #[inline]
    fn from_u32(n: u32) -> Self {
        n as f64
    }
}

/// The samples the pattern covers: below `2¹⁹`, every real slot's value is exact in f32.
#[inline]
pub fn max_samples() -> u32 {
    1 << 19
}

/// The value of sample `i`'s real slot `k`: `32·i + k`.
#[inline]
pub fn real_slot<R: PatternReal>(i: u32, k: u32) -> R {
    R::from_u32(i * 32 + k)
}

/// Sample `i`'s `packed_a`, through the generated setters over zero, so its reserved bits stay zero.
#[inline]
pub fn packed_a(i: u32) -> u32 {
    let w = set_state(0, i % 6);
    let w = set_detail(w, (i >> 1) & 3);
    let w = set_saturated(w, (i >> 3) & 1 == 1);
    let w = set_dmin_pair(w, (i >> 4) & 3);
    let w = set_last_symbol(w, (i >> 6) & 3);
    set_d_min_unset(w)
}

/// Sample `i`'s `packed_b`: `dE_max` and `dLz_max` both +0.
#[inline]
pub fn packed_b(_i: u32) -> u32 {
    0
}

/// Sample `i`'s `times`: `t_end_step = j`, `t_dmin_step = 65535 − j`, `j = i mod 2¹⁶`.
#[inline]
pub fn times(i: u32) -> u32 {
    let j = i & 0xffff;
    pack_times(j, 0xffff - j)
}

/// Sample `i`'s `total_substeps`: `i`.
#[inline]
pub fn total_substeps(i: u32) -> u32 {
    i
}

/// Sample `i`'s `closure_step`: `i mod 2¹⁶`.
#[inline]
pub fn closure_step(i: u32) -> u32 {
    i & 0xffff
}

/// The three 2-vectors of real slots `k0 .. k0 + 6` of sample `i`, `[b][c]` at `k0 + 2b + c`.
#[inline]
fn vectors<R: PatternReal>(i: u32, k0: u32) -> [[R; 2]; 3] {
    let v = |b: u32| [real_slot(i, k0 + 2 * b), real_slot(i, k0 + 2 * b + 1)];
    [v(0), v(1), v(2)]
}

/// Sample `i`'s `SimStateFTLE` under the bring-up mode, at the `Real` `R`: the native writer.
#[inline]
pub fn pattern<R: PatternReal>(i: u32) -> SimStateFTLEOf<R> {
    SimStateFTLEOf {
        r: vectors(i, 0),
        p: vectors(i, 6),
        r_sh: vectors(i, 12),
        p_sh: vectors(i, 18),
        S: real_slot(i, 24),
        theta: real_slot(i, 25),
        mean_y: real_slot(i, 26),
        C_ty: real_slot(i, 27),
        E_0: real_slot(i, 28),
        Lz_0: real_slot(i, 29),
        packed_a: packed_a(i),
        packed_b: packed_b(i),
        times: times(i),
        total_substeps: total_substeps(i),
        closure_min: real_slot(i, 30),
        closure_step: closure_step(i) as u16,
        _reserved: 0,
        _tail: Default::default(),
    }
}

/// `SimStateFTLE`'s size in words at f32, the GPU's.
#[inline]
pub fn words_per_sample() -> usize {
    size_of::<SimStateFTLE>() / 4
}

/// Writes the f32 value `v` at word `at` of `out`.
#[inline]
fn put(out: &mut [u32], at: usize, v: f32) {
    out[at] = v.to_bits();
}

/// The six real slots `k0 .. k0 + 6` of sample `i` at f32, from word `at` on.
#[inline]
fn put_vectors(out: &mut [u32], at: usize, i: u32, k0: u32) {
    let mut c = 0;
    while c < 6 {
        put(out, at + c as usize, real_slot(i, k0 + c));
        c += 1;
    }
}

/// Sample `i`'s `SimStateFTLE` under the bring-up mode, at f32, as its words in `out`, the `SimState` buffer: each
/// member at its offset in the generated struct, the two u16 members as their one word, `closure_step` low.
#[inline]
pub fn write_words(i: u32, out: &mut [u32]) {
    let base = i as usize * words_per_sample();
    let at = |offset: usize| base + offset / 4;
    put_vectors(out, at(offset_of!(SimStateFTLE, r)), i, 0);
    put_vectors(out, at(offset_of!(SimStateFTLE, p)), i, 6);
    put_vectors(out, at(offset_of!(SimStateFTLE, r_sh)), i, 12);
    put_vectors(out, at(offset_of!(SimStateFTLE, p_sh)), i, 18);
    put(out, at(offset_of!(SimStateFTLE, S)), real_slot(i, 24));
    put(out, at(offset_of!(SimStateFTLE, theta)), real_slot(i, 25));
    put(out, at(offset_of!(SimStateFTLE, mean_y)), real_slot(i, 26));
    put(out, at(offset_of!(SimStateFTLE, C_ty)), real_slot(i, 27));
    put(out, at(offset_of!(SimStateFTLE, E_0)), real_slot(i, 28));
    put(out, at(offset_of!(SimStateFTLE, Lz_0)), real_slot(i, 29));
    out[at(offset_of!(SimStateFTLE, packed_a))] = packed_a(i);
    out[at(offset_of!(SimStateFTLE, packed_b))] = packed_b(i);
    out[at(offset_of!(SimStateFTLE, times))] = times(i);
    out[at(offset_of!(SimStateFTLE, total_substeps))] = total_substeps(i);
    put(
        out,
        at(offset_of!(SimStateFTLE, closure_min)),
        real_slot(i, 30),
    );
    out[at(offset_of!(SimStateFTLE, closure_step))] = closure_step(i);
}

/// The bring-up mode's compute entry point, one invocation per sample, at the workgroup size of the GPU harness (64):
/// sample `i` writes its pattern into `simstate`, the f32 `SimState` buffer as words. Invocations past the buffer's
/// last sample write nothing.
#[spirv(compute(threads(64)))]
pub fn write_pattern(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] simstate: &mut [u32],
) {
    if (id.x as usize) < simstate.len() / words_per_sample() {
        write_words(id.x, simstate);
    }
}
