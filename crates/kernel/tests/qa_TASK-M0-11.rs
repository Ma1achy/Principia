//! QA tests for TASK-M0-11 on the kernel side, written from REQ-PAY-001 (dd_generation_root §3.6, payload §2's
//! descriptor table, R-86), payload §3's `.w` bit map and frozen continuation table (dd_simstate_payload §3,
//! dd_generation_root §3.3, R-307), not from the implementation. Every name, bit position and table value below is
//! transcribed from those sections; the word-buffer round trip follows payload §3's append and decode pseudocode.
//! Each test has a registered negative control (R-176).

use std::mem::{offset_of, size_of};

use kernel::payload::*;
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-001: ICDescriptor is 64 B, its twelve §3.6 f32 fields then 16 B of declared padding, and E₀ is not stored.
// R-313 amends §3.6: the fields follow `Real`, `_pad` is 16 B at every width, and the declared `_tail` follows it,
// empty at each row; at f32 the struct has fourteen members and is 64 B.

/// Generation-root §3.6's twelve fields, in its order.
const IC_FIELDS: [&str; 12] = [
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
];

/// The member names of a derived `Debug` rendering, `Name { a: …, b: … }`.
fn debug_members(debug: &str) -> Vec<String> {
    debug
        .split([' ', '{', ','])
        .filter_map(|t| t.strip_suffix(':'))
        .map(str::to_owned)
        .collect()
}

/// Whether a member name is a stored E₀: `E0`, `E_0`, `e0`, or any name containing "energy".
fn stores_energy(name: &str) -> bool {
    let lower = name.to_lowercase();
    (lower.starts_with('e') && lower.contains('0')) || lower.contains("energy")
}

/// The struct's members are §3.6's twelve fields in order, then exactly two more, `_pad` and `_tail`, the declared
/// padding, neither an energy (R-86, R-313); each field is a 4-byte f32 at `4·i`; `_pad` fills bytes 48..64; `_tail`
/// is empty at f32, at byte 64; and the struct is 64 B.
fn check_icdescriptor(
    members: &[String],
    offsets: &[usize],
    pad: (usize, usize),
    tail: (usize, usize),
    size: usize,
) {
    assert_eq!(size, 64, "size_of::<ICDescriptor>() is 64 B (R-86)");
    assert_eq!(
        members.len(),
        14,
        "ICDescriptor has §3.6's twelve fields and two declared padding members, `_pad` and `_tail`, nothing else: \
         {members:?}"
    );
    assert_eq!(
        &members[..12],
        IC_FIELDS.map(str::to_owned).as_slice(),
        "ICDescriptor's fields are §3.6's, in its order"
    );
    for name in &members[12..] {
        assert!(
            !stores_energy(name),
            "ICDescriptor stores E₀ as `{name}`; it is derived as K₀ + V₀ (R-86)"
        );
    }
    assert_eq!(
        &members[12..],
        ["_pad", "_tail"].map(str::to_owned).as_slice(),
        "ICDescriptor's declared padding is §3.6's `_pad` then `_tail` (R-313)"
    );
    for (i, (&o, name)) in offsets.iter().zip(IC_FIELDS).enumerate() {
        assert_eq!(o, 4 * i, "`{name}` sits at byte {}", 4 * i);
    }
    assert_eq!(
        pad,
        (48, 16),
        "the declared padding fills bytes 48..64, so no byte of the 64 is implicit"
    );
    assert_eq!(
        tail,
        (64, 0),
        "the declared `_tail` is empty at f32, at byte 64 (§3.6, R-313)"
    );
}

fn ic_offsets() -> Vec<usize> {
    vec![
        offset_of!(ICDescriptor, m0),
        offset_of!(ICDescriptor, m1),
        offset_of!(ICDescriptor, m2),
        offset_of!(ICDescriptor, q_mass),
        offset_of!(ICDescriptor, rho_mag),
        offset_of!(ICDescriptor, lambda_mag),
        offset_of!(ICDescriptor, rho_ratio),
        offset_of!(ICDescriptor, rho_angle),
        offset_of!(ICDescriptor, K_0),
        offset_of!(ICDescriptor, V_0),
        offset_of!(ICDescriptor, virial_ratio),
        offset_of!(ICDescriptor, r_min_pair_0),
    ]
}

