//! QA tests for TASK-M5-01 on the engine side: the generated `QuadReduction` as the engine compiles it, and
//! `QuadMeta`. Written from the requirements and dd_generation_root §3.7 ("Size", "Bin width (R-410)", "Layout and
//! §3.8 entries", "Not reduction fields", the temporal-accumulators sentinel paragraph), not from the implementation:
//! - REQ-PAY-089 / REQ-PAY-077: 44 four-byte slots, 176 B, aligned to 4, no padding: `class_histogram` bins at slots
//!   0–31, the packed words `qr_w0` … `qr_w8` at slots 32–40, then `running_mean_divergence`, `first_divergence_t`,
//!   `coherence` at slots 41–43. The u8-binned struct §3.7 used to describe (80 B) is the control.
//! - REQ-PAY-075 (R-410): each bin is a u32 and counts Extreme's provisional 16² × 16 = 4096 copies and 2³² − 1 without
//!   wrapping; §3.7's former u8 bin wraps at Medium's 16² × 2 = 512.
//! - REQ-PAY-077: each packed member's accessor reads exactly its bits of its word, as §3.7's table places them; an f16
//!   member reads its binary16 value widened to f32 (R-312, R-317), checked against binary16 values whose bits are known.
//! - REQ-PAY-094: `first_divergence_t`'s sentinel is +∞, f32 bits 0x7f80_0000, tested by its bits, never NaN.
//! - REQ-REF-007 / REQ-REF-008: the struct's members are exactly the 13 above (no `spread_t_end`, no CPU metadata);
//!   `QuadMeta` carries `priority`, `cache_age`, `lifecycle`, `compute_cost_ms` and `gentime`.
//!
//! Each test has a registered negative control (R-176).

use std::mem::{align_of, offset_of, size_of};

use engine::quad::*;
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// The layout, transcribed from §3.7's "Layout and §3.8 entries".

/// The struct's members in order with their 4-byte slot and their slot count.
const SLOTS: [(&str, usize, usize); 13] = [
    ("class_histogram", 0, 32),
    ("qr_w0", 32, 1),
    ("qr_w1", 33, 1),
    ("qr_w2", 34, 1),
    ("qr_w3", 35, 1),
    ("qr_w4", 36, 1),
    ("qr_w5", 37, 1),
    ("qr_w6", 38, 1),
    ("qr_w7", 39, 1),
    ("qr_w8", 40, 1),
    ("running_mean_divergence", 41, 1),
    ("first_divergence_t", 42, 1),
    ("coherence", 43, 1),
];

/// The aligned sum of the member list with `bin_bytes`-byte histogram bins: 32 bins, nine u32 words, three f32s,
/// rounded up to the alignment 4.
fn member_list_size(bin_bytes: usize) -> usize {
    let sum = 32 * bin_bytes + 9 * 4 + 3 * 4;
    sum.div_ceil(4) * 4
}

/// The compiled struct is `size` bytes, aligned to 4, with each member at byte 4·slot.
fn check_size(size: usize, offsets: &[(&str, usize)]) {
    assert_eq!(
        size, 176,
        "QuadReduction is {size} B, not §3.7's 176 B (R-410)"
    );
    assert_eq!(
        size,
        member_list_size(4),
        "QuadReduction's size is not the aligned sum of its member list with u32 bins"
    );
    for (&(name, slot, _), &(got_name, at)) in SLOTS.iter().zip(offsets) {
        assert_eq!(got_name, name);
        assert_eq!(at, 4 * slot, "`{name}` is at byte {at}, not 4·{slot}");
    }
    let last = SLOTS[SLOTS.len() - 1];
    assert_eq!(
        4 * (last.1 + last.2),
        size,
        "the slots leave padding at the end"
    );
}

