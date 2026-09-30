//! The generated pack/unpack/insert accessors against dd_simstate_payload §1, §2 and §6 (REQ-PAY-004, REQ-PAY-012,
//! REQ-PAY-014, REQ-PAY-015, REQ-PAY-018, REQ-GEN-005, REQ-PAY-092): the descriptor's bit ranges, the packed words'
//! raw bits, the state codes, the reserved bits, `roundtrip_ctl` over every bit (pitfalls §9), the binary16
//! conversion, and `d_min`'s unset value and subnormal floor (R-271).

use kernel::payload::roundtrip::{roundtrip_ctl, PackedA};
use kernel::payload::*;
use std::sync::atomic::Ordering;
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-004: the `sd_*` accessors, over payload §2's table.

/// Payload §2's descriptor table: `(field, first bit, width)`.
const TABLE: [(&str, u32, u32); 5] = [
    ("state", 0, 3),
    ("detail", 3, 2),
    ("saturated", 5, 1),
    ("dmin_pair", 6, 2),
    ("last_symbol", 8, 2),
];

/// The accessor of `field`, as a u32.
fn accessor(field: &str) -> fn(u32) -> u32 {
    match field {
        "state" => sd_state,
        "detail" => sd_detail,
        "saturated" => |w| u32::from(sd_saturated(w)),
        "dmin_pair" => sd_dmin_pair,
        "last_symbol" => sd_last_symbol,
        _ => panic!("no accessor for `{field}`"),
    }
}

/// Each accessor, over a word with only bit `b` set, for every `b` of the 32, reads that bit where `table` puts it in
/// its field and nothing elsewhere: so it reads exactly its bits, and bits 10–31 decode as zero in every `sd_*`.
fn check_extract(table: &[(&str, u32, u32)]) {
    for &(field, offset, width) in table {
        for b in 0..32 {
            let expected = if (offset..offset + width).contains(&b) {
                1 << (b - offset)
            } else {
                0
            };
            assert_eq!(
                accessor(field)(1 << b),
                expected,
                "sd_{field} on a word with only bit {b} set"
            );
        }
    }
}

#[test]
fn sd_accessors_extract_exactly_their_bits() {
    check_extract(&TABLE);
}

negative_control!(
    sd_accessors_extract_exactly_their_bits,
    "`detail` read as bits 3–5 overlaps `saturated`, so the extract check must fail",
    expected = "sd_detail on a word with only bit 5 set",
    check_extract(&[("detail", 3, 3)])
);

/// `repack` returns every descriptor of bits 0–9 unchanged (with `d_min` = 1.0 in bits 16–31, a valid stored value).
fn check_all_1024(repack: impl Fn(u32) -> u32) {
    let one = 0x3c00 << 16;
    for low in 0..1024u32 {
        let w = one | low;
        assert_eq!(
            repack(w),
            w,
            "descriptor {low:#05x} does not round-trip through pack/unpack"
        );
    }
}

#[test]
fn sd_accessors_round_trip_all_1024_descriptors() {
    check_all_1024(|w| PackedA::unpack(w).pack());
}