fn ic_pad() -> (usize, usize) {
    let d = ICDescriptor::default();
    (offset_of!(ICDescriptor, _pad), size_of_val_of(&d._pad))
}

fn ic_tail() -> (usize, usize) {
    let d = ICDescriptor::default();
    (offset_of!(ICDescriptor, _tail), size_of_val_of(&d._tail))
}

fn size_of_val_of<T>(v: &T) -> usize {
    std::mem::size_of_val(v)
}

fn ic_members() -> Vec<String> {
    // Every §3.6 field is an f32: this compiles only if each is.
    let d = ICDescriptor {
        m0: 0.0f32,
        m1: 1.0f32,
        m2: 2.0f32,
        q_mass: 0.0f32,
        rho_mag: 0.0f32,
        lambda_mag: 0.0f32,
        rho_ratio: 0.0f32,
        rho_angle: 0.0f32,
        K_0: 0.5f32,
        V_0: -1.0f32,
        virial_ratio: 0.0f32,
        r_min_pair_0: 0.0f32,
        ..ICDescriptor::default()
    };
    debug_members(&format!("{d:?}"))
}

#[test]
fn qa_payload_sizes_icdescriptor_members_are_section_3_6_and_declared_padding() {
    let members = ic_members();
    println!(
        "ICDescriptor: {} B, members {members:?}",
        size_of::<ICDescriptor>()
    );
    check_icdescriptor(
        &members,
        &ic_offsets(),
        ic_pad(),
        ic_tail(),
        size_of::<ICDescriptor>(),
    );
}

negative_control!(
    qa_payload_sizes_icdescriptor_members_are_section_3_6_and_declared_padding,
    "an ICDescriptor with a stored E_0 member beside its padding must fail the member check",
    expected = "ICDescriptor has §3.6's twelve fields and two declared padding members, `_pad` and `_tail`, nothing else",
    {
        let mut members = ic_members();
        members.insert(12, "E_0".to_owned());
        check_icdescriptor(&members, &ic_offsets(), ic_pad(), ic_tail(), size_of::<ICDescriptor>())
    }
);

negative_control!(
    qa_payload_sizes_icdescriptor_e0_in_place_of_padding,
    "an ICDescriptor whose 16 B `_pad` is an `E0` member, not padding, must fail the E₀ check",
    expected = "ICDescriptor stores E₀ as `E0`",
    {
        let mut members = ic_members();
        members[12] = "E0".to_owned();
        check_icdescriptor(
            &members,
            &ic_offsets(),
            ic_pad(),
            ic_tail(),
            size_of::<ICDescriptor>(),
        )
    }
);

negative_control!(
    qa_payload_sizes_icdescriptor_implicit_tail,
    "a 12-byte padding member at 48 leaves 4 implicit bytes and must fail the padding check",
    expected = "the declared padding fills bytes 48..64",
    check_icdescriptor(&ic_members(), &ic_offsets(), (48, 12), ic_tail(), 64)
);

negative_control!(
    qa_payload_sizes_icdescriptor_e0_in_place_of_tail,
    "an ICDescriptor whose `_tail` is an `E_0` member, not padding, must fail the E₀ check",
    expected = "ICDescriptor stores E₀ as `E_0`",
    {
        let mut members = ic_members();
        members[13] = "E_0".to_owned();
        check_icdescriptor(
            &members,
            &ic_offsets(),
            ic_pad(),
            ic_tail(),
            size_of::<ICDescriptor>(),
        )
    }
);