fn offsets() -> Vec<(&'static str, usize)> {
    vec![
        (
            "class_histogram",
            offset_of!(QuadReduction, class_histogram),
        ),
        ("qr_w0", offset_of!(QuadReduction, qr_w0)),
        ("qr_w1", offset_of!(QuadReduction, qr_w1)),
        ("qr_w2", offset_of!(QuadReduction, qr_w2)),
        ("qr_w3", offset_of!(QuadReduction, qr_w3)),
        ("qr_w4", offset_of!(QuadReduction, qr_w4)),
        ("qr_w5", offset_of!(QuadReduction, qr_w5)),
        ("qr_w6", offset_of!(QuadReduction, qr_w6)),
        ("qr_w7", offset_of!(QuadReduction, qr_w7)),
        ("qr_w8", offset_of!(QuadReduction, qr_w8)),
        (
            "running_mean_divergence",
            offset_of!(QuadReduction, running_mean_divergence),
        ),
        (
            "first_divergence_t",
            offset_of!(QuadReduction, first_divergence_t),
        ),
        ("coherence", offset_of!(QuadReduction, coherence)),
    ]
}

#[test]
fn qa_quad_reduction_size_is_176_aligned_4() {
    check_size(size_of::<QuadReduction>(), &offsets());
    assert_eq!(
        align_of::<QuadReduction>(),
        4,
        "QuadReduction is aligned to 4"
    );
    assert_eq!(QUAD_REDUCTION_SIZE, 176, "QUAD_REDUCTION_SIZE is not 176");
    assert_eq!(CLASS_HISTOGRAM_BINS, 32, "the bin count is 2⁵ = 32");
    assert_eq!(
        size_of::<[u32; CLASS_HISTOGRAM_BINS]>(),
        4 * 32,
        "the histogram is 32 u32s"
    );
}

negative_control!(
    qa_quad_reduction_size_is_176_aligned_4,
    "the struct §3.7 described with u8 bins (80 B) is not the u32-binned 176 B",
    expected = "QuadReduction is 80 B, not §3.7's 176 B",
    check_size(member_list_size(1), &offsets())
);