negative_control!(
    sd_accessors_round_trip_all_1024_descriptors,
    "a repack that drops `last_symbol` loses bits 8–9, so the round trip must fail",
    expected = "descriptor 0x100 does not round-trip",
    check_all_1024(|w| set_last_symbol(PackedA::unpack(w).pack(), 0))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-012: the packed words' bit diagrams (payload §2).

/// Each `(word, packed, expected)` packs to the expected raw bits.
fn check_raw(cases: &[(&str, u32, u32)]) {
    for &(word, packed, expected) in cases {
        assert_eq!(packed, expected, "`{word}` raw bits: {packed:#010x}");
    }
}

/// Payload §2's diagrams at known values: `packed_a` = d_min 1.0 (f16 0x3c00) | state 2, detail 1, saturated,
/// dmin_pair 2, last_symbol 3 (2 + 8 + 32 + 128 + 768 = 0x3aa); `packed_b` = dLz_max 2.0 (0x4000) | dE_max 0.5
/// (0x3800); `times` = t_dmin_step 0xabcd | t_end_step 0x1234.
fn known() -> [(&'static str, u32, u32); 3] {
    [
        (
            "packed_a",
            pack_packed_a(2, 1, true, 2, 3, 1.0),
            0x3c00_03aa,
        ),
        ("packed_b", pack_packed_b(0.5, 2.0), 0x4000_3800),
        ("times", pack_times(0x1234, 0xabcd), 0xabcd_1234),
    ]
}

#[test]
fn packed_words_pack_to_payload_section_2_bits() {
    check_raw(&known());
    let [(_, a, _), (_, b, _), (_, t, _)] = known();
    assert_eq!(pa_d_min(a), 1.0, "pa_d_min");
    assert_eq!(pa_d_min(a), unpack2x16float(a)[1], "d_min is .y");
    assert_eq!(
        (pb_dE_max(b), pb_dLz_max(b)),
        (0.5, 2.0),
        "pb_dE_max, pb_dLz_max"
    );
    assert_eq!(pack2x16float([0.5, 2.0]), b, "packed_b is pack2x16float");
    assert_eq!(
        (tm_t_end_step(t), tm_t_dmin_step(t)),
        (0x1234, 0xabcd),
        "tm_t_end_step, tm_t_dmin_step"
    );
}

negative_control!(
    packed_words_pack_to_payload_section_2_bits,
    "`times` with its halves swapped is not payload §2's diagram, so the raw check must fail",
    expected = "`times` raw bits",
    check_raw(&[("times", pack_times(0xabcd, 0x1234), 0xabcd_1234)])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-014: the state codes and their predicates (payload §2, §6).

/// `(code, resolved outcome, running, failed, finished)`: 0–2 resolved; 3 running; 4–5 failed; 6–7 reserved, decoded
/// as finished and untrusted.
const CODES: [(u32, bool, bool, bool, bool); 8] = [
    (0, true, false, false, true),
    (1, true, false, false, true),
    (2, true, false, false, true),
    (3, false, true, false, false),
    (4, false, false, true, true),
    (5, false, false, true, true),
    (6, false, false, true, true),
    (7, false, false, true, true),
];

/// Each code decodes, and its predicates are as `codes` says, with every other descriptor field set.
fn check_codes(codes: &[(u32, bool, bool, bool, bool)]) {
    for &(code, resolved, running, failed, finished) in codes {
        let w = pack_packed_a(code, 3, true, 3, 3, 1.0);
        assert_eq!(sd_state(w), code, "state {code} decodes");
        let got = (
            sd_is_resolved_outcome(w),
            sd_is_running(w),
            sd_is_failed(w),
            sd_is_finished(w),
        );
        assert_eq!(
            got,
            (resolved, running, failed, finished),
            "state {code}: (resolved, running, failed, finished)"
        );
    }
}

#[test]
fn state_codes_decode_and_6_7_are_finished_and_untrusted() {
    check_codes(&CODES);
}

negative_control!(
    state_codes_decode_and_6_7_are_finished_and_untrusted,
    "reserved code 6 read as trusted breaks the forward-compatibility rule, so the check must fail",
    expected = "state 6: (resolved, running, failed, finished)",
    check_codes(&[(6, false, false, false, true)])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-015: the reserved bits 10–15.

/// Every field at its maximum packs bits 0–9 all set, `reserved` (the ledger's reserved spans) zero, and those spans
/// are bits 10–15.
fn check_reserved(w: u32, reserved: &[(u32, u32)]) {
    assert_eq!(reserved, [(10, 6)], "packed_a's reserved spans");
    assert_eq!(
        extract(w, 0, 10),
        0x3ff,
        "bits 0–9 at every field's maximum"
    );
    for &(offset, width) in reserved {
        assert_eq!(
            extract(w, offset, width),
            0,
            "reserved bits {offset}–{}",
            offset + width - 1
        );
    }
}

#[test]
fn reserved_bits_are_zero_with_every_field_at_max() {
    check_reserved(pack_packed_a(7, 3, true, 3, 3, 65504.0), &PACKED_A_RESERVED);
}

negative_control!(
    reserved_bits_are_zero_with_every_field_at_max,
    "a word with bit 12 set has reserved bits written, so the check must fail",
    expected = "reserved bits 10–15",
    check_reserved(
        pack_packed_a(7, 3, true, 3, 3, 65504.0) | 1 << 12,
        &PACKED_A_RESERVED
    )
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-018: `roundtrip_ctl` fails on a contaminated value in any bit (pitfalls §9).

/// `ctl` passes on the clean word and fails on each of its 32 single-bit flips.
fn check_flips(ctl: fn(&PackedA, u32) -> bool, expected: &PackedA) {
    let w = expected.pack();
    assert!(
        ctl(expected, w),
        "the ctl fails on the clean word {w:#010x}"
    );
    for b in 0..32 {
        assert!(
            !ctl(expected, w ^ 1 << b),
            "the ctl passed with bit {b} flipped in {w:#010x}"
        );
    }
}

/// The check pitfalls §9 records, over the unpacked fields: it cannot see a bit no accessor reads.
fn fields_ctl(expected: &PackedA, observed: u32) -> bool {
    let (a, b) = (PackedA::unpack(expected.pack()), PackedA::unpack(observed));
    (
        a.state,
        a.detail,
        a.saturated,
        a.dmin_pair,
        a.last_symbol,
        a.d_min.to_bits(),
    ) == (
        b.state,
        b.detail,
        b.saturated,
        b.dmin_pair,
        b.last_symbol,
        b.d_min.to_bits(),
    )
}

/// The recorded check itself: `from_bits` masks to bits 2–4 before comparing.
fn from_bits_ctl(expected: &PackedA, observed: u32) -> bool {
    extract(expected.pack(), 2, 3) == extract(observed, 2, 3)
}

fn sample() -> PackedA {
    PackedA {
        state: 1,
        detail: 0,
        saturated: false,
        dmin_pair: 2,
        last_symbol: 1,
        d_min: 0.25,
    }
}

#[test]
fn roundtrip_ctl_fails_on_every_flipped_bit() {
    check_flips(roundtrip_ctl, &sample());
}

negative_control!(
    roundtrip_ctl_fails_on_every_flipped_bit,
    "a field comparison cannot see the reserved bits, so the flip check must fail on bit 10",
    expected = "the ctl passed with bit 10 flipped",
    check_flips(fields_ctl, &sample())
);

#[test]
fn roundtrip_ctl_property_every_flip_of_every_descriptor_fails() {
    let fields = (0u32..8, 0u32..4, 0u32..2, 0u32..4, 0u32..4, 1u16..0x7c00);
    validation::prop::run(&fields, |(state, detail, sat, pair, sym, h)| {
        let expected = PackedA {
            state,
            detail,
            saturated: sat == 1,
            dmin_pair: pair,
            last_symbol: sym,
            d_min: f16_bits_to_f32(h),
        };
        check_flips(roundtrip_ctl, &expected);
        Ok(())
    });
}

negative_control!(
    roundtrip_ctl_property_every_flip_of_every_descriptor_fails,
    "the recorded from_bits check masks bits 0–1, so the flip check must fail on bit 0",
    expected = "the ctl passed with bit 0 flipped",
    check_flips(from_bits_ctl, &sample())
);

/// `ctl` fails on the contaminated values pitfalls §9 is about: a fork in bits 0–1, which `from_bits` masks off, and
/// one in the reserved bits 10–15, which no accessor reads.
fn check_contaminated(ctl: fn(&PackedA, u32) -> bool) {
    let clean = sample().pack();
    for contaminated in [
        clean ^ 0b01,
        clean ^ 0b10,
        clean | 1 << 10,
        clean | 0x3f << 10,
    ] {
        assert!(
            !ctl(&sample(), contaminated),
            "the ctl passed on a contaminated value {contaminated:#010x}"
        );
    }
}

#[test]
fn roundtrip_ctl_fails_where_the_recorded_check_passed() {
    let clean = sample().pack();
    assert!(
        from_bits_ctl(&sample(), clean ^ 0b01),
        "the recorded check passes on the fork in bit 0 (pitfalls §9)"
    );
    assert!(
        fields_ctl(&sample(), clean | 1 << 10),
        "a field comparison passes on a reserved-bit contamination"
    );
    check_contaminated(roundtrip_ctl);
}

negative_control!(
    roundtrip_ctl_fails_where_the_recorded_check_passed,
    "the recorded from_bits check passes on the bit-0 fork, so the contamination check must fail",
    expected = "the ctl passed on a contaminated value",
    check_contaminated(from_bits_ctl)
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-005: f16 pairs through the `pack2x16float` / `unpack2x16float` equivalents.

/// Half an f16 ULP at `x`: 2⁻¹¹·|x| rounded down to a power of two for a normal, 2⁻²⁵ for a subnormal; so within
/// f16 epsilon (2⁻¹⁰) relative, or its subnormal step.
fn half_ulp(x: f32) -> f32 {
    let normal_min = f32::from_bits(0x3880_0000); // 2⁻¹⁴
    if x.abs() < normal_min {
        f32::from_bits(0x3300_0000) // 2⁻²⁵
    } else {
        f32::from_bits((x.abs().to_bits() & 0x7f80_0000) - (11 << 23))
    }
}

/// `[x, y]` packed by `pack` and unpacked by `unpack` is within half an f16 ULP of each.
fn check_pair(pack: fn([f32; 2]) -> u32, unpack: fn(u32) -> [f32; 2], x: f32, y: f32) {
    let back = unpack(pack([x, y]));
    for (v, b) in [(x, back[0]), (y, back[1])] {
        assert!(
            (b - v).abs() <= half_ulp(v),
            "{v:e} came back {b:e}: not within f16 epsilon"
        );
    }
}

/// A finite f32 of magnitude at most 65504 (bits up to 0x477f_e000) with the sign `sign`.
fn finite(bits: u32, sign: u32) -> f32 {
    f32::from_bits(bits | sign << 31)
}

#[test]
fn f16_pairs_round_trip_within_f16_epsilon() {
    let pairs = (0u32..=0x477f_e000, 0u32..2, 0u32..=0x477f_e000, 0u32..2);
    validation::prop::run(&pairs, |(a, sa, b, sb)| {
        check_pair(pack2x16float, unpack2x16float, finite(a, sa), finite(b, sb));
        Ok(())
    });
    for x in [65504.0, -65504.0, 1.0, 0.0, 6.0e-8, 1.0e-4, 0.1] {
        check_pair(pack2x16float, unpack2x16float, x, -x);
    }
}

negative_control!(
    f16_pairs_round_trip_within_f16_epsilon,
    "a pack that scales by 1.01 is off by far more than f16 epsilon, so the pair check must fail",
    expected = "not within f16 epsilon",
    check_pair(
        |v| pack2x16float([v[0] * 1.01, v[1]]),
        unpack2x16float,
        1.0,
        1.0
    )
);

/// Every finite f16 bit pattern survives f16 → f32 → f16 through `to` and `from`, both halves.
fn check_exact(to: fn(u16) -> f32, from: fn(f32) -> u16) {
    for h in 0..=u16::MAX {
        if h & 0x7c00 == 0x7c00 {
            continue;
        }
        assert_eq!(from(to(h)), h, "f16 {h:#06x} does not round-trip");
    }
}

#[test]
fn f16_pairs_every_finite_f16_round_trips_exactly() {
    check_exact(f16_bits_to_f32, f32_to_f16_bits);
    let w = pack2x16float([f16_bits_to_f32(0x0001), f16_bits_to_f32(0xfbff)]);
    assert_eq!(w, 0xfbff_0001, "a subnormal and -65504 pack exactly");
}

negative_control!(
    f16_pairs_every_finite_f16_round_trips_exactly,
    "a conversion that flushes subnormals to zero, as a GPU may, loses 0x0001, so the exact check must fail",
    expected = "f16 0x0001 does not round-trip",
    check_exact(
        |h| if h & 0x7c00 == 0 { 0.0 } else { f16_bits_to_f32(h) },
        f32_to_f16_bits
    )
);

/// Ties round to even and the finite range ends at 65504: 1 + 2⁻¹¹ → 1, 1 + 3·2⁻¹¹ → 1 + 2⁻⁹, 65519 → 65504, and
/// 65520 → +∞ unclamped but 65504 clamped.
fn check_rounding(to_bits: fn(f32) -> u16) {
    let cases = [
        (1.0 + f32::from_bits(0x3a00_0000), 0x3c00),
        (1.0 + 3.0 * f32::from_bits(0x3a00_0000), 0x3c02),
        (65519.0, 0x7bff),
        (65520.0, 0x7c00),
        (clamp_f16(65520.0), 0x7bff),
        (clamp_f16(f32::NEG_INFINITY), 0xfbff),
    ];
    for (x, h) in cases {
        assert_eq!(to_bits(x), h, "{x} converts to {h:#06x}");
    }
}

#[test]
fn f16_pairs_round_to_nearest_even_and_clamp() {
    check_rounding(f32_to_f16_bits);
}

negative_control!(
    f16_pairs_round_to_nearest_even_and_clamp,
    "truncation rounds 1 + 3·2⁻¹¹ down, so the rounding check must fail",
    expected = "converts to 0x3c02",
    check_rounding(|x| f32_to_f16_bits(f32::from_bits(x.to_bits() & !0x1fff)))
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-092: `d_min`'s unset value and subnormal floor (R-271).

/// `d_min`'s bits in `w`.
fn d_min_bits(w: u32) -> u32 {
    w >> 16
}

/// `(case, packed_a, d_min bits)`: each word holds the expected bits.
fn check_d_min(cases: &[(&str, u32, u32)]) {
    for &(case, w, bits) in cases {
        assert_eq!(d_min_bits(w), bits, "{case}: d_min bits");
    }
}

/// A failed sample (sim_failed, written unset); an unstepped sample (running, written unset: R-271's value before the
/// first step, written by bits, not seeded as an f32 +∞); an f32 +∞ given to the packer, R-271's unset value, tested
/// by its bits; 1e-9, 0.0 and 2⁻²⁵ below the smallest subnormal; 2⁻²⁴ itself; and 1.0 and 65504 as normals.
fn d_min_cases() -> Vec<(&'static str, u32, u32)> {
    vec![
        (
            "failed",
            set_d_min_unset(pack_packed_a(4, 0, false, 3, 0, 0.5)),
            0x7c00,
        ),
        (
            "unstepped",
            set_d_min_unset(pack_packed_a(3, 0, false, 3, 0, 0.5)),
            0x7c00,
        ),
        (
            "+inf",
            pack_packed_a(3, 0, false, 3, 0, f32::INFINITY),
            0x7c00,
        ),
        ("1e-9", set_d_min(0, 1.0e-9), 0x0001),
        ("0.0", set_d_min(0, 0.0), 0x0001),
        ("2^-25", set_d_min(0, f32::from_bits(0x3300_0000)), 0x0001),
        ("2^-24", set_d_min(0, F16_MIN_SUBNORMAL), 0x0001),
        ("1.0", set_d_min(0, 1.0), 0x3c00),
        ("65504", set_d_min(0, 65504.0), 0x7bff),
        ("1e6", set_d_min(0, 1.0e6), 0x7bff),
    ]
}

#[test]
fn dmin_unset_and_subnormal_bits() {
    check_d_min(&d_min_cases());
    assert_eq!(PA_D_MIN_UNSET, 0x7c00, "the unset bits are f16 +inf");
    assert_eq!(
        PA_D_MIN_SENTINEL,
        f32::INFINITY,
        "the ledger declares d_min's sentinel as +inf"
    );
    assert_eq!(
        u32::from(f32_to_f16_bits(PA_D_MIN_SENTINEL)),
        PA_D_MIN_UNSET,
        "the sentinel's f16 bits are the unset bits"
    );
}

negative_control!(
    dmin_unset_and_subnormal_bits,
    "through the conversion alone 1e-9 rounds to 0x0000, so the d_min check must fail",
    expected = "1e-9: d_min bits",
    check_d_min(&[("1e-9", u32::from(f32_to_f16_bits(1.0e-9)) << 16, 0x0001)])
);

/// `set` stores no valid `d_min` (a non-negative finite f32, or +∞ for unset) as 0x0000.
fn check_never_zero(set: fn(u32, f32) -> u32, bits: u32) {
    let v = f32::from_bits(bits);
    assert_ne!(d_min_bits(set(0, v)), 0, "d_min {v:e} packs to 0x0000");
}

#[test]
fn dmin_unset_property_no_valid_input_packs_to_zero() {
    validation::prop::run(&(0u32..=0x7f80_0000), |bits| {
        check_never_zero(set_d_min, bits);
        Ok(())
    });
    for bits in [0, 1, 0x3300_0000, 0x3380_0000, 0x7f80_0000] {
        check_never_zero(set_d_min, bits);
    }
}

negative_control!(
    dmin_unset_property_no_valid_input_packs_to_zero,
    "the plain clamping packer stores 0.0 as 0x0000, so the check must fail",
    expected = "packs to 0x0000",
    check_never_zero(
        |w, v| insert(w, u32::from(f32_to_f16_bits(clamp_f16(v))), 16, 16),
        0
    )
);

// R-281: the packer never stores NaN (R-79) and never silently rewrites a negative value. Each is a `debug_assert!`
// failure; in release a NaN stores the unset bits and a negative value clamps to the floor, and each case increments
// its telemetry counter, `dmin_nan_unset` or `dmin_negative_floored` (R-288; telemetry §2).

/// The release packer counting into a scratch pair, as a `fn(u32, f32) -> u32` for the checks below.
fn release_scratch(w: u32, v: f32) -> u32 {
    set_d_min_release(w, v, &DminCounters::new())
}

/// NaNs of either sign, quiet and signalling, with and without payload bits.
const NANS: [u32; 5] = [
    0x7fc0_0000,
    0x7f80_0001,
    0x7fff_ffff,
    0xffc0_0000,
    0xff80_0001,
];

/// Negative values: the smallest-magnitude f32 subnormal, one below the f16 floor, −2⁻²⁴, −1, −65504, past −65504,
/// −f32::MAX and −∞.
const NEGATIVES: [u32; 8] = [
    0x8000_0001,
    0xb300_0000,
    0xb380_0000,
    0xbf80_0000,
    0xc77f_e000,
    0xd000_0000,
    0xff7f_ffff,
    0xff80_0000,
];

/// The text a caught panic carries. Only the debug-assertion tests use it.
#[cfg(debug_assertions)]
fn panic_text(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("")
}

/// `set` panics on every `d_min` whose f32 bits are in `inputs`, with a message containing `message`. Only the
/// debug-assertion tests use it.
#[cfg(debug_assertions)]
fn check_trips(set: fn(u32, f32) -> u32, inputs: &[u32], message: &str) {
    for &bits in inputs {
        let v = f32::from_bits(bits);
        match std::panic::catch_unwind(|| set(0x0000_03ff, v)) {
            Ok(w) => {
                panic!("d_min {bits:#010x} did not trip the debug assertion: stored {w:#010x}")
            }
            Err(payload) => {
                let text = panic_text(payload.as_ref());
                assert!(
                    text.contains(message),
                    "d_min {bits:#010x} tripped with {text:?}, not {message:?}"
                );
            }
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn dmin_unset_debug_nan_trips_the_assertion() {
    check_trips(set_d_min, &NANS, "`d_min` is NaN");
    check_trips(
        |w, v| pack_packed_a(sd_state(w), 0, false, 3, 0, v),
        &NANS,
        "`d_min` is NaN",
    );
}

#[cfg(debug_assertions)]
negative_control!(
    dmin_unset_debug_nan_trips_the_assertion,
    "the release path has no assertion, so the trip check must fail on NaN",
    expected = "did not trip the debug assertion",
    check_trips(release_scratch, &NANS, "`d_min` is NaN")
);

#[cfg(debug_assertions)]
#[test]
fn dmin_unset_debug_negative_trips_the_assertion() {
    check_trips(set_d_min, &NEGATIVES, "`d_min` is negative");
    check_trips(
        |w, v| pack_packed_a(sd_state(w), 0, false, 3, 0, v),
        &NEGATIVES,
        "`d_min` is negative",
    );
}

#[cfg(debug_assertions)]
negative_control!(
    dmin_unset_debug_negative_trips_the_assertion,
    "the release path has no assertion, so the trip check must fail on a negative value",
    expected = "did not trip the debug assertion",
    check_trips(release_scratch, &NEGATIVES, "`d_min` is negative")
);

/// `set` stores `want` as `d_min` for every input whose f32 bits are in `inputs`, keeping the descriptor half.
fn check_release(set: fn(u32, f32) -> u32, inputs: &[u32], want: u32) {
    for &bits in inputs {
        for low in [0u32, 0x03ff, 0xffff] {
            let w = set(0x3c00_0000 | low, f32::from_bits(bits));
            assert_eq!(d_min_bits(w), want, "d_min {bits:#010x}: d_min bits");
            assert_eq!(w & 0xffff, low, "d_min {bits:#010x}: descriptor kept");
        }
    }
}

#[test]
fn dmin_unset_release_nan_stores_the_unset_bits() {
    check_release(release_scratch, &NANS, 0x7c00);
    // −0.0 is a zero distance, not a negative value: the floor, silently (R-271).
    check_release(release_scratch, &[0x8000_0000], 0x0001);
    check_release(set_d_min, &[0x8000_0000], 0x0001);
}

negative_control!(
    dmin_unset_release_nan_stores_the_unset_bits,
    "through the conversion a NaN stays NaN, so the unset check must fail",
    expected = "d_min bits",
    check_release(
        |w, v| insert(w, u32::from(f32_to_f16_bits(clamp_f16(v))), 16, 16),
        &NANS,
        0x7c00
    )
);

#[test]
fn dmin_unset_release_negative_stores_the_floor() {
    check_release(release_scratch, &NEGATIVES, 0x0001);
}

negative_control!(
    dmin_unset_release_negative_stores_the_floor,
    "through the conversion a negative value keeps its sign, so the floor check must fail",
    expected = "d_min bits",
    check_release(
        |w, v| insert(w, u32::from(f32_to_f16_bits(clamp_f16(v))), 16, 16),
        &NEGATIVES,
        0x0001
    )
);

// R-288: the counters. `dmin_nan_unset` counts the packs that stored a NaN as unset, `dmin_negative_floored` the packs
// that clamped a negative value to the floor (telemetry §2), in release builds as well as debug ones; a valid value and
// `roundtrip_ctl`'s repack count neither (RQ-171 option (a)).

/// A counter packer: `set_d_min_release`'s signature.
type Counted = fn(u32, f32, &DminCounters) -> u32;

/// `(dmin_nan_unset, dmin_negative_floored)` of `c`.
fn counts(c: &DminCounters) -> (u32, u32) {
    (
        c.dmin_nan_unset.load(Ordering::Relaxed),
        c.dmin_negative_floored.load(Ordering::Relaxed),
    )
}

/// For each input whose f32 bits are in `inputs`, `set` over a fresh pair stores `want` as `d_min`, keeps the
/// descriptor half, and leaves the pair at `per_input`; over one shared pair, the counts add up input by input.
fn check_counts(set: Counted, inputs: &[u32], want: u32, per_input: (u32, u32)) {
    let shared = DminCounters::new();
    for (i, &bits) in inputs.iter().enumerate() {
        let v = f32::from_bits(bits);
        let fresh = DminCounters::new();
        let w = set(0x0000_03ff, v, &fresh);
        assert_eq!(d_min_bits(w), want, "d_min {bits:#010x}: d_min bits");
        assert_eq!(w & 0xffff, 0x03ff, "d_min {bits:#010x}: descriptor kept");
        assert_eq!(
            counts(&fresh),
            per_input,
            "d_min {bits:#010x}: counters (dmin_nan_unset, dmin_negative_floored)"
        );
        set(0, v, &shared);
        let n = i as u32 + 1;
        assert_eq!(
            counts(&shared),
            (per_input.0 * n, per_input.1 * n),
            "after {n} inputs: counters (dmin_nan_unset, dmin_negative_floored)"
        );
    }
}

/// The release packer storing the right bits but counting into a pair of its own, which the caller never sees (a
/// control's packer, so compiled only with them).
#[cfg(feature = "controls")]
fn uncounted(w: u32, v: f32, _: &DminCounters) -> u32 {
    release_scratch(w, v)
}

#[test]
fn dmin_unset_counter_nan_increments_dmin_nan_unset() {
    check_counts(set_d_min_release, &NANS, 0x7c00, (1, 0));
    // Release builds pass the pair through the asserting packer too (a debug build's assertion fires first).
    #[cfg(not(debug_assertions))]
    check_counts(set_d_min_counted, &NANS, 0x7c00, (1, 0));
}

negative_control!(
    dmin_unset_counter_nan_increments_dmin_nan_unset,
    "a packer that stores the unset bits but counts nothing, so the counter check must fail",
    expected = "counters (dmin_nan_unset, dmin_negative_floored)",
    check_counts(uncounted, &NANS, 0x7c00, (1, 0))
);

#[test]
fn dmin_unset_counter_negative_increments_dmin_negative_floored() {
    check_counts(set_d_min_release, &NEGATIVES, 0x0001, (0, 1));
    #[cfg(not(debug_assertions))]
    check_counts(set_d_min_counted, &NEGATIVES, 0x0001, (0, 1));
}

negative_control!(
    dmin_unset_counter_negative_increments_dmin_negative_floored,
    "a packer that stores the floor but counts a negative value as a NaN, so the counter check must fail",
    expected = "counters (dmin_nan_unset, dmin_negative_floored)",
    check_counts(
        |w, v, c| {
            c.dmin_nan_unset.fetch_add(1, Ordering::Relaxed);
            release_scratch(w, v)
        },
        &NEGATIVES,
        0x0001,
        (0, 1)
    )
);

/// `set` counts nothing for a valid `d_min`: +0.0 and −0.0, below the floor, the floor itself, normals, past 65504,
/// and +∞ (unset), each stored as its expected bits; and over a sample of every non-negative f32 up to +∞.
fn check_valid_uncounted(set: Counted) {
    let cases: [(u32, u32); 9] = [
        (0x0000_0000, 0x0001),
        (0x8000_0000, 0x0001),
        (0x3089_705f, 0x0001),
        (0x3380_0000, 0x0001),
        (0x3f80_0000, 0x3c00),
        (0x477f_e000, 0x7bff),
        (0x4974_2400, 0x7bff),
        (0x7f7f_ffff, 0x7bff),
        (0x7f80_0000, 0x7c00),
    ];
    for (bits, want) in cases {
        check_counts(set, &[bits], want, (0, 0));
    }
    let c = DminCounters::new();
    validation::prop::run(&(0u32..=0x7f80_0000), |bits| {
        set(0, f32::from_bits(bits), &c);
        Ok(())
    });
    assert_eq!(
        counts(&c),
        (0, 0),
        "valid d_min: counters (dmin_nan_unset, dmin_negative_floored)"
    );
}

#[test]
fn dmin_unset_counter_valid_values_count_nothing() {
    check_valid_uncounted(set_d_min_release);
    check_valid_uncounted(set_d_min_counted);
}

negative_control!(
    dmin_unset_counter_valid_values_count_nothing,
    "a packer that counts every store as a NaN, so the no-count check must fail",
    expected = "counters (dmin_nan_unset, dmin_negative_floored)",
    check_valid_uncounted(|w, v, c| {
        c.dmin_nan_unset.fetch_add(1, Ordering::Relaxed);
        release_scratch(w, v)
    })
);

/// `packed_a` words whose `d_min` bits are contaminated with values the packer counts when it stores them: f16 NaNs of
/// either sign, −1.0, −0 with a subnormal (the smallest negative), −65504 and −∞.
const CONTAMINATED_D_MIN: [u32; 6] = [0x7e00, 0xfe00, 0xbc00, 0x8001, 0xfbff, 0xfc00];

/// `ctl` fails on every contaminated word (its repack writes the unset bits or the floor, not the observed bits), and
/// leaves `watched` — the pair it could count into — as it found it: the repack observes, it does not store.
fn check_roundtrip_uncounted(ctl: fn(&PackedA, u32) -> bool, watched: &DminCounters) {
    let expected = PackedA::unpack(pack_packed_a(2, 1, true, 0, 3, 1.0));
    let before = counts(watched);
    for h in CONTAMINATED_D_MIN {
        let observed = h << 16 | pack_packed_a(2, 1, true, 0, 3, 1.0) & 0xffff;
        assert!(
            !ctl(&expected, observed),
            "roundtrip_ctl passed d_min bits {h:#06x}"
        );
    }
    assert_eq!(
        counts(watched),
        before,
        "roundtrip_ctl's repack: counters (dmin_nan_unset, dmin_negative_floored)"
    );
}

/// The pair the control's repack counts into, its own so the control cannot disturb the crate-level pair (compiled
/// only with the controls).
#[cfg(feature = "controls")]
static CONTROL_PAIR: DminCounters = DminCounters::new();

#[test]
fn dmin_unset_counter_roundtrip_repack_counts_nothing() {
    // No other test in this binary stores a NaN or negative `d_min` through the crate-level pair, which
    // `roundtrip_ctl`'s packing of `expected` uses.
    check_roundtrip_uncounted(roundtrip_ctl, &DMIN_COUNTERS);
}

negative_control!(
    dmin_unset_counter_roundtrip_repack_counts_nothing,
    "a parity check whose repack stores through a live pair counts each contaminated value, so the check must fail",
    expected = "roundtrip_ctl's repack: counters",
    check_roundtrip_uncounted(
        |expected, observed| {
            let d_min = PackedA::unpack(observed).d_min;
            let repacked = set_d_min_release(observed & 0xffff, d_min, &CONTROL_PAIR);
            observed == expected.pack() && repacked == observed
        },
        &CONTROL_PAIR
    )
);

/// `is_unset` holds exactly when `d_min`'s bits are 0x7c00, over all 65536 of them (descriptor bits set too).
fn check_unset_reads_bits(is_unset: fn(u32) -> bool) {
    for h in 0..=u16::MAX as u32 {
        let w = h << 16 | 0x3ff;
        assert_eq!(is_unset(w), h == 0x7c00, "is_unset on d_min bits {h:#06x}");
    }
}

#[test]
fn dmin_unset_test_reads_bits() {
    check_unset_reads_bits(pa_d_min_is_unset);
}

negative_control!(
    dmin_unset_test_reads_bits,
    "a float test for infinity also takes -inf (0xfc00) as unset, so the bits check must fail",
    expected = "is_unset on d_min bits 0xfc00",
    check_unset_reads_bits(|w| pa_d_min(w).is_infinite())
);

/// `to_bits` sends ±∞ and every value past binary16's range to ±∞ (`0x7c00`, `0xfc00`), and keeps NaN a NaN with its
/// sign and its payload's top ten bits, quieting one whose top ten bits are zero (IEEE 754 binary16; payload §1's
/// clamp is the packers', not the conversion's).
fn check_special(to_bits: fn(f32) -> u16) {
    let cases: [(&str, f32, u16); 10] = [
        ("+inf", f32::INFINITY, 0x7c00),
        ("-inf", f32::NEG_INFINITY, 0xfc00),
        ("65536", 65536.0, 0x7c00),
        ("-65536", -65536.0, 0xfc00),
        ("1e10", 1e10, 0x7c00),
        ("f32::MAX", f32::MAX, 0x7c00),
        ("quiet NaN", f32::from_bits(0x7fc0_0000), 0x7e00),
        ("-quiet NaN", f32::from_bits(0xffc0_0000), 0xfe00),
        ("NaN, all payload bits", f32::from_bits(0x7fff_ffff), 0x7fff),
        (
            "NaN, low payload bit only",
            f32::from_bits(0x7f80_0001),
            0x7e00,
        ),
    ];
    for (name, x, want) in cases {
        assert_eq!(to_bits(x), want, "binary16 bits of {name}");
    }
}

#[test]
fn f16_pairs_infinity_overflow_and_nan_bits() {
    check_special(f32_to_f16_bits);
}

negative_control!(
    f16_pairs_infinity_overflow_and_nan_bits,
    "a conversion that clamps first sends +inf to 65504 (0x7bff), not +inf, so the check must fail",
    expected = "binary16 bits of +inf",
    check_special(|x| f32_to_f16_bits(clamp_f16(x)))
);

/// `fraction(w, 0)` is 0 for any `times` word, and `fraction(w, h)` is `step / h` otherwise (payload §6's guard).
fn check_fraction(fraction: fn(u32, u32) -> f32, step: fn(u32) -> u32) {
    for w in [0, 0x0001_0002, 0xffff_ffff, 0x1234_5678] {
        assert_eq!(fraction(w, 0), 0.0, "fraction of {w:#010x} at horizon 0");
        for h in [1, 7, 65535] {
            assert_eq!(
                fraction(w, h),
                step(w) as f32 / h as f32,
                "fraction of {w:#010x} at horizon {h}"
            );
        }
    }
}

#[test]
fn packed_words_time_fractions_guard_horizon_zero() {
    check_fraction(tm_t_end_fraction, tm_t_end_step);
    check_fraction(tm_t_dmin_fraction, tm_t_dmin_step);
}

negative_control!(
    packed_words_time_fractions_guard_horizon_zero,
    "an unguarded division gives NaN or inf at horizon 0, so the check must fail",
    expected = "at horizon 0",
    check_fraction(|w, h| tm_t_dmin_step(w) as f32 / h as f32, tm_t_dmin_step)
);