negative_control!(
    qa_payload_sizes_icdescriptor_padding_misnamed,
    "an ICDescriptor whose second padding member is not §3.6's `_tail` must fail the name check",
    expected = "ICDescriptor's declared padding is §3.6's `_pad` then `_tail`",
    {
        let mut members = ic_members();
        members[13] = "_reserved".to_owned();
        check_icdescriptor(
            &members,
            &ic_offsets(),
            ic_pad(),
            ic_tail(),
            size_of::<ICDescriptor>(),
        )
    }
);

negative_control!(
    qa_payload_sizes_icdescriptor_nonempty_tail,
    "a 4-byte `_tail` at f32 is tail padding §3.6 says is empty there, and must fail the tail check",
    expected = "the declared `_tail` is empty at f32",
    check_icdescriptor(&ic_members(), &ic_offsets(), ic_pad(), (64, 4), 64)
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-PAY-001: descriptor bits 10–15 are reserved, "decode as zero, never opportunistically reused" (payload §2): no
// pack or setter of `packed_a` writes them, over every in-range field value and a spread of d_min values.

/// `d_min` values across binary16's range and its edges: zero, the subnormal floor, normal values, the clamp, beyond
/// it, and infinity. NaN and negative values, which the debug pack refuses (R-79, R-281), go through
/// `set_d_min_release` only.
const D_MINS: [f32; 9] = [
    0.0,
    5.9604645e-8,
    1.0e-6,
    0.001,
    1.0,
    300.0,
    65504.0,
    1.0e9,
    f32::INFINITY,
];

/// Every word `packed_a`'s pack and setters produce, from zero, over payload §2's full field ranges (state 0–7,
/// detail 0–3, saturated, dmin_pair 0–3, last_symbol 0–3) and [`D_MINS`].
fn packed_a_words() -> Vec<u32> {
    let c = DminCounters::new();
    let mut out = Vec::new();
    for state in 0..8 {
        for detail in 0..4 {
            for saturated in [false, true] {
                for dmin_pair in 0..4 {
                    for last_symbol in 0..4 {
                        for d in D_MINS {
                            out.push(pack_packed_a(
                                state,
                                detail,
                                saturated,
                                dmin_pair,
                                last_symbol,
                                d,
                                &c,
                            ));
                        }
                    }
                }
            }
        }
    }
    for v in [0, 1, 3, 7, 0xff, u32::MAX] {
        out.push(set_state(0, v));
        out.push(set_detail(0, v));
        out.push(set_dmin_pair(0, v));
        out.push(set_last_symbol(0, v));
        out.push(set_last_symbol(set_dmin_pair(0, v), v));
    }
    out.push(set_saturated(0, true));
    for d in D_MINS {
        out.push(set_d_min(0, d, &c));
        out.push(set_d_min_release(0, d, &c));
    }
    for d in [f32::NAN, -1.0, -0.0, f32::NEG_INFINITY] {
        out.push(set_d_min_release(0, d, &c));
    }
    out.push(set_d_min_unset(0));
    out
}

fn check_reserved_zero(words: &[u32]) {
    for &w in words {
        assert_eq!(
            (w >> 10) & 0x3f,
            0,
            "descriptor bits 10–15 are written: {w:#034b}"
        );
    }
}

#[test]
fn qa_payload_sizes_descriptor_bits_10_to_15_never_written() {
    let words = packed_a_words();
    println!("{} packed_a words checked", words.len());
    check_reserved_zero(&words);
}

negative_control!(
    qa_payload_sizes_descriptor_bits_10_to_15_never_written,
    "a packer that spends bit 15 must fail the reserved-bits check",
    expected = "descriptor bits 10–15 are written",
    check_reserved_zero(
        &packed_a_words()
            .into_iter()
            .map(|w| w | 1 << 15)
            .collect::<Vec<_>>()
    )
);

negative_control!(
    qa_payload_sizes_descriptor_bit_10,
    "a packer that spends bit 10, the span's first, must fail the reserved-bits check",
    expected = "descriptor bits 10–15 are written",
    check_reserved_zero(&[pack_packed_a(1, 2, true, 3, 3, 1.0, &DminCounters::new()) | 1 << 10])
);

// ---------------------------------------------------------------------------------------------------------------
// Payload §3's `.w` bit map: `length` in `.w` bits 25–31, 0…76 valid, 127 the truncation sentinel, and
// `fgw_retained_prefix_length = select(length_raw, 76, truncated)`. The capacity is 1 + ⌊(121 − 2)/log₂3⌋.

/// A small xorshift, so the payload bits vary without a dev-dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn u32(&mut self) -> u32 {
        (self.next() >> 32) as u32
    }
}

