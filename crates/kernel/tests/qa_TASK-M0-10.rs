//! QA tests for TASK-M0-10, written from REQ-PAY-004, REQ-PAY-012, REQ-PAY-014, REQ-PAY-015, REQ-PAY-018,
//! REQ-GEN-005 and REQ-PAY-092 against their sources (dd_simstate_payload §1, §2, §6; dd_generation_root §3.1, §5;
//! pitfalls §9; R-22, R-86, R-271), not from the implementation. Every bit position, bit pattern and tolerance below
//! is payload §2's table, IEEE 754 binary16, or the requirement's own figure. Each test has a registered negative
//! control (R-176).
//!
//! R-294 moved the `d_min` counters to the frame: `set_d_min` and `pack_packed_a` take the caller's `DminCounters`.
//! Every value this file packs is valid, so none counts; each pack here goes through [`pa`] or [`sdm`], which pass a
//! fresh frame pair and assert that it reads back `(0, 0)`, so no assertion below changed. The counting itself is
//! `qa_TASK-M0-10_r288.rs`'s.

use kernel::payload::roundtrip::{roundtrip_ctl, PackedA};
use kernel::payload::*;
use validation::negative_control;

/// `pack_packed_a` over a fresh frame pair (R-294), which a valid `d_min` must leave at `(0, 0)` (R-288).
fn pa(
    state: u32,
    detail: u32,
    saturated: bool,
    dmin_pair: u32,
    last_symbol: u32,
    d_min: f32,
) -> u32 {
    let frame = DminCounters::new();
    let w = pack_packed_a(
        state,
        detail,
        saturated,
        dmin_pair,
        last_symbol,
        d_min,
        &frame,
    );
    assert_eq!(
        frame.read(),
        (0, 0),
        "pack_packed_a counted a valid d_min {d_min:e}"
    );
    w
}

/// `set_d_min` over a fresh frame pair (R-294), which a valid `d_min` must leave at `(0, 0)` (R-288).
fn sdm(w: u32, v: f32) -> u32 {
    let frame = DminCounters::new();
    let out = set_d_min(w, v, &frame);
    assert_eq!(
        frame.read(),
        (0, 0),
        "set_d_min counted a valid d_min {v:e}"
    );
    out
}

/// `PackedA::pack` over a fresh frame pair (R-294).
fn pk(e: &PackedA) -> u32 {
    let frame = DminCounters::new();
    let w = e.pack(&frame);
    assert_eq!(frame.read(), (0, 0), "PackedA::pack counted a valid d_min");
    w
}

/// Payload §2's `sample_descriptor` table: `(field, first bit, width)`.
const TABLE: [(&str, u32, u32); 5] = [
    ("state", 0, 3),
    ("detail", 3, 2),
    ("saturated", 5, 1),
    ("dmin_pair", 6, 2),
    ("last_symbol", 8, 2),
];

/// The generated `sd_*` accessor for `field`, as a u32.
fn sd(field: &str, w: u32) -> u32 {
    match field {
        "state" => sd_state(w),
        "detail" => sd_detail(w),
        "saturated" => u32::from(sd_saturated(w)),
        "dmin_pair" => sd_dmin_pair(w),
        "last_symbol" => sd_last_symbol(w),
        _ => unreachable!("{field}"),
    }
}