negative_control!(
    qa_quad_reduction_member_off_its_slot,
    "coherence at byte 168 is off slot 43",
    expected = "`coherence` is at byte 168, not 4·43",
    {
        let mut o = offsets();
        o[12].1 = 168;
        check_size(176, &o);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-075 (R-410): the bin counts N² × (E+1) copies without wrapping.

/// Counts `copies` footprint copies into one bin, one at a time as a histogram does, by `add` (the bin type's wrapping
/// add); the bin must read `copies` afterwards.
fn count_into<T: Copy + Into<u64>>(zero: T, one: T, add: fn(T, T) -> T, copies: u64) {
    let mut bin = zero;
    for _ in 0..copies {
        bin = add(bin, one);
    }
    let got: u64 = bin.into();
    assert_eq!(
        got, copies,
        "a bin counting {copies} copies reads {got}: it wrapped"
    );
}

/// Sets a bin of a real `QuadReduction` to `copies` and reads it back.
fn store_in_bin(copies: u64) {
    let mut q = QuadReduction::default();
    let k = CLASS_HISTOGRAM_BINS - 1;
    q.class_histogram[k] = u32::try_from(copies)
        .unwrap_or_else(|_| panic!("a QuadReduction bin cannot hold {copies} copies"));
    assert_eq!(u64::from(q.class_histogram[k]), copies);
}

/// Extreme (provisional, R-137): N = 16 samples per quad side, E = 15, so N² × (E+1) copies per quad.
const EXTREME: u64 = 16 * 16 * 16;
/// Medium: N = 16, E = 1.
const MEDIUM: u64 = 16 * 16 * 2;
/// The bound §3.7 states and dispatch asserts: N² × (E+1) ≤ 2³² − 1.
const BOUND: u64 = (1 << 32) - 1;

#[test]
fn qa_quad_reduction_histogram_capacity_u32_bins() {
    assert_eq!(EXTREME, 4096);
    count_into(0u32, 1u32, u32::wrapping_add, EXTREME);
    count_into(0u32, 1u32, u32::wrapping_add, MEDIUM);
    store_in_bin(EXTREME);
    store_in_bin(BOUND);
    // One past the bound is the first count a u32 bin cannot hold; dispatch refuses it (R-410).
    let q = QuadReduction::default();
    assert_eq!(
        q.class_histogram[0].wrapping_add(u32::MAX).wrapping_add(1),
        0,
        "2³² copies wraps a u32 bin, so the bound is exactly 2³² − 1"
    );
    // At N = 32 (a 1024-invocation workgroup, memory_tiers §5) the bound allows E + 1 up to ⌊(2³² − 1)/1024⌋, about
    // 4.19 M copies per footprint (§3.7).
    assert_eq!(BOUND / (32 * 32), 4_194_303);
}

negative_control!(
    qa_quad_reduction_histogram_capacity_u32_bins,
    "§3.7's former u8 bin wraps at Medium's 16² × 2 = 512 copies",
    expected = "a bin counting 512 copies reads 0: it wrapped",
    count_into(0u8, 1u8, u8::wrapping_add, MEDIUM)
);

negative_control!(
    qa_quad_reduction_histogram_capacity_past_bound,
    "2³² copies is past what a u32 bin holds",
    expected = "a QuadReduction bin cannot hold 4294967296 copies",
    store_in_bin(BOUND + 1)
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-077: the packed accessors read §3.7's bits.

/// A packed member's accessor, unsigned bits or an f16 widened to f32.
#[derive(Clone, Copy)]
enum Read {
    Bits(fn(u32) -> u32),
    Half(fn(u32) -> f32),
}

/// §3.7's packed members: name, word index (`qr_w<i>`), offset, width, accessor.
fn packed() -> Vec<(&'static str, usize, u32, u32, Read)> {
    use Read::{Bits, Half};
    vec![
        ("dominant_outcome", 0, 0, 5, Bits(qr_dominant_outcome)),
        ("spread_winner", 0, 5, 2, Bits(qr_spread_winner)),
        ("level", 0, 8, 8, Bits(qr_level)),
        ("outcome_impurity", 0, 16, 16, Half(qr_outcome_impurity)),
        (
            "terminated_fraction",
            1,
            0,
            16,
            Half(qr_terminated_fraction),
        ),
        ("spread_shape", 1, 16, 16, Half(qr_spread_shape)),
        ("spread_event", 2, 0, 16, Half(qr_spread_event)),
        ("error_ratio", 2, 16, 16, Half(qr_error_ratio)),
        ("roundtrip_error", 3, 0, 16, Half(qr_roundtrip_error)),
        ("alpha_area", 3, 16, 16, Half(qr_alpha_area)),
        ("alpha_energy", 4, 0, 16, Half(qr_alpha_energy)),
        ("worst_energy_drift", 4, 16, 16, Half(qr_worst_energy_drift)),
        ("n_unresolved", 5, 0, 16, Bits(qr_n_unresolved)),
        ("suspect_fraction", 5, 16, 16, Half(qr_suspect_fraction)),
        ("saturated_fraction", 6, 0, 16, Half(qr_saturated_fraction)),
        ("valid_sample_count", 6, 16, 16, Bits(qr_valid_sample_count)),
        ("escape_time_min", 7, 0, 16, Half(qr_escape_time_min)),
        ("escape_time_max", 7, 16, 16, Half(qr_escape_time_max)),
        (
            "retrograde_fraction",
            8,
            0,
            16,
            Half(qr_retrograde_fraction),
        ),
        (
            "mean_orbit_count_spread",
            8,
            16,
            16,
            Half(qr_mean_orbit_count_spread),
        ),
    ]
}

/// binary16 bit patterns with their values, known independently of any decoder (IEEE 754 binary16).
const HALVES: [(u16, f32); 11] = [
    (0x0000, 0.0),
    (0x8000, -0.0),
    (0x3c00, 1.0),
    (0x3800, 0.5),
    (0xc000, -2.0),
    (0x7bff, 65504.0),
    (0xfbff, -65504.0),
    (0x7c00, f32::INFINITY),
    (0x0001, f32::from_bits(0x3380_0000)), // 2⁻²⁴, the least subnormal
    (0x0400, f32::from_bits(0x3880_0000)), // 2⁻¹⁴, the least normal
    (0x5a66, 204.75),                      // §3.7's measured error_ratio, 204.8, to binary16
];

/// `value` at bits `offset .. offset + width` of a word whose other bits are `background`.
fn place(background: u32, offset: u32, width: u32, value: u32) -> u32 {
    let mask = (u32::MAX >> (32 - width)) << offset;
    (background & !mask) | ((value << offset) & mask)
}

fn check_accessors(members: &[(&str, usize, u32, u32, Read)]) {
    for &(name, _, offset, width, read) in members {
        for background in [0u32, u32::MAX, 0xa5a5_a5a5, 0x5a5a_5a5a] {
            match read {
                Read::Bits(f) => {
                    let max = u32::MAX >> (32 - width);
                    for v in [0, 1, max, max / 3, max ^ (max >> 1)] {
                        let w = place(background, offset, width, v);
                        assert_eq!(f(w), v, "qr_{name}({w:#010x}) does not read {v}");
                    }
                }
                Read::Half(f) => {
                    for (h, want) in HALVES {
                        let w = place(background, offset, width, u32::from(h));
                        assert_eq!(
                            f(w).to_bits(),
                            want.to_bits(),
                            "qr_{name}({w:#010x}) does not read binary16 {h:#06x} = {want}"
                        );
                    }
                    let nan = place(background, offset, width, 0x7e00);
                    assert!(
                        f(nan).is_nan(),
                        "qr_{name} does not read binary16 NaN as NaN"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_quad_reduction_layout_accessors_read_section_3_7_bits() {
    check_accessors(&packed());
    // The members tile each word: every bit is some member's, except qr_w0's reserved bit 7.
    for word in 0..9 {
        let mut used = 0u32;
        for &(name, w, offset, width, _) in &packed() {
            if w == word {
                let mask = (u32::MAX >> (32 - width)) << offset;
                assert_eq!(
                    used & mask,
                    0,
                    "`{name}` overlaps another member of qr_w{word}"
                );
                used |= mask;
            }
        }
        let reserved = if word == 0 { 1 << 7 } else { 0 };
        assert_eq!(used | reserved, u32::MAX, "qr_w{word} has uncovered bits");
    }
}

negative_control!(
    qa_quad_reduction_layout_accessors_read_section_3_7_bits,
    "level placed at bit 7, qr_w0's reserved bit, is not where its accessor reads",
    expected = "qr_level(",
    {
        let mut m = packed();
        m[2].2 = 7;
        check_accessors(&m[2..3]);
    }
);

negative_control!(
    qa_quad_reduction_layout_half_read_from_the_wrong_half,
    "error_ratio read from qr_w2's low half reads spread_event's bits",
    expected = "qr_error_ratio(",
    {
        let mut m = packed();
        m[7].4 = Read::Half(qr_spread_event);
        check_accessors(&m[7..8]);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-094: `first_divergence_t`'s sentinel, +∞ by its bits.

fn check_sentinel(bits: u32, is_sentinel: fn(f32) -> bool) {
    assert_eq!(
        bits, 0x7f80_0000,
        "first_divergence_t's sentinel bits {bits:#010x} are not +∞'s"
    );
    assert!(!f32::from_bits(bits).is_nan(), "the sentinel is NaN");
    assert!(is_sentinel(f32::INFINITY), "+∞ is not read as the sentinel");
    // Every crossing time is finite and ≥ 0; NaN is the invalid value; −∞ and other infinity-adjacent bits are not it.
    for v in [
        0.0f32,
        -0.0,
        1.0,
        16.0,
        1.0e30,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xff80_0000),
    ] {
        assert!(
            !is_sentinel(v),
            "{v:?} ({:#010x}) is read as the sentinel",
            v.to_bits()
        );
    }
}

#[test]
fn qa_quad_reduction_first_divergence_t_sentinel_is_infinity() {
    check_sentinel(
        QR_FIRST_DIVERGENCE_T_SENTINEL_BITS,
        qr_first_divergence_t_is_sentinel,
    );
}

negative_control!(
    qa_quad_reduction_first_divergence_t_sentinel_is_infinity,
    "a sentinel test by float comparison `v >= f32::MAX` takes f32::MAX, a crossing time, for the sentinel",
    expected = "is read as the sentinel",
    check_sentinel(0x7f80_0000, |v| v >= f32::MAX)
);

negative_control!(
    qa_quad_reduction_first_divergence_t_sentinel_nan,
    "a NaN sentinel is the invalid value",
    expected = "are not +∞'s",
    check_sentinel(0x7fc0_0000, |v| v.is_nan())
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-REF-007, REQ-REF-008: the GPU struct's members are exactly §3.7's; the CPU metadata lives in `QuadMeta`.

/// The names `Debug` prints for a value's fields, in order: `Name { a: …, b: … }`, nested values skipped.
fn field_names(debug: &str) -> Vec<String> {
    let inner = &debug[debug.find('{').expect("a struct") + 1..debug.rfind('}').unwrap()];
    let mut names = Vec::new();
    let mut depth = 0;
    let mut token = String::new();
    for c in inner.chars() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            ':' if depth == 0 => {
                let n = token.trim().trim_start_matches(',').trim().to_owned();
                names.push(n);
                token.clear();
                continue;
            }
            ',' if depth == 0 => {
                token.clear();
                continue;
            }
            _ => {}
        }
        if depth == 0 {
            token.push(c);
        }
    }
    names
}

/// Not `QuadReduction` members: the conditional `spread_t_end` (REQ-REF-007), the CPU quad metadata in both spellings
/// (REQ-REF-008), the omitted `id`, the per-footprint latch and the dropped trend member (§3.7).
const NOT_MEMBERS: [&str; 13] = [
    "spread_t_end",
    "priority",
    "cacheAge",
    "cache_age",
    "lifecycle",
    "computeCostMs",
    "compute_cost_ms",
    "gentime",
    "id",
    "running_max_divergence",
    "divergence_trend",
    "parent_child_disagreement",
    "alpha",
];

fn check_gpu_members(debug: &str) {
    let names = field_names(debug);
    for n in &names {
        assert!(
            !NOT_MEMBERS.contains(&n.as_str()),
            "`{n}` is a QuadReduction member"
        );
    }
    let want: Vec<&str> = SLOTS.iter().map(|s| s.0).collect();
    assert_eq!(names, want, "QuadReduction's members are not §3.7's layout");
}

#[test]
fn qa_quad_reduction_holds_no_cpu_metadata_or_spread_t_end() {
    check_gpu_members(&format!("{:?}", QuadReduction::default()));
    // The CPU metadata has its own struct, one per quad beside the reduction (§3.7, "Not reduction fields").
    let meta = QuadMeta {
        priority: 0.5,
        cache_age: 3,
        lifecycle: 2,
        compute_cost_ms: 1.25,
        gentime: 7,
    };
    let names = field_names(&format!("{meta:?}"));
    for want in [
        "priority",
        "cache_age",
        "lifecycle",
        "compute_cost_ms",
        "gentime",
    ] {
        assert!(names.iter().any(|n| n == want), "QuadMeta has no `{want}`");
    }
}

negative_control!(
    qa_quad_reduction_holds_no_cpu_metadata_or_spread_t_end,
    "a QuadReduction with a gentime member",
    expected = "`gentime` is a QuadReduction member",
    check_gpu_members(&format!("{:?}", QuadReduction::default()).replacen(
        "coherence: ",
        "gentime: 0, coherence: ",
        1
    ))
);

negative_control!(
    qa_quad_reduction_holds_no_spread_t_end,
    "a QuadReduction with a spread_t_end member",
    expected = "`spread_t_end` is a QuadReduction member",
    check_gpu_members(&format!("{:?}", QuadReduction::default()).replacen(
        "coherence: ",
        "spread_t_end: 0.0, coherence: ",
        1
    ))
);