type Reader = fn([u32; 4]) -> (u32, bool, u32);

fn read(w: [u32; 4]) -> (u32, bool, u32) {
    (
        fgw_length_raw(w),
        fgw_truncated(w),
        fgw_retained_prefix_length(w),
    )
}

/// For every 7-bit `length` and payload bits of zero, all ones and random, the accessors read payload §3's values.
fn check_fgw(read: Reader) {
    let capacity = 1 + ((121.0 - 2.0) / 3f64.log2()).floor() as u32;
    assert_eq!(
        (FGW_CAPACITY, FGW_LENGTH_SENTINEL),
        (capacity, 127),
        "capacity 1 + ⌊119/log₂3⌋ and sentinel 127 (payload §3)"
    );
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for length in 0..128u32 {
        let mut payloads = vec![[0u32; 4], [u32::MAX, u32::MAX, u32::MAX, 0x01ff_ffff]];
        for _ in 0..16 {
            payloads.push([rng.u32(), rng.u32(), rng.u32(), rng.u32() & 0x01ff_ffff]);
        }
        let expected = (
            length,
            length == 127,
            if length == 127 { 76 } else { length },
        );
        for p in payloads {
            let w = [p[0], p[1], p[2], length << 25 | p[3]];
            assert_eq!(
                read(w),
                expected,
                "length {length} over payload {p:08x?} reads (length_raw, truncated, prefix)"
            );
        }
    }
}

#[test]
fn qa_fgw_accessors_every_length_over_any_payload() {
    check_fgw(read);
}

negative_control!(
    qa_fgw_accessors_every_length_over_any_payload,
    "a length read from bits 24–30 of .w must fail the accessor check",
    expected = "reads (length_raw, truncated, prefix)",
    check_fgw(|w| read([w[0], w[1], w[2], w[3] << 1]))
);

negative_control!(
    qa_fgw_accessors_sentinel_as_count,
    "a retained prefix that exposes 127 as a crossing count must fail the accessor check",
    expected = "length 127 over payload",
    check_fgw(|w| {
        let (raw, t, _) = read(w);
        (raw, t, raw)
    })
);

// ---------------------------------------------------------------------------------------------------------------
// Payload §3's frozen continuation table, from its letter table (prev → digits 0, 1, 2; the excluded inverse).

/// Symbol codes `a = 0, A = 1, b = 2, B = 3` (payload §3).
fn code(c: char) -> u32 {
    match c {
        'a' => 0,
        'A' => 1,
        'b' => 2,
        'B' => 3,
        _ => unreachable!("{c}"),
    }
}

/// Payload §3's table rows: `(prev, [digit 0, digit 1, digit 2], excluded)`.
const LETTER_TABLE: [(char, [char; 3], char); 4] = [
    ('a', ['a', 'b', 'B'], 'A'),
    ('A', ['A', 'B', 'b'], 'a'),
    ('b', ['b', 'a', 'A'], 'B'),
    ('B', ['B', 'A', 'a'], 'b'),
];

/// The tables a check reads: `(inverse, cont_symbol, predecessor_symbol, continuation_index)` as functions.
#[derive(Clone, Copy)]
struct Tables {
    inverse: fn(u32) -> u32,
    cont: fn(u32, u32) -> u32,
    pred: fn(u32, u32) -> u32,
    index: fn(u32, u32) -> u32,
}