/// The generated `set_*` insert for `field`, `v` as a u32.
fn set(field: &str, w: u32, v: u32) -> u32 {
    match field {
        "state" => set_state(w, v),
        "detail" => set_detail(w, v),
        "saturated" => set_saturated(w, v != 0),
        "dmin_pair" => set_dmin_pair(w, v),
        "last_symbol" => set_last_symbol(w, v),
        _ => unreachable!("{field}"),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-004: the `sd_*` accessors extract exactly payload §2's bits; all 1024 values of bits 0–9 round-trip; bits
// 10–15 decode as zero.

/// With one bit set at a time over the whole word, each accessor in `table` sees it exactly when it is in its range.
fn check_single_bits(table: &[(&str, u32, u32)]) {
    for bit in 0..32 {
        let w = 1u32 << bit;
        for &(field, off, width) in table {
            let want = if (off..off + width).contains(&bit) {
                1 << (bit - off)
            } else {
                0
            };
            assert_eq!(sd(field, w), want, "sd_{field} on bit {bit} alone");
        }
    }
}

#[test]
fn sd_accessors_qa_each_reads_exactly_its_bits() {
    check_single_bits(&TABLE);
}

negative_control!(
    sd_accessors_qa_each_reads_exactly_its_bits,
    "`detail` at bits 3–5 is not payload §2's table, so the single-bit sweep must fail",
    expected = "sd_detail on bit 5 alone",
    check_single_bits(&[("detail", 3, 3)])
);

type PackA = fn(u32, u32, bool, u32, u32, f32) -> u32;

/// Every one of the 1024 values `v` of bits 0–9, split by payload §2's table, packs through `pack` to a word whose
/// low 16 bits are `v` (so bits 10–15 are zero) and whose accessors return the fields.
fn check_all_1024(pack: PackA) {
    for v in 0u32..1024 {
        let f: Vec<u32> = TABLE
            .iter()
            .map(|&(_, off, width)| (v >> off) & ((1 << width) - 1))
            .collect();
        let w = pack(f[0], f[1], f[2] == 1, f[3], f[4], 1.0);
        assert_eq!(w & 0xffff, v, "descriptor {v:#05x} packed to {w:#010x}");
        for (i, &(field, _, _)) in TABLE.iter().enumerate() {
            assert_eq!(sd(field, w), f[i], "descriptor {v:#05x}: sd_{field}");
        }
        // Unpacking ignores whatever is above bit 9.
        for high in [0u32, 0xfc00, 0xffff_fc00, 0x7c00_0000] {
            for (i, &(field, _, _)) in TABLE.iter().enumerate() {
                assert_eq!(sd(field, v | high), f[i], "descriptor {v:#05x}: sd_{field}");
            }
        }
    }
}

#[test]
fn sd_accessors_qa_all_1024_descriptors_round_trip() {
    check_all_1024(pa);
}

negative_control!(
    sd_accessors_qa_all_1024_descriptors_round_trip,
    "a packer that swaps dmin_pair and last_symbol is not payload §2's, so the round trip must fail",
    expected = "packed to",
    check_all_1024(|s, d, sat, pair, sym, dm| pa(s, d, sat, sym, pair, dm))
);

/// Each `set_*` replaces its field's bits and keeps every other bit, over an all-zero and an all-one word; a value
/// wider than the field is cut to its width (payload §6's Rust `set_last_symbol`: `sym & 0b11`).
fn check_setters(set: fn(&str, u32, u32) -> u32) {
    for &(field, off, width) in &TABLE {
        let mask = ((1u32 << width) - 1) << off;
        for base in [0u32, u32::MAX, 0xa5a5_a5a5] {
            for v in [0u32, 1, (1 << width) - 1, u32::MAX] {
                let want = (base & !mask) | ((v << off) & mask);
                assert_eq!(
                    set(field, base, v),
                    want,
                    "set_{field}({base:#010x}, {v:#x})"
                );
            }
        }
    }
}

#[test]
fn sd_accessors_qa_setters_keep_every_other_bit() {
    check_setters(set);
}

negative_control!(
    sd_accessors_qa_setters_keep_every_other_bit,
    "an OR insert without a mask spills a wide value past its field, so the setter check must fail",
    expected = "set_",
    check_setters(|f, w, v| if f == "last_symbol" {
        w | v << 8
    } else {
        set(f, w, v)
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-012: known values give payload §2's raw words, and the accessors return them.

/// `(case, got, want)` rows.
fn check_rows(rows: &[(&str, u32, u32)]) {
    for &(case, got, want) in rows {
        assert_eq!(got, want, "{case}: got {got:#010x}, want {want:#010x}");
    }
}

/// Raw words for known values. `packed_a` = d_min(1.0 → 0x3c00) << 16 | last_symbol 3 << 8 | dmin_pair 0 << 6 |
/// saturated << 5 | detail 1 << 3 | state 2 = 0x3c00_032a. `packed_b` = dLz_max(2.0 → 0x4000) << 16 | dE_max(0.5 →
/// 0x3800). `times` = t_dmin_step << 16 | t_end_step.
fn known_rows() -> Vec<(&'static str, u32, u32)> {
    vec![
        ("packed_a", pa(2, 1, true, 0, 3, 1.0), 0x3c00_032a),
        ("packed_b", pack_packed_b(0.5, 2.0), 0x4000_3800),
        ("times", pack_times(0x1234, 0xabcd), 0xabcd_1234),
        ("times max end", pack_times(0xffff, 0), 0x0000_ffff),
        ("times max dmin", pack_times(0, 0xffff), 0xffff_0000),
        (
            "set_dE_max keeps high",
            set_dE_max(0xabcd_0000, 1.0),
            0xabcd_3c00,
        ),
        (
            "set_dLz_max keeps low",
            set_dLz_max(0x0000_1234, 1.0),
            0x3c00_1234,
        ),
        ("set_d_min keeps low", sdm(0x0000_03ff, 1.0), 0x3c00_03ff),
        (
            "set_t_end_step keeps high",
            set_t_end_step(0xabcd_0000, 7),
            0xabcd_0007,
        ),
        (
            "set_t_dmin_step keeps low",
            set_t_dmin_step(0x0000_1234, 7),
            0x0007_1234,
        ),
        // Payload §1: valid values are clamped to ±65504 before packing.
        ("dE_max clamp", set_dE_max(0, 1.0e9), 0x0000_7bff),
        ("dE_max clamp -", set_dE_max(0, -1.0e9), 0x0000_fbff),
        (
            "dLz_max clamp inf",
            set_dLz_max(0, f32::INFINITY),
            0x7bff_0000,
        ),
    ]
}

#[test]
fn packed_words_qa_known_values_and_accessors() {
    check_rows(&known_rows());
    let a = 0x3c00_032a;
    assert_eq!(pa_d_min(a), 1.0, "pa_d_min");
    assert_eq!(
        pa_d_min(a),
        unpack2x16float(a)[1],
        "d_min is .y (payload §2)"
    );
    let b = 0x4000_3800;
    assert_eq!(pb_dE_max(b), 0.5, "pb_dE_max");
    assert_eq!(pb_dLz_max(b), 2.0, "pb_dLz_max");
    assert_eq!(
        unpack2x16float(b),
        [0.5, 2.0],
        "dE_max .x, dLz_max .y (payload §2)"
    );
    let t = 0xabcd_1234;
    assert_eq!(tm_t_end_step(t), 0x1234, "tm_t_end_step");
    assert_eq!(tm_t_dmin_step(t), 0xabcd, "tm_t_dmin_step");
    assert_eq!(
        tm_t_end_fraction(pack_times(50, 25), 100),
        0.5,
        "t_end fraction"
    );
    assert_eq!(
        tm_t_dmin_fraction(pack_times(50, 25), 100),
        0.25,
        "t_dmin fraction"
    );
    assert_eq!(
        tm_t_end_fraction(pack_times(50, 25), 0),
        0.0,
        "fraction guard"
    );
}

negative_control!(
    packed_words_qa_known_values_and_accessors,
    "dE_max and dLz_max swapped is not payload §2's packed_b, so the raw-word check must fail",
    expected = "packed_b: got",
    check_rows(&[("packed_b", pack_packed_b(2.0, 0.5), 0x4000_3800)])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-014: codes 0–7 decode; 6 and 7 are finished and untrusted.

/// Each code 0–7, with every bit above the state set, decodes to itself, and the predicates follow payload §2/§6:
/// resolved 0–2, running 3, untrusted 4–7, finished all but 3; exactly one of resolved/running/untrusted.
fn check_codes(failed: fn(u32) -> bool, finished: fn(u32) -> bool) {
    for code in 0u32..8 {
        let w = 0xffff_fff8 | code;
        assert_eq!(sd_state(w), code, "code {code} decodes");
        let (res, run, fail, fin) = (
            sd_is_resolved_outcome(w),
            sd_is_running(w),
            failed(w),
            finished(w),
        );
        assert_eq!(res, code <= 2, "code {code}: resolved");
        assert_eq!(run, code == 3, "code {code}: running");
        assert_eq!(fail, code >= 4, "code {code}: untrusted");
        assert_eq!(fin, code != 3, "code {code}: finished");
        assert_eq!(
            u32::from(res) + u32::from(run) + u32::from(fail),
            1,
            "code {code}: one class"
        );
    }
}

#[test]
fn state_codes_qa_reserved_are_finished_and_untrusted() {
    check_codes(sd_is_failed, sd_is_finished);
}

negative_control!(
    state_codes_qa_reserved_are_finished_and_untrusted,
    "a decoder that trusts the reserved codes is not payload §2's, so the code check must fail",
    expected = "code 6: untrusted",
    check_codes(|w| matches!(sd_state(w), 4 | 5), sd_is_finished)
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-015: with every field at its maximum, bits 10–15 are zero; the reserved list is 10–15.

/// Every combination of each field at 0 or its maximum, with the largest `d_min`s, leaves bits 10–15 zero through
/// `pack`; so does every setter given u32::MAX over a zero word.
fn check_reserved_zero(pack: PackA, set: fn(&str, u32, u32) -> u32) {
    for m in 0u32..32 {
        let on = |i: u32, max: u32| if m >> i & 1 == 1 { max } else { 0 };
        for d_min in [65504.0, f32::MAX, f32::INFINITY, 1.0] {
            let w = pack(on(0, 7), on(1, 3), on(2, 1) == 1, on(3, 3), on(4, 3), d_min);
            assert_eq!(
                w & 0xfc00,
                0,
                "fields {m:#07b} with d_min {d_min}: bits 10–15 {w:#010x}"
            );
        }
    }
    for &(field, _, _) in &TABLE {
        let w = set(field, 0, u32::MAX);
        assert_eq!(w & 0xfc00, 0, "set_{field}(0, MAX): bits 10–15 {w:#010x}");
    }
}

#[test]
fn reserved_bits_qa_zero_with_every_field_at_max() {
    check_reserved_zero(pa, set);
    assert_eq!(PACKED_A_RESERVED, [(10, 6)], "packed_a's reserved list");
}

negative_control!(
    reserved_bits_qa_zero_with_every_field_at_max,
    "an unmasked last_symbol write spills into bit 10, so the reserved check must fail",
    expected = "bits 10–15",
    check_reserved_zero(pa, |f, w, v| if f == "last_symbol" {
        w | v << 8
    } else {
        set(f, w, v)
    })
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-018: roundtrip_ctl fails on a flip of any bit, including those an unpack masks off (pitfalls §9).

type Ctl = fn(&PackedA, u32) -> bool;

/// `ctl` passes the clean word of `e` and fails every single-bit flip of it, and every non-zero value in bits 10–15.
fn check_every_flip(ctl: Ctl, e: &PackedA) {
    let clean = pk(e);
    assert!(ctl(e, clean), "the ctl fails the clean word {clean:#010x}");
    for bit in 0..32 {
        assert!(
            !ctl(e, clean ^ 1 << bit),
            "the ctl passed with bit {bit} flipped"
        );
    }
    for r in 1u32..64 {
        assert!(
            !ctl(e, clean | r << 10),
            "the ctl passed with reserved bits {r:#04x}"
        );
    }
}

/// A field-by-field comparison after unpacking: it cannot see bits no accessor reads.
#[cfg(feature = "controls")]
fn fields_ctl(e: &PackedA, observed: u32) -> bool {
    let key = |p: PackedA| {
        (
            p.state,
            p.detail,
            p.saturated,
            p.dmin_pair,
            p.last_symbol,
            p.d_min.to_bits(),
        )
    };
    key(PackedA::unpack(observed)) == key(PackedA::unpack(pk(e)))
}

/// Pitfalls §9's recorded check: `from_bits` masks to bits 2–4 and compares there.
fn from_bits_ctl(e: &PackedA, observed: u32) -> bool {
    (observed >> 2) & 0b111 == (pk(e) >> 2) & 0b111
}

fn sample() -> PackedA {
    PackedA {
        state: 2,
        detail: 1,
        saturated: true,
        dmin_pair: 0,
        last_symbol: 3,
        d_min: 0.75,
    }
}

#[test]
fn roundtrip_ctl_qa_property_every_bit_of_every_word() {
    // d_min over every positive finite f16 value (0x0001–0x7bff) and the unset +inf (0x7c00).
    let fields = (0u32..8, 0u32..4, 0u32..2, 0u32..4, 0u32..4, 1u32..=0x7c00);
    validation::prop::run(&fields, |(state, detail, sat, pair, sym, h)| {
        let e = PackedA {
            state,
            detail,
            saturated: sat == 1,
            dmin_pair: pair,
            last_symbol: sym,
            d_min: pa_d_min(h << 16),
        };
        check_every_flip(roundtrip_ctl, &e);
        Ok(())
    });
    check_every_flip(roundtrip_ctl, &sample());
}

negative_control!(
    roundtrip_ctl_qa_property_every_bit_of_every_word,
    "a field comparison cannot see reserved bit 10, so the every-flip check must fail",
    expected = "the ctl passed with bit 10 flipped",
    check_every_flip(fields_ctl, &sample())
);

/// The recorded pitfalls §9 pass: `ctl` must fail the bit-0 and bit-1 forks `from_bits` masks off.
fn check_recorded_fork(ctl: Ctl) {
    let clean = pk(&sample());
    for fork in [clean ^ 0b01, clean ^ 0b10, clean ^ 0b11] {
        assert!(
            !ctl(&sample(), fork),
            "the ctl passed the masked fork {fork:#010x}"
        );
    }
}

#[test]
fn roundtrip_ctl_qa_recorded_pitfall_9_pass_now_fails() {
    let clean = pk(&sample());
    // The recorded check does pass the fork (so it is the pitfall, reproduced) ...
    assert!(
        from_bits_ctl(&sample(), clean ^ 0b01),
        "the recorded check did not pass the fork"
    );
    // ... and roundtrip_ctl fails it.
    check_recorded_fork(roundtrip_ctl);
}

negative_control!(
    roundtrip_ctl_qa_recorded_pitfall_9_pass_now_fails,
    "the recorded from_bits check masks bits 0–1, so the fork check must fail",
    expected = "the ctl passed the masked fork",
    check_recorded_fork(from_bits_ctl)
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-005: f16 pairs round-trip through the pack2x16float / unpack2x16float equivalents within f16 epsilon over
// finite values in ±65504.

/// f16 epsilon, 2⁻¹⁰, at `x`: one f16 ULP for a normal (2⁻¹⁰·2^⌊log₂|x|⌋), the subnormal step 2⁻²⁴ below 2⁻¹⁴.
fn f16_eps_at(x: f32) -> f32 {
    let min_normal = f32::from_bits(0x3880_0000); // 2⁻¹⁴
    if x.abs() < min_normal {
        f32::from_bits(0x3380_0000) // 2⁻²⁴
    } else {
        f32::from_bits(x.abs().to_bits() & 0x7f80_0000) * f32::from_bits(0x3a80_0000)
        // · 2⁻¹⁰
    }
}

fn check_pair(pack: fn([f32; 2]) -> u32, x: f32, y: f32) {
    let back = unpack2x16float(pack([x, y]));
    for (i, (v, b)) in [(x, back[0]), (y, back[1])].into_iter().enumerate() {
        assert!(
            b.is_finite() && (b - v).abs() <= f16_eps_at(v),
            "half {i}: {v:e} came back {b:e}: not within f16 epsilon"
        );
    }
}

#[test]
fn f16_pairs_qa_within_epsilon_over_65504() {
    // Uniform in value, and uniform in bits (which reaches the subnormal and tiny range).
    validation::prop::run(&(-65504.0f32..=65504.0, -65504.0f32..=65504.0), |(x, y)| {
        check_pair(pack2x16float, x, y);
        Ok(())
    });
    let bits = (
        0u32..=0x477f_e000,
        any_sign(),
        0u32..=0x477f_e000,
        any_sign(),
    );
    validation::prop::run(&bits, |(a, sa, b, sb)| {
        check_pair(
            pack2x16float,
            f32::from_bits(a | sa << 31),
            f32::from_bits(b | sb << 31),
        );
        Ok(())
    });
    for x in [
        0.0,
        -0.0,
        65504.0,
        -65504.0,
        1.0e-9,
        3.0e-8,
        6.1e-5,
        1.0 / 3.0,
    ] {
        check_pair(pack2x16float, x, -x);
    }
}

fn any_sign() -> std::ops::Range<u32> {
    0..2
}

negative_control!(
    f16_pairs_qa_within_epsilon_over_65504,
    "a pack keeping 8 mantissa bits loses 1 + 2⁻⁹, so the epsilon check must fail",
    expected = "not within f16 epsilon",
    check_pair(
        |v| pack2x16float([f32::from_bits(v[0].to_bits() & !0x7fff), v[1]]),
        1.0 + f32::from_bits(0x3b00_0000),
        1.0
    )
);

/// IEEE 754 binary16 bit patterns of known values, in both halves.
fn check_known_f16(pack: fn([f32; 2]) -> u32) {
    let known: [(f32, u32); 10] = [
        (1.0, 0x3c00),
        (-2.0, 0xc000),
        (0.5, 0x3800),
        (65504.0, 0x7bff),
        (f32::from_bits(0x3880_0000), 0x0400), // 2⁻¹⁴, smallest normal
        (f32::from_bits(0x3380_0000), 0x0001), // 2⁻²⁴, smallest subnormal
        (f32::from_bits(0x387f_c000), 0x03ff), // 2⁻¹⁴ − 2⁻²⁴, largest subnormal
        (0.1, 0x2e66),
        (1.0 / 3.0, 0x3555),
        (-0.0, 0x8000),
    ];
    for (x, h) in known {
        assert_eq!(pack([x, 0.0]) & 0xffff, h, "{x:e} in .x");
        assert_eq!(pack([0.0, x]) >> 16, h, "{x:e} in .y");
        assert_eq!(
            unpack2x16float(h | h << 16),
            [x_as_f16(h, x); 2],
            "{h:#06x} unpacks"
        );
    }
}

/// The value of `h` when `x` is exact in binary16, else `x` rounded as IEEE gives it (0.1 and 1/3).
fn x_as_f16(h: u32, x: f32) -> f32 {
    match h {
        0x2e66 => 1638.0 / 16384.0,
        0x3555 => 1365.0 / 4096.0,
        _ => x,
    }
}

#[test]
fn f16_pairs_qa_known_binary16_patterns() {
    check_known_f16(pack2x16float);
}

negative_control!(
    f16_pairs_qa_known_binary16_patterns,
    "a pack with its halves swapped puts 1.0 in .y, so the known-pattern check must fail",
    expected = "in .x",
    check_known_f16(|v| pack2x16float([v[1], v[0]]))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-092: d_min's unset value is f16 +inf (0x7c00); a valid value below 2⁻²⁴ stores 0x0001; no valid input
// stores 0x0000; the unset test reads bits; the packer writes both patterns itself (R-271).

type SetDMin = fn(u32, f32) -> u32;

/// Three named cases store 0x7c00 with the descriptor half untouched (R-271):
/// - a failed sample (states 4, 5), written unset by bits with `set_d_min_unset`;
/// - an unstepped sample (state 3, before its first step), written unset by bits with `set_d_min_unset`. R-271's
///   +inf is a stored value written by its bits, not a fold seed (integrator contract § "Rules the new kernel must
///   hold by construction", rule 4; PIT-9), so no latch value is modelled here;
/// - an f32 +inf given to the packer `set` (a plain `f32::INFINITY` input). Through the clamping conversion payload
///   §1 requires of valid values, +inf would store 0x7bff: so `set` writes the unset bits itself.
fn check_unset_written(set: SetDMin) {
    for state in [4u32, 5] {
        let w = set_d_min_unset(pa(state, 2, true, 3, 1, 0.5));
        assert_eq!(w >> 16, 0x7c00, "failed state {state}: d_min bits");
        assert_eq!(
            w & 0xffff,
            pa(state, 2, true, 3, 1, 0.5) & 0xffff,
            "failed state {state}: descriptor kept"
        );
        assert!(pa_d_min_is_unset(w), "failed state {state} reads unset");
    }
    let unstepped = set_d_min_unset(pa(3, 0, false, SD_DMIN_PAIR_SENTINEL, 0, 0.5));
    assert_eq!(unstepped >> 16, 0x7c00, "unstepped: d_min bits");
    assert_eq!(
        unstepped & 0xffff,
        pa(3, 0, false, SD_DMIN_PAIR_SENTINEL, 0, 0.5) & 0xffff,
        "unstepped: descriptor kept"
    );
    assert!(pa_d_min_is_unset(unstepped), "unstepped sample reads unset");
    for low in [0u32, 0x03ff, 0xffff] {
        assert_eq!(
            set_d_min_unset(0x3c00_0000 | low),
            0x7c00_0000 | low,
            "set_d_min_unset"
        );
    }
    for state in [3u32, 4, 5] {
        let w = set(pa(state, 2, true, 3, 1, 0.5), f32::INFINITY);
        assert_eq!(
            w >> 16,
            0x7c00,
            "+inf to the packer, state {state}: d_min bits"
        );
        assert_eq!(
            w & 0xffff,
            pa(state, 2, true, 3, 1, 0.5) & 0xffff,
            "+inf to the packer, state {state}: descriptor kept"
        );
    }
}

#[test]
fn dmin_unset_qa_failed_and_unstepped_store_7c00() {
    check_unset_written(sdm);
}

negative_control!(
    dmin_unset_qa_failed_and_unstepped_store_7c00,
    "the clamping conversion stores +inf as 65504 (0x7bff), so the unset check must fail",
    expected = "d_min bits",
    check_unset_written(|w, v| insert(w, u32::from(f32_to_f16_bits(clamp_f16(v))), 16, 16))
);

/// Valid inputs below 2⁻²⁴ (0, −0 as zero distance, 1e-9, 2⁻²⁵, the f32 subnormals) store 0x0001; 2⁻²⁴ itself too.
fn check_floor(set: SetDMin) {
    for v in [
        1.0e-9,
        0.0,
        -0.0,
        f32::from_bits(0x3300_0000),
        f32::from_bits(0x337f_ffff),
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        f32::from_bits(0x3380_0000),
    ] {
        assert_eq!(set(0, v) >> 16, 0x0001, "{v:e}: d_min bits");
    }
    assert_eq!(
        F16_MIN_SUBNORMAL,
        f32::from_bits(0x3380_0000),
        "F16_MIN_SUBNORMAL is 2⁻²⁴"
    );
    assert_eq!(F16_MIN_SUBNORMAL_BITS, 0x0001, "its bits");
}

#[test]
fn dmin_unset_qa_below_smallest_subnormal_stores_0001() {
    check_floor(sdm);
}

negative_control!(
    dmin_unset_qa_below_smallest_subnormal_stores_0001,
    "through the conversion alone 1e-9 rounds to 0x0000, so the floor check must fail",
    expected = "d_min bits",
    check_floor(|w, v| insert(w, u32::from(f32_to_f16_bits(clamp_f16(v))), 16, 16))
);

/// Over every non-negative f32 bit pattern (finite and +inf), `set` never stores 0x0000, and a finite value never
/// stores the unset 0x7c00.
fn check_valid(set: SetDMin, bits: u32) {
    let v = f32::from_bits(bits);
    let h = set(0x0000_03ff, v) >> 16;
    assert_ne!(h, 0x0000, "d_min {v:e} stored 0x0000");
    if v.is_finite() {
        assert_ne!(h, 0x7c00, "finite d_min {v:e} stored as unset");
    }
}

#[test]
fn dmin_unset_qa_property_no_valid_input_stores_zero() {
    validation::prop::run(&(0u32..=0x7f80_0000), |bits| {
        check_valid(sdm, bits);
        Ok(())
    });
    // Around the edges: zero, the f16 subnormal floor, the finite max and the f16 overflow threshold, f32::MAX, +inf.
    for bits in [
        0,
        1,
        0x3300_0000,
        0x337f_ffff,
        0x3380_0000,
        0x3380_0001,
        0x477f_e000,
        0x477f_f000,
        0x7f7f_ffff,
        0x7f80_0000,
    ] {
        check_valid(sdm, bits);
    }
}

negative_control!(
    dmin_unset_qa_property_no_valid_input_stores_zero,
    "an unclamped conversion stores f32::MAX as +inf, so the finite-never-unset check must fail",
    expected = "stored as unset",
    check_valid(
        |w, v| insert(w, u32::from(f32_to_f16_bits(v)), 16, 16),
        0x7f7f_ffff
    )
);

/// `is_unset` holds exactly when bits 16–31 are 0x7c00, whatever the descriptor half: not for −inf (0xfc00), NaNs or
/// 65504.
fn check_unset_bits(is_unset: fn(u32) -> bool) {
    for h in 0u32..=0xffff {
        for low in [0u32, 0xffff] {
            let w = h << 16 | low;
            assert_eq!(is_unset(w), h == 0x7c00, "is_unset on d_min bits {h:#06x}");
        }
    }
}

#[test]
fn dmin_unset_qa_test_reads_bits() {
    check_unset_bits(pa_d_min_is_unset);
    assert_eq!(
        PA_D_MIN_SENTINEL.to_bits(),
        0x7f80_0000,
        "the emitted sentinel is +inf, by its bits"
    );
}

negative_control!(
    dmin_unset_qa_test_reads_bits,
    "a float test for any infinity takes −inf (0xfc00) as unset, so the bits check must fail",
    expected = "is_unset on d_min bits 0xfc00",
    check_unset_bits(|w| pa_d_min(w).is_infinite())
);

/// The unset `d_min` (f16 +inf, 0x7c00, stored by bits under R-271) reads back as f32 +inf, and −inf and NaN halves read back as
/// themselves: `unpack2x16float` converts every binary16 pattern exactly.
fn check_non_finite_unpack(unpack: fn(u32) -> [f32; 2]) {
    assert_eq!(
        unpack(PA_D_MIN_UNSET << 16)[1],
        f32::INFINITY,
        "unset d_min reads back"
    );
    assert_eq!(
        unpack(0xfc00_7c00),
        [f32::INFINITY, f32::NEG_INFINITY],
        "±inf halves read back"
    );
    let nan = unpack(0x7e00_fe01);
    assert!(nan[0].is_nan() && nan[1].is_nan(), "NaN halves read back");
}

#[test]
fn dmin_unset_qa_unset_reads_back_as_positive_infinity() {
    check_non_finite_unpack(unpack2x16float);
    assert_eq!(
        pa_d_min(set_d_min_unset(0x03ff)),
        f32::INFINITY,
        "pa_d_min of the unset word"
    );
}

negative_control!(
    dmin_unset_qa_unset_reads_back_as_positive_infinity,
    "an unpack that zeroes infinities reads unset as 0.0, so the read-back check must fail",
    expected = "unset d_min reads back",
    check_non_finite_unpack(|w| unpack2x16float(w).map(|x| if x.is_infinite() { 0.0 } else { x }))
);

// ---------------------------------------------------------------------------------------------------------------
// Payload §6's remaining named accessors: `sd_last_symbol_valid` (payload §2: meaningful iff length ≥ 1 and length ≠
// 127) and `total_substeps_log2` (⌊log₂ total⌋, 0 for total ≤ 1, derived from the exact counter).

fn check_last_symbol_valid(valid: fn(u32) -> bool) {
    for len in 0u32..=127 {
        assert_eq!(
            valid(len),
            len >= 1 && len != 127,
            "last_symbol valid at length {len}"
        );
    }
}

#[test]
fn sd_accessors_qa_last_symbol_valid_gates_on_word_length() {
    check_last_symbol_valid(sd_last_symbol_valid);
}

negative_control!(
    sd_accessors_qa_last_symbol_valid_gates_on_word_length,
    "a gate that trusts a truncated word (127) is not payload §2's, so the validity check must fail",
    expected = "last_symbol valid at length 127",
    check_last_symbol_valid(|len| len >= 1)
);

/// ⌊log₂ t⌋ by repeated halving, 0 for t ≤ 1.
fn floor_log2(mut t: u32) -> u32 {
    let mut k = 0;
    while t > 1 {
        t >>= 1;
        k += 1;
    }
    k
}

fn check_log2(log2: fn(u32) -> u32) {
    let mut totals = vec![0u32, 1, 2, 3, 4, 1023, 1024, 2047, 2048, u32::MAX];
    totals.extend((0..32).map(|k| 1u32 << k));
    totals.extend((1..32).map(|k| (1u32 << k) - 1));
    for t in totals {
        assert_eq!(log2(t), floor_log2(t), "total_substeps_log2({t})");
    }
}

#[test]
fn sd_accessors_qa_total_substeps_log2_is_floor_log2() {
    check_log2(total_substeps_log2);
}

negative_control!(
    sd_accessors_qa_total_substeps_log2_is_floor_log2,
    "32 − clz is ⌊log₂⌋ + 1, so the log2 check must fail",
    expected = "total_substeps_log2(",
    check_log2(|t| 32 - t.leading_zeros())
);
