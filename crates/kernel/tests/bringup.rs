//! The kernel's debug variants (`kernel::bringup`; REQ-TOOL-013; R-41, R-75): exactly one, colour_composition Appendix
//! A's bring-up mode, a baked variant selected by its type, reading no flag bit, whose output decodes through the
//! normal unpack path to Appendix A's pattern (REQ-TOOL-123). Bits 6–7's reservation is checked on `QuadRequest.flags`
//! by `quad_request_flags` (REQ-PAY-064, TASK-M5-04), where the flags word exists (RQ-220).
//! - `debug_variants_are_the_bring_up_mode_alone`: the kernel's variants hold one debug mode, the bring-up mode, and
//!   no fragment preset (UV, DECODE, ROUNDTRIP).
//! - `debug_variants_bring_up_decodes_through_the_unpack`: the native writer at f32 and at f64, each a function of
//!   the sample index alone (a compile check that no flags word reaches it), read through `sim_state_from_ftle` and by
//!   its raw words, is the pattern; written one slot off, it is not (pitfalls §9).
//! - `debug_variants_bring_up_golden`: two samples' fields, as literals from Appendix A.
//! - `debug_variants_gpu_writer`: the GPU entry point's word writer, run natively, writes the native writer's struct at
//!   the ledger's offsets, each sample once; invocations past the buffer write nothing.
//!
//! The GPU build's own readback is `validation`'s `bringup_pattern_spirv`, which needs the built kernel.

use kernel::bringup::{
    max_samples, pattern, words_per_sample, write_pattern, write_words, BringUp, UVec3, Variant,
    VARIANTS,
};
use kernel::payload::{SimStateFTLE, SimStateFTLEOf};
use validation::bringup::{check, check_f64};
use validation::negative_control;
use validation::synthetic::simstate_from_words;

/// The fragment presets, which are never kernel variants (R-75).
const PRESETS: [&str; 3] = ["uv", "decode", "roundtrip"];

/// `variants` holds exactly one debug variant, the bring-up mode, and no fragment preset.
fn check_debug_variants(variants: &[(&str, bool)]) {
    let debug: Vec<&str> = variants.iter().filter(|v| v.1).map(|v| v.0).collect();
    assert_eq!(
        debug,
        [BringUp::NAME],
        "the kernel's debug variants are not exactly the bring-up mode: {debug:?}"
    );
    for (name, _) in variants {
        assert!(
            !PRESETS.contains(&name.to_lowercase().as_str()),
            "the fragment preset `{name}` is a kernel variant (R-75)"
        );
    }
}

#[test]
fn debug_variants_are_the_bring_up_mode_alone() {
    assert_eq!(
        (BringUp::NAME, BringUp::DEBUG),
        ("bring_up", true),
        "the bring-up mode is not the debug variant `bring_up`"
    );
    check_debug_variants(VARIANTS);
}

negative_control!(
    debug_variants_are_the_bring_up_mode_alone,
    "a UV kernel variant beside the bring-up mode must fail",
    expected = "are not exactly the bring-up mode",
    check_debug_variants(&[(BringUp::NAME, true), ("uv", true)])
);

#[cfg(feature = "controls")]
mod preset_control {
    use super::*;

    negative_control!(
        debug_variants_are_the_bring_up_mode_alone,
        "a non-debug DECODE kernel variant must fail",
        expected = "is a kernel variant (R-75)",
        check_debug_variants(&[(BringUp::NAME, true), ("DECODE", false)])
    );
}

/// The native writer at f32: a function of the sample index alone, so no flags word reaches it (R-41). The type is a
/// compile check: a writer taking a flag would not coerce to it.
const WRITER_F32: fn(u32) -> SimStateFTLE = pattern::<f32>;

/// The native writer at f64, as [`WRITER_F32`].
const WRITER_F64: fn(u32) -> SimStateFTLEOf<f64> = pattern::<f64>;

/// The sample indices checked: the first 2048, either side of 2¹⁶ (where `j` wraps), and the last below
/// [`max_samples`].
fn indices() -> Vec<u32> {
    let top = max_samples();
    (0..2048)
        .chain(65_530..65_542)
        .chain(top - 8..top)
        .collect()
}

/// Each index's f32 and f64 `SimState` from `f32_writer` and `f64_writer` is Appendix A's pattern.
fn check_writers(f32_writer: fn(u32) -> SimStateFTLE, f64_writer: fn(u32) -> SimStateFTLEOf<f64>) {
    for i in indices() {
        check(i, &f32_writer(i)).unwrap_or_else(|e| panic!("f32: {e}"));
        check_f64(i, &f64_writer(i)).unwrap_or_else(|e| panic!("f64: {e}"));
    }
}

#[test]
fn debug_variants_bring_up_decodes_through_the_unpack() {
    check_writers(WRITER_F32, WRITER_F64);
}

/// The f32 pattern written one slot off: each real member holds the next slot's value.
#[cfg(feature = "controls")]
fn one_slot_off(i: u32) -> SimStateFTLE {
    let mut s = pattern::<f32>(i);
    let next = |k: u32| kernel::bringup::real_slot::<f32>(i, k + 1);
    s.r = [[next(0), next(1)], [next(2), next(3)], [next(4), next(5)]];
    s.theta = next(25);
    s
}