/// The four tables as arrays: `(INVERSE, CONT_SYMBOL, PREDECESSOR_SYMBOL, CONTINUATION_INDEX)`.
type Arrays = ([u32; 4], [[u32; 4]; 3], [[u32; 4]; 3], [[u32; 4]; 4]);

const GENERATED: Tables = Tables {
    inverse,
    cont: continuation_symbol,
    pred: predecessor_symbol,
    index: continuation_index,
};

/// Every cell of the four tables equals payload §3's letter table, with `continuation_index` its inversion and 3 in
/// the four `next = inverse(prev)` cells (R-307); and the arrays equal the functions.
fn check_against_letter_table(t: Tables, arrays: Arrays) {
    for (prev, digits, excluded) in LETTER_TABLE {
        let p = code(prev);
        assert_eq!((t.inverse)(p), code(excluded), "inverse({prev})");
        for (e, next) in digits.iter().enumerate() {
            let n = code(*next);
            assert_eq!((t.cont)(p, e as u32), n, "cont_symbol[{e}][{prev}]");
            assert_eq!((t.pred)(n, e as u32), p, "predecessor_symbol[{e}][{next}]");
            assert_eq!(
                (t.index)(p, n),
                e as u32,
                "continuation_index[{prev}][{next}]"
            );
        }
        assert_eq!(
            (t.index)(p, code(excluded)),
            3,
            "continuation_index[{prev}][{excluded}] holds 3, the inverse cell (R-307)"
        );
    }
    let (inv, cont, pred, index) = arrays;
    for s in 0..4u32 {
        assert_eq!(inv[s as usize], (t.inverse)(s), "INVERSE[{s}]");
        for e in 0..3u32 {
            assert_eq!(
                cont[e as usize][s as usize],
                (t.cont)(s, e),
                "CONT_SYMBOL[{e}][{s}]"
            );
            assert_eq!(
                pred[e as usize][s as usize],
                (t.pred)(s, e),
                "PREDECESSOR_SYMBOL[{e}][{s}]"
            );
        }
        for n in 0..4u32 {
            assert_eq!(
                index[s as usize][n as usize],
                (t.index)(s, n),
                "CONTINUATION_INDEX[{s}][{n}]"
            );
        }
    }
}

#[test]
fn qa_continuation_table_rust_equals_payload_section_3_letter_table() {
    check_against_letter_table(
        GENERATED,
        (INVERSE, CONT_SYMBOL, PREDECESSOR_SYMBOL, CONTINUATION_INDEX),
    );
}

negative_control!(
    qa_continuation_table_rust_equals_payload_section_3_letter_table,
    "continuation_index reading 0 in the inverse cells, not R-307's 3, must fail the letter-table check",
    expected = "holds 3, the inverse cell (R-307)",
    check_against_letter_table(
        Tables {
            index: |p, n| if n == inverse(p) { 0 } else { continuation_index(p, n) },
            ..GENERATED
        },
        (INVERSE, CONT_SYMBOL, PREDECESSOR_SYMBOL, CONTINUATION_INDEX),
    )
);

negative_control!(
    qa_continuation_table_array_differs_from_function,
    "a CONTINUATION_INDEX array that differs from the function in one cell must fail",
    expected = "CONTINUATION_INDEX[2][3]",
    check_against_letter_table(
        GENERATED,
        (INVERSE, CONT_SYMBOL, PREDECESSOR_SYMBOL, {
            let mut i = CONTINUATION_INDEX;
            i[2][3] = 2;
            i
        })
    )
);

/// REQ-PAY-016's properties, from the statement: each digit's map is a permutation that never gives `inverse(prev)`,
/// the three digits cover the three legal continuations, each map is an involution so `predecessor_symbol ==
/// cont_symbol`, and `inverse` is itself an involution with no fixed point.
fn check_properties(t: Tables) {
    for s in 0..4u32 {
        assert_ne!((t.inverse)(s), s, "inverse({s}) is not {s}");
        assert_eq!(
            (t.inverse)((t.inverse)(s)),
            s,
            "inverse is an involution at {s}"
        );
    }
    for e in 0..3u32 {
        let mut image = [
            (t.cont)(0, e),
            (t.cont)(1, e),
            (t.cont)(2, e),
            (t.cont)(3, e),
        ];
        image.sort();
        assert_eq!(image, [0, 1, 2, 3], "digit {e}'s map is a permutation");
        for s in 0..4u32 {
            assert_eq!(
                (t.cont)((t.cont)(s, e), e),
                s,
                "digit {e}'s map is an involution at {s}"
            );
            assert_eq!(
                (t.pred)(s, e),
                (t.cont)(s, e),
                "predecessor_symbol == cont_symbol at [{e}][{s}]"
            );
        }
    }
    for prev in 0..4u32 {
        let mut legal: Vec<u32> = (0..3).map(|e| (t.cont)(prev, e)).collect();
        legal.sort();
        let mut want: Vec<u32> = (0..4).filter(|&s| s != (t.inverse)(prev)).collect();
        want.sort();
        assert_eq!(
            legal, want,
            "the three digits of {prev} cover its three legal continuations"
        );
    }
}

#[test]
fn qa_continuation_table_rust_properties() {
    check_properties(GENERATED);
}

negative_control!(
    qa_continuation_table_rust_properties,
    "a digit-2 map that sends every symbol to its inverse (still an involutive permutation) must fail the property check",
    expected = "cover its three legal continuations",
    check_properties(Tables {
        cont: |p, e| if e == 2 { INVERSE_LETTERS[p as usize] } else { continuation_symbol(p, e) },
        pred: |n, e| if e == 2 { INVERSE_LETTERS[n as usize] } else { predecessor_symbol(n, e) },
        ..GENERATED
    })
);

// ---------------------------------------------------------------------------------------------------------------
// End to end: payload §3's append (push, pop, truncate at the cap) and decode, over the generated tables and the
// `.w` layout, recover the freely-reduced word a reference list keeps; `W` never exceeds 121 bits.

/// The word as payload §3 holds it: `W` over x, y, z (low 96 bits) and `.w[0:24]`, `length` in `.w[25:31]`.
fn to_vec4(w: u128, length_raw: u32) -> [u32; 4] {
    assert!(w < 1u128 << 121, "W exceeds the 121-bit payload: {w:#x}");
    [
        w as u32,
        (w >> 32) as u32,
        (w >> 64) as u32,
        ((w >> 96) as u32 & 0x01ff_ffff) | length_raw << 25,
    ]
}

/// The generator's state across appends: `W`, `prev` and `length_raw`.
struct Word {
    w: u128,
    prev: u32,
    length_raw: u32,
}

/// Payload §3's append, verbatim.
fn append(word: &mut Word, s: u32, t: Tables) {
    if word.length_raw == 127 {
    } else if word.length_raw == 0 {
        word.w = s as u128;
        word.prev = s;
        word.length_raw = 1;
    } else if s == (t.inverse)(word.prev) {
        if word.length_raw == 1 {
            word.w = 0;
            word.prev = 4;
            word.length_raw = 0;
        } else {
            let e_last = (word.w % 3) as u32;
            word.w /= 3;
            word.prev = (t.pred)(word.prev, e_last);
            word.length_raw -= 1;
        }
    } else if word.length_raw == 76 {
        word.length_raw = 127;
    } else {
        let e = (t.index)(word.prev, s);
        assert!(
            e < 3,
            "continuation_index({}, {s}) is no digit: {e}",
            word.prev
        );
        word.w = 3 * word.w + e as u128;
        word.prev = s;
        word.length_raw += 1;
    }
}