negative_control!(
    debug_variants_bring_up_decodes_through_the_unpack,
    "the pattern written one slot off must fail the readback (pitfalls §9)",
    expected = "not the bring-up pattern's",
    check_writers(one_slot_off, WRITER_F64)
);

#[cfg(feature = "controls")]
mod reserved_control {
    use super::*;

    /// The pattern with a contaminated reserved bit of `packed_a`, bit 12, which the unpack masks off.
    fn reserved_bit(i: u32) -> SimStateFTLE {
        let mut s = pattern::<f32>(i);
        s.packed_a |= 1 << 12;
        s
    }

    negative_control!(
        debug_variants_bring_up_decodes_through_the_unpack,
        "a reserved bit the unpack masks off must fail the raw-word check (pitfalls §9)",
        expected = "packed_a's raw word matches",
        check_writers(reserved_bit, WRITER_F64)
    );
}

/// Sample 77's and sample 65 537's fields, from Appendix A by hand.
fn check_golden(writer: fn(u32) -> SimStateFTLE) {
    let s = writer(77);
    assert_eq!(
        s.r,
        [[2464.0, 2465.0], [2466.0, 2467.0], [2468.0, 2469.0]],
        "golden: sample 77's r"
    );
    assert_eq!(
        (s.S, s.theta, s.closure_min),
        (2488.0, 2489.0, 2494.0),
        "golden: sample 77's S, theta, closure_min"
    );
    // state 77 mod 6 = 5, detail 38 mod 4 = 2, saturated 9 mod 2 = 1, dmin_pair 4 mod 4 = 0, last_symbol 1, d_min
    // unset 0x7c00.
    assert_eq!(
        s.packed_a,
        5 | (2 << 3) | (1 << 5) | (1 << 8) | (0x7c00 << 16),
        "golden: sample 77's packed_a"
    );
    assert_eq!(s.times, 77 | (65_458 << 16), "golden: sample 77's times");
    assert_eq!(
        (s.total_substeps, s.closure_step, s.packed_b),
        (77, 77, 0),
        "golden: sample 77's counts"
    );
    let s = writer(65_537);
    assert_eq!(s.times, 1 | (65_534 << 16), "golden: sample 65537's times");
    assert_eq!(
        (s.total_substeps, s.closure_step),
        (65_537, 1),
        "golden: sample 65537's counts"
    );
    assert_eq!(s.p[0][0], 2_097_190.0, "golden: sample 65537's p[0][0]");
}

#[test]
fn debug_variants_bring_up_golden() {
    check_golden(WRITER_F32);
}

negative_control!(
    debug_variants_bring_up_golden,
    "the next sample's pattern must fail the golden",
    expected = "golden: sample 77's r",
    check_golden(|i| pattern::<f32>(i + 1))
);

/// `n` samples' buffer, filled by `entry` dispatched natively over `invocations` invocations, as the GPU dispatches it.
fn dispatch(entry: fn(UVec3, &mut [u32]), n: usize, invocations: u32) -> Vec<u32> {
    let mut out = vec![0u32; n * words_per_sample()];
    for i in 0..invocations {
        entry(UVec3::new(i, 0, 0), &mut out);
    }
    out
}

/// Each of `n` samples written by `entry` is the native writer's f32 struct, read at the ledger's offsets.
fn check_gpu_writer(entry: fn(UVec3, &mut [u32]), n: usize) {
    let out = dispatch(entry, n, n as u32 + 64);
    for (i, words) in out.chunks_exact(words_per_sample()).enumerate() {
        let s = simstate_from_words(words).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            s,
            pattern::<f32>(i as u32),
            "the entry point's sample {i} is not the native writer's"
        );
    }
}

/// [`write_pattern`] with its signature as a plain function pointer's.
fn entry(id: UVec3, out: &mut [u32]) {
    write_pattern(id, out)
}

#[test]
fn debug_variants_gpu_writer() {
    check_gpu_writer(entry, 300);
    // Invocations past the buffer's last sample write nothing: a buffer one word short of two samples holds one.
    let mut short = vec![0u32; 2 * words_per_sample() - 1];
    write_pattern(UVec3::new(1, 0, 0), &mut short);
    assert!(
        short.iter().all(|&w| w == 0),
        "invocation 1 wrote into a one-sample buffer"
    );
    let mut words = vec![0u32; words_per_sample()];
    write_words(0, &mut words);
    assert_eq!(
        simstate_from_words(&words).map(|s| s == pattern::<f32>(0)),
        Ok(true),
        "write_words(0) is not sample 0's pattern"
    );
}

/// The entry point writing each sample's words one word late.
#[cfg(feature = "controls")]
fn late_entry(id: UVec3, out: &mut [u32]) {
    let w = words_per_sample();
    let mut scratch = vec![0u32; out.len() + 1];
    write_pattern(id, &mut scratch[..out.len()]);
    let i = id.x as usize;
    if i < out.len() / w {
        let end = ((i + 1) * w + 1).min(out.len());
        out[i * w + 1..end].copy_from_slice(&scratch[i * w..end - 1]);
    }
}

negative_control!(
    debug_variants_gpu_writer,
    "an entry point writing one word late must fail",
    expected = "is not the native writer's",
    check_gpu_writer(late_entry, 300)
);