/// Payload §3's decode: pop `length − 1` base-3 digits, the residue is `d₀`, replay forward.
fn decode(v: [u32; 4], t: Tables) -> Vec<u32> {
    let length = fgw_retained_prefix_length(v);
    if length == 0 {
        return vec![];
    }
    let mut w = v[0] as u128
        | (v[1] as u128) << 32
        | (v[2] as u128) << 64
        | ((v[3] & 0x01ff_ffff) as u128) << 96;
    let mut digits = Vec::new();
    for _ in 0..length - 1 {
        digits.push((w % 3) as u32);
        w /= 3;
    }
    let mut s = w as u32;
    let mut out = vec![s];
    for &e in digits.iter().rev() {
        s = (t.cont)(s, e);
        out.push(s);
    }
    out
}

/// Random crossing streams, biased to cancel so pops and pop-to-empty occur, and long enough to reach the cap: the
/// decoded word equals the freely-reduced reference until truncation, `prev` is its last symbol, and after the cap the
/// word is truncated with its 76-symbol prefix kept.
fn check_round_trip(t: Tables) {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let (mut pops, mut truncations, mut empties) = (0, 0, 0);
    for stream in 0..400 {
        let mut word = Word {
            w: 0,
            prev: 4,
            length_raw: 0,
        };
        let mut reference: Vec<u32> = Vec::new();
        let mut truncated_prefix: Option<Vec<u32>> = None;
        let steps = if stream % 4 == 0 { 400 } else { 60 };
        for _ in 0..steps {
            let r = rng.next();
            let s = match (reference.last(), r % 5) {
                (Some(&p), 0 | 1) if stream % 2 == 1 => INVERSE_LETTERS[p as usize],
                _ => ((r >> 8) % 4) as u32,
            };
            if truncated_prefix.is_none() {
                let reduces = reference
                    .last()
                    .is_some_and(|&p| INVERSE_LETTERS[p as usize] == s);
                if reduces {
                    reference.pop();
                    pops += 1;
                    if reference.is_empty() {
                        empties += 1;
                    }
                } else if reference.len() == 76 {
                    truncated_prefix = Some(reference.clone());
                    truncations += 1;
                } else {
                    reference.push(s);
                }
            }
            append(&mut word, s, t);
            let v = to_vec4(word.w, word.length_raw);
            match &truncated_prefix {
                None => {
                    assert_eq!(
                        fgw_length_raw(v),
                        reference.len() as u32,
                        "length after append"
                    );
                    assert!(!fgw_truncated(v), "truncated before the cap");
                    assert_eq!(
                        decode(v, t),
                        reference,
                        "the decoded word is the freely-reduced word"
                    );
                    if let Some(&last) = reference.last() {
                        assert_eq!(
                            word.prev, last,
                            "prev after a pop is the reduced word's last symbol"
                        );
                    }
                }
                Some(prefix) => {
                    assert!(fgw_truncated(v), "a push at 76 symbols truncates");
                    assert_eq!(
                        fgw_retained_prefix_length(v),
                        76,
                        "the retained prefix is 76"
                    );
                    assert_eq!(
                        &decode(v, t),
                        prefix,
                        "the truncated word keeps its 76-symbol prefix"
                    );
                }
            }
        }
    }
    println!("round trip: {pops} pops, {empties} to empty, {truncations} truncations");
    assert!(
        pops > 1000 && empties > 10 && truncations > 10,
        "the streams reach pops, pop-to-empty and the cap"
    );
}

/// `inverse` for the reference, from payload §3's letter table's excluded column, independent of the generated one.
const INVERSE_LETTERS: [u32; 4] = [1, 0, 3, 2];

#[test]
fn qa_continuation_table_rust_word_round_trip() {
    check_round_trip(GENERATED);
}

negative_control!(
    qa_continuation_table_rust_word_round_trip,
    "a predecessor table that is not cont_symbol's inverse (digits 1 and 2 swapped) must fail the round trip",
    expected = "prev after a pop is the reduced word's last symbol",
    check_round_trip(Tables {
        pred: |n, e| predecessor_symbol(n, [0, 2, 1][e as usize]),
        ..GENERATED
    })
);
