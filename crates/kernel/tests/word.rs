//! The word buffer's append and decode (`kernel::word`) and the generated word accessors' Rust target
//! (`kernel::payload`), against payload §3's rules computed here independently: the mixed-radix `W` by its Horner
//! recurrence in u128, and free reduction by a stack (dd_simstate_payload §3, §6; dd_generation_root §3.3, §3.3a).
//! - REQ-PAY-023: random freely-reduced words up to 76 symbols pack as payload §3 lays them out and decode back; a push
//!   at 76 symbols truncates the word to length 127, `fgw_truncated`, the retained prefix 76 (`word_roundtrip`).
//! - REQ-PAY-029: `decode(append(s))` is the freely reduced `s` for random streams up to 76 symbols, and for fixed ones
//!   (`word_decode_reduced`); a decode replaying a corrupted continuation-table entry, or one dropping a base-3 digit,
//!   fails it (pitfalls §9).
//! - REQ-PAY-028, REQ-PAY-033 on the Rust target: the truncated fixture word (`fixtures/words/truncated.txt`) and the
//!   accessors at lengths 0, 1, 76 and 127; `ledger/tests/word_accessors.rs` holds the WGSL target to the same.
//! - The two targets' `fgw_symbol` and accessors agree on the GPU (`word_accessors_rust_and_wgsl_agree`).
//!
//! Each check takes the append or the decode as a function, so its control runs the same check on one with a fault.

use std::mem::{align_of, size_of};

use kernel::payload::{
    continuation_symbol, fgw_div3, fgw_length_raw, fgw_reduced_length, fgw_reduced_length_valid,
    fgw_retained_prefix_length, fgw_symbol, fgw_truncated, sd_last_symbol, sd_last_symbol_valid,
    FreeGroupWord, SimStateBase, SimStateFTLE, FGW_CAPACITY, FGW_LENGTH_SENTINEL, FGW_NO_SYMBOL,
    INVERSE,
};
use kernel::word::{fgw_append, fgw_decode, fgw_mul3_add, FgwAppended};
use proptest::prelude::*;
use validation::gpu::GpuHarness;
use validation::{negative_control, prop};

/// An append: [`fgw_append`], or a control's faulty one.
type Append = fn([u32; 4], u32, u32, u32) -> FgwAppended;

/// A decode: [`fgw_decode`] collected, or a control's faulty one.
type Decode = fn([u32; 4]) -> Vec<u32>;

/// [`fgw_decode`] collected, at most one symbol past the capacity, so a decode that never ends fails rather than
/// hangs.
fn decode(w: [u32; 4]) -> Vec<u32> {
    fgw_decode(w).take(FGW_CAPACITY as usize + 1).collect()
}

/// `packed_a`'s bits outside `last_symbol` (bits 8–9) the appends start from, so a check sees they are kept.
const PACKED_A: u32 = 0xa5c3_0c3a;

/// The empty word, `prev` none, and [`PACKED_A`], each symbol of `stream` appended in turn.
fn append_all(append: Append, stream: &[u32]) -> FgwAppended {
    stream.iter().fold(
        FgwAppended {
            word: [0; 4],
            prev: FGW_NO_SYMBOL,
            packed_a: PACKED_A,
        },
        |a, &s| append(a.word, a.prev, a.packed_a, s),
    )
}

/// `stream` freely reduced by a stack, as payload §3 defines it: a symbol after its inverse cancels both; a push onto
/// 76 symbols truncates, after which every symbol is ignored. The reduced symbols and whether it truncated.
fn reduce(stream: &[u32]) -> (Vec<u32>, bool) {
    let mut stack: Vec<u32> = Vec::new();
    for &s in stream {
        if stack.last().is_some_and(|&p| s == INVERSE[p as usize]) {
            stack.pop();
        } else if stack.len() == 76 {
            return (stack, true);
        } else {
            stack.push(s);
        }
    }
    (stack, false)
}

/// The digit of payload §3's frozen table that continues `prev` with `next`: `cont_symbol[e][prev] == next`.
fn digit(prev: u32, next: u32) -> u32 {
    let table = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];
    (0..3)
        .find(|&e| table[e as usize][prev as usize] == next)
        .expect("a reduced word never continues a symbol with its inverse")
}

/// The word payload §3 packs `symbols` (freely reduced) into: `W = d₀`, then `W = 3W + e_k`, in u128; its low 96 bits in
/// `x`, `y`, `z`, its high bits in `.w` bits 0–24, `length` in bits 25–31.
fn packed(symbols: &[u32], length: u32) -> [u32; 4] {
    let w = symbols
        .windows(2)
        .fold(symbols.first().map_or(0u128, |&d| d.into()), |w, p| {
            3 * w + u128::from(digit(p[0], p[1]))
        });
    assert!(w < 1 << 121, "W fits 121 bits");
    [
        w as u32,
        (w >> 32) as u32,
        (w >> 64) as u32,
        (w >> 96) as u32 | (length << 25),
    ]
}

/// A freely-reduced word from `raw`: its first value the first symbol, each next value mod 3 the digit continuing the
/// symbol before.
fn reduced_word(raw: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for &r in raw {
        let s = match out.last() {
            None => r & 3,
            Some(&p) => continuation_symbol(p, r % 3),
        };
        out.push(s);
    }
    out
}

// ── word_roundtrip (REQ-PAY-023) ─────────────────────────────────────────────────────────────────────────────────────

/// Each freely-reduced word in `words` (up to 76 symbols), appended from empty, is payload §3's packing of it, reads its
/// length through every accessor, decodes back, and gives each symbol by `fgw_symbol`; `prev` is its last symbol and
/// `last_symbol` mirrors it, every other bit of `packed_a` kept.
fn check_roundtrip(append: Append, decode: Decode, words: &[Vec<u32>]) {
    for symbols in words {
        let n = symbols.len() as u32;
        let a = append_all(append, symbols);
        let w = a.word;
        assert_eq!(
            w,
            packed(symbols, n),
            "{symbols:?} packs otherwise than payload §3"
        );
        assert_eq!(
            (
                fgw_length_raw(w),
                fgw_truncated(w),
                fgw_retained_prefix_length(w)
            ),
            (n, false, n),
            "{symbols:?}: length, truncated, retained prefix"
        );
        assert_eq!(
            (fgw_reduced_length(w), fgw_reduced_length_valid(w)),
            (n, true),
            "{symbols:?}: the reduced crossing count"
        );
        assert_eq!(decode(w), *symbols, "{symbols:?} does not decode back");
        let by_k: Vec<u32> = (0..n).map(|k| fgw_symbol(w, k)).collect();
        assert_eq!(by_k, *symbols, "fgw_symbol of {symbols:?}");
        assert_eq!(
            (fgw_symbol(w, n), fgw_symbol(w, n + 7)),
            (FGW_NO_SYMBOL, FGW_NO_SYMBOL),
            "fgw_symbol past {symbols:?}"
        );
        assert_eq!(
            a.packed_a & !(3 << 8),
            PACKED_A & !(3 << 8),
            "packed_a's other bits"
        );
        if let Some(&last) = symbols.last() {
            assert_eq!(
                (a.prev, sd_last_symbol(a.packed_a), sd_last_symbol_valid(n)),
                (last, last, true),
                "{symbols:?}: prev and its last_symbol mirror"
            );
        }
    }
}

/// Words of up to 76 symbols, each from a raw draw ([`reduced_word`]).
fn reduced_words() -> impl Strategy<Value = Vec<Vec<u32>>> {
    proptest::collection::vec(proptest::collection::vec(any::<u32>(), 0..=76), 1..8)
        .prop_map(|raws| raws.iter().map(|r| reduced_word(r)).collect())
}

#[test]
fn word_roundtrip_random_reduced_words() {
    prop::run(&reduced_words(), |words| {
        check_roundtrip(fgw_append, decode, &words);
        Ok(())
    });
}

/// A decode that pops one base-3 digit too few, so `d₀` is read off a larger residue.
#[cfg(feature = "controls")]
fn decode_short(w: [u32; 4]) -> Vec<u32> {
    let mut d = decode(w);
    if let Some(first) = d.first_mut() {
        let mut v = kernel::payload::fgw_mixed_radix(w);
        for _ in 2..fgw_retained_prefix_length(w) {
            v = fgw_div3(v).0;
        }
        *first = v[0];
    }
    d
}

negative_control!(
    word_roundtrip_random_reduced_words,
    "a decode that pops one digit too few misreads d₀",
    expected = "does not decode back",
    check_roundtrip(fgw_append, decode_short, &[vec![1, 2, 1, 3, 0]])
);

/// Each 76-symbol word of `words`, with its next symbol `s` (not the inverse of its last): the push truncates it to the
/// sentinel 127, `fgw_truncated`, its retained prefix 76 and every word-derived validity false, its 76 symbols kept and
/// decoding as before, `prev` and `last_symbol` frozen; then every crossing, a cancelling one included, is ignored.
fn check_truncation(append: Append, words: &[(Vec<u32>, u32)]) {
    for (symbols, s) in words {
        assert_eq!(symbols.len(), 76, "a full word");
        let full = append_all(append, symbols);
        let cut = append(full.word, full.prev, full.packed_a, *s);
        let w = cut.word;
        assert_eq!(
            (
                fgw_length_raw(w),
                fgw_truncated(w),
                fgw_retained_prefix_length(w)
            ),
            (FGW_LENGTH_SENTINEL, true, FGW_CAPACITY),
            "a push at 76 symbols is not the truncation: {symbols:?} then {s}"
        );
        assert_eq!(
            (FGW_LENGTH_SENTINEL, FGW_CAPACITY),
            (127, 76),
            "the sentinel and cap"
        );
        assert_eq!(w, packed(symbols, 127), "the truncated word keeps W");
        assert!(
            !fgw_reduced_length_valid(w) && !sd_last_symbol_valid(fgw_length_raw(w)),
            "a truncated word's reduced length and last symbol read valid"
        );
        assert_eq!(decode(w), *symbols, "the retained prefix decodes");
        assert_eq!(
            (cut.prev, cut.packed_a),
            (full.prev, full.packed_a),
            "the truncating push moved prev or last_symbol"
        );
        for later in [INVERSE[full.prev as usize], *s, 0, 3] {
            assert_eq!(
                append(w, cut.prev, cut.packed_a, later),
                cut,
                "a crossing after truncation is not ignored"
            );
        }
    }
}

/// 76-symbol words, each with a next symbol that is not its last's inverse.
fn full_words() -> impl Strategy<Value = Vec<(Vec<u32>, u32)>> {
    proptest::collection::vec(
        (proptest::collection::vec(any::<u32>(), 76), any::<u32>()),
        1..4,
    )
    .prop_map(|draws| {
        draws
            .iter()
            .map(|(raw, r)| {
                let symbols = reduced_word(raw);
                let s = continuation_symbol(symbols[75], r % 3);
                (symbols, s)
            })
            .collect()
    })
}

#[test]
fn word_roundtrip_a_push_at_the_cap_truncates_to_127() {
    prop::run(&full_words(), |words| {
        check_truncation(fgw_append, &words);
        Ok(())
    });
}

/// An append whose cap is one symbol late: it pushes a 77th.
#[cfg(feature = "controls")]
fn append_late_cap(word: [u32; 4], prev: u32, packed_a: u32, s: u32) -> FgwAppended {
    if fgw_length_raw(word) == 76 && s != INVERSE[prev as usize] {
        let v = fgw_mul3_add(kernel::payload::fgw_mixed_radix(word), digit(prev, s));
        let mut w = v;
        w[3] = (v[3] & 0x01ff_ffff) | (77 << 25);
        return FgwAppended {
            word: w,
            prev: s,
            packed_a: kernel::word::set_last_symbol(packed_a, s),
        };
    }
    fgw_append(word, prev, packed_a, s)
}

negative_control!(
    word_roundtrip_a_push_at_the_cap_truncates_to_127,
    "an append that pushes a 77th symbol does not truncate",
    expected = "a push at 76 symbols is not the truncation",
    check_truncation(append_late_cap, &[(reduced_word(&[0; 76]), 2)])
);

/// `fgw_mul3_add` is `3v + e` and `fgw_div3` undoes it, over all four limbs, against u128 arithmetic.
fn check_mul3_div3(mul: fn([u32; 4], u32) -> [u32; 4], cases: &[([u32; 4], u32)]) {
    let wide = |v: [u32; 4]| {
        v.iter()
            .rev()
            .fold(0u128, |w, &l| (w << 32) | u128::from(l))
    };
    for &(v, e) in cases {
        let got = mul(v, e);
        assert_eq!(
            wide(got),
            3 * wide(v) + u128::from(e),
            "3·{v:x?} + {e} is not {got:x?}"
        );
        assert_eq!(fgw_div3(got), (v, e), "div3 does not undo mul3_add");
    }
}

#[test]
fn word_roundtrip_mul3_add_and_div3() {
    let limbs = (any::<[u32; 3]>(), 0u32..1 << 30, 0u32..3);
    prop::run(&proptest::collection::vec(limbs, 1..16), |cases| {
        let cases: Vec<([u32; 4], u32)> = cases
            .iter()
            .map(|&(l, top, e)| ([l[0], l[1], l[2], top], e))
            .collect();
        check_mul3_div3(fgw_mul3_add, &cases);
        Ok(())
    });
}

negative_control!(
    word_roundtrip_mul3_add_and_div3,
    "a mul3_add that drops the carry between limbs must fail",
    expected = "is not",
    check_mul3_div3(
        |v, e| {
            let mut out = fgw_mul3_add([v[0], 0, 0, 0], e);
            for (o, l) in out.iter_mut().zip(v).skip(1) {
                *o = l.wrapping_mul(3);
            }
            out
        },
        &[([u32::MAX, 0, 0, 0], 0)]
    )
);

// ── word_decode_reduced (REQ-PAY-029) ─────────────────────────────────────────────────────────────────────────────────

/// Each stream of `streams` (any symbols, up to 76), appended from empty, decodes as its free reduction ([`reduce`]),
/// its length that reduction's; `prev` and `last_symbol` are its last symbol.
fn check_decode_reduced(decode: Decode, streams: &[Vec<u32>]) {
    for stream in streams {
        let (want, truncated) = reduce(stream);
        assert!(!truncated, "a stream of {} truncated", stream.len());
        let a = append_all(fgw_append, stream);
        assert_eq!(
            decode(a.word),
            want,
            "decode(append(s)) is not the free reduction of s = {stream:?}"
        );
        assert_eq!(
            fgw_length_raw(a.word),
            want.len() as u32,
            "the reduced length of {stream:?}"
        );
        if let Some(&last) = want.last() {
            assert_eq!(
                (a.prev, sd_last_symbol(a.packed_a)),
                (last, last),
                "prev and last_symbol after {stream:?}"
            );
        }
    }
}

/// Streams of up to 76 symbols, cancelling ones weighted in: each symbol is the inverse of the one before it a third
/// of the time.
fn streams() -> impl Strategy<Value = Vec<Vec<u32>>> {
    proptest::collection::vec(proptest::collection::vec((0u32..4, 0u32..3), 0..=76), 1..8).prop_map(
        |draws| {
            draws
                .iter()
                .map(|d| {
                    let mut out: Vec<u32> = Vec::new();
                    for &(s, c) in d {
                        let s = match out.last() {
                            Some(&p) if c == 0 => INVERSE[p as usize],
                            _ => s,
                        };
                        out.push(s);
                    }
                    out
                })
                .collect()
        },
    )
}

#[test]
fn word_decode_reduced_random_streams() {
    prop::run(&streams(), |streams| {
        check_decode_reduced(decode, &streams);
        Ok(())
    });
}

/// A decode replaying through payload §3's table with `cont_symbol[1][a]` corrupted from `b` to `B`.
#[cfg(feature = "controls")]
fn decode_corrupt_table(w: [u32; 4]) -> Vec<u32> {
    replay(
        w,
        |prev, e| {
            if (prev, e) == (0, 1) {
                3
            } else {
                continuation_symbol(prev, e)
            }
        },
        0,
        3,
    )
}

/// Payload §3's decode written out with `cont` as the continuation table, popping `skip` digits too few, and `d₀` the
/// residue `& mask` (3 masks it; `u32::MAX` leaves it unmasked); each continuation replays from the symbol masked.
#[cfg(feature = "controls")]
fn replay(w: [u32; 4], cont: fn(u32, u32) -> u32, skip: u32, mask: u32) -> Vec<u32> {
    let length = fgw_retained_prefix_length(w);
    let mut v = kernel::payload::fgw_mixed_radix(w);
    let mut digits = Vec::new();
    for _ in (1 + skip)..length {
        let (q, e) = fgw_div3(v);
        v = q;
        digits.push(e);
    }
    let mut out = Vec::new();
    if length > 0 {
        out.push(v[0] & mask);
    }
    for (k, &e) in digits.iter().rev().enumerate() {
        out.push(cont(out[k] & 3, e));
    }
    out
}

negative_control!(
    word_decode_reduced_random_streams,
    "a decode through a continuation table with one corrupted entry must fail",
    expected = "is not the free reduction",
    check_decode_reduced(decode_corrupt_table, &[vec![0, 2, 3, 2]])
);

/// Fixed streams: the empty one, a cancellation to empty, cancellations at the head and inside, every pair of
/// symbols, a 76-symbol word, and 38 symbols followed by their inverses in reverse, 76 that reduce to nothing.
fn fixed_streams() -> Vec<Vec<u32>> {
    let mut out = vec![
        vec![],
        vec![0, 1],
        vec![2, 3, 3, 2, 0],
        vec![0, 2, 1, 0, 3, 2, 2, 1],
        reduced_word(&(0..76).collect::<Vec<u32>>()),
        (0..38)
            .map(|k| k % 4)
            .chain((0..38).rev().map(|k| INVERSE[k % 4]))
            .collect(),
    ];
    for a in 0..4 {
        for b in 0..4 {
            out.push(vec![a, b]);
        }
    }
    out
}

#[test]
fn word_decode_reduced_fixed_streams() {
    check_decode_reduced(decode, &fixed_streams());
}

negative_control!(
    word_decode_reduced_fixed_streams,
    "a decode that drops a base-3 digit must fail",
    expected = "is not the free reduction",
    check_decode_reduced(|w| replay(w, continuation_symbol, 1, 3), &fixed_streams())
);

// ── fgw_decode and fgw_symbol agree on every word (R-321) ─────────────────────────────────────────────────────────────

/// [`fgw_decode`] collected, at most one symbol past `w`'s retained prefix, so a decode that never ends fails rather
/// than hangs.
fn decode_any(w: [u32; 4]) -> Vec<u32> {
    fgw_decode(w)
        .take(fgw_retained_prefix_length(w) as usize + 1)
        .collect()
}

/// Each word of `words`, any `[u32; 4]` (its `d₀` above 3 included, which `fgw_append` never writes), decodes to
/// `fgw_symbol` at each `k` of its retained prefix: the symbol values are total and masked (R-321).
fn check_decode_agrees(decode: Decode, words: &[[u32; 4]]) {
    for &w in words {
        let by_k: Vec<u32> = (0..fgw_retained_prefix_length(w))
            .map(|k| fgw_symbol(w, k))
            .collect();
        assert_eq!(
            decode(w),
            by_k,
            "fgw_decode and fgw_symbol differ on {w:x?}"
        );
    }
}

/// `W` with `length` in `.w` bits 25–31.
fn with_length(w: [u32; 4], length: u32) -> [u32; 4] {
    [w[0], w[1], w[2], (w[3] & ((1 << 25) - 1)) | (length << 25)]
}

/// Words whose `d₀` is out of range: `W = 5` at length 1, `W = 12` (`d₀ = 4`, [`FGW_NO_SYMBOL`]) at length 2, and
/// `W = 5·3⁶⁹` (`d₀ = 5`) at length 70.
fn out_of_range_words() -> Vec<[u32; 4]> {
    let w = (0..69).fold(5u128, |w, _| 3 * w);
    vec![
        with_length([5, 0, 0, 0], 1),
        with_length([12, 0, 0, 0], 2),
        with_length(
            [
                w as u32,
                (w >> 32) as u32,
                (w >> 64) as u32,
                (w >> 96) as u32,
            ],
            70,
        ),
    ]
}

/// Any `[u32; 4]`, every bit of `W` arbitrary, at any `length_raw`, 0…127: payload §3's 0…76 and 127, truncated, and
/// the 77…126 it leaves unused, which the decode reads as `fgw_symbol` does (R-321).
fn any_words() -> impl Strategy<Value = Vec<[u32; 4]>> {
    proptest::collection::vec((any::<[u32; 4]>(), 0u32..=127), 1..16)
        .prop_map(|draws| draws.iter().map(|&(w, n)| with_length(w, n)).collect())
}

#[test]
fn word_decode_agrees_with_fgw_symbol_on_any_word() {
    prop::run(&any_words(), |words| {
        check_decode_agrees(decode_any, &[out_of_range_words(), words].concat());
        Ok(())
    });
}

negative_control!(
    word_decode_agrees_with_fgw_symbol_on_any_word,
    "a decode that leaves d₀ unmasked must fail",
    expected = "fgw_decode and fgw_symbol differ",
    check_decode_agrees(
        |w| replay(w, continuation_symbol, 0, u32::MAX),
        &out_of_range_words()
    )
);

/// A decode that pushes all `length − 1` popped digits onto a 128-bit integer, which wraps past 80 of them, as
/// [`fgw_decode`] would without its cap at 77 popped digits.
#[cfg(feature = "controls")]
fn decode_wrapping(w: [u32; 4]) -> Vec<u32> {
    let length = fgw_retained_prefix_length(w);
    let m = kernel::payload::fgw_mixed_radix(w);
    let mut v = m
        .iter()
        .rev()
        .fold(0u128, |v, &l| (v << 32) | u128::from(l));
    let mut digits = 0u128;
    for _ in 1..length {
        digits = digits.wrapping_mul(3).wrapping_add(v % 3);
        v /= 3;
    }
    let mut out: Vec<u32> = Vec::new();
    if length > 0 {
        out.push(v as u32 & 3);
    }
    for _ in 1..length {
        out.push(continuation_symbol(out[out.len() - 1], (digits % 3) as u32));
        digits /= 3;
    }
    out
}

/// Fixed limbs, `W` all ones (2¹²¹ − 1, its top digit at position 76) and a mixed pattern, at every `length_raw` past
/// the cap, 77…127, where more digits are read than `W` can make nonzero.
fn long_words() -> Vec<[u32; 4]> {
    let limbs = [
        [u32::MAX; 4],
        [0x19a9_5bab, 0x2ee0_ae18, 0x90e0_5326, 0xd579_6857],
    ];
    limbs
        .iter()
        .flat_map(|&w| (77..=127).map(move |n| with_length(w, n)))
        .collect()
}

#[test]
fn word_decode_agrees_with_fgw_symbol_past_the_cap() {
    check_decode_agrees(decode_any, &long_words());
}

negative_control!(
    word_decode_agrees_with_fgw_symbol_past_the_cap,
    "a decode whose popped digits overflow at a length_raw of 77…126 must fail",
    expected = "fgw_decode and fgw_symbol differ",
    check_decode_agrees(decode_wrapping, &long_words())
);

// ── The truncated fixture word and the accessors at 0, 1, 76, 127 (REQ-PAY-028, REQ-PAY-033) ─────────────────────────

/// `fixtures/words/truncated.txt`: the stream, the packed word and the retained prefix.
struct Fixture {
    stream: Vec<u32>,
    word: [u32; 4],
    prefix: Vec<u32>,
}

fn fixture() -> Fixture {
    let text = include_str!("../../../fixtures/words/truncated.txt");
    let field = |key: &str| -> Vec<&str> {
        text.lines()
            .find_map(|l| l.strip_prefix(key)?.trim_start().strip_prefix('='))
            .unwrap_or_else(|| panic!("the fixture has no `{key}`"))
            .split_whitespace()
            .collect()
    };
    let symbols = |key: &str| -> Vec<u32> {
        field(key)
            .iter()
            .map(|s| {
                ["a", "A", "b", "B"]
                    .iter()
                    .position(|c| c == s)
                    .expect("a symbol") as u32
            })
            .collect()
    };
    let word: Vec<u32> = field("word")
        .iter()
        .map(|h| u32::from_str_radix(h.trim_start_matches("0x"), 16).expect("hex"))
        .collect();
    Fixture {
        stream: symbols("stream"),
        word: word.try_into().expect("four u32s"),
        prefix: symbols("prefix"),
    }
}

/// The fixture's stream appends to its word; the word reads truncated, every word-derived validity false, its
/// retained prefix 76, the prefix decoding and read by `fgw_symbol` as the fixture's.
fn check_truncated_fixture(append: Append, f: &Fixture) {
    let a = append_all(append, &f.stream);
    assert_eq!(
        a.word, f.word,
        "the fixture's stream does not append to its word"
    );
    let w = f.word;
    assert!(fgw_truncated(w), "the fixture word is not truncated");
    assert!(
        !fgw_reduced_length_valid(w),
        "fgw_reduced_length_valid is true"
    );
    assert!(
        !sd_last_symbol_valid(fgw_length_raw(w)),
        "sd_last_symbol_valid is true"
    );
    assert_eq!(
        fgw_retained_prefix_length(w),
        76,
        "the retained prefix length"
    );
    assert_eq!(decode(w), f.prefix, "the retained prefix");
    let by_k: Vec<u32> = (0..80).map(|k| fgw_symbol(w, k)).collect();
    let mut want = f.prefix.clone();
    want.extend([FGW_NO_SYMBOL; 4]);
    assert_eq!(by_k, want, "fgw_symbol of the retained prefix");
}

#[test]
fn truncated_word_validity_rust_fixture() {
    check_truncated_fixture(fgw_append, &fixture());
}

negative_control!(
    truncated_word_validity_rust_fixture,
    "an append whose cap is one symbol late does not truncate the fixture's stream",
    expected = "does not append to its word",
    check_truncated_fixture(append_late_cap, &fixture())
);

/// A word of `length_raw` `length`, its symbols `a b a b …` where it has any.
fn word_of_length(length: u32) -> [u32; 4] {
    if length == 127 {
        return fixture().word;
    }
    append_all(
        fgw_append,
        &(0..length).map(|k| 2 * (k % 2)).collect::<Vec<u32>>(),
    )
    .word
}

/// The accessors at `length_raw` 0, 1, 76 and 127: `(length_raw, truncated, reduced valid, reduced, retained prefix,
/// last symbol valid)`.
const AT_LENGTHS: [(u32, [u32; 6]); 4] = [
    (0, [0, 0, 1, 0, 0, 0]),
    (1, [1, 0, 1, 1, 1, 1]),
    (76, [76, 0, 1, 76, 76, 1]),
    (127, [127, 1, 0, 127, 76, 0]),
];

/// A read of the six accessors [`AT_LENGTHS`] lists, a bool as 0 or 1.
type Accessors = fn([u32; 4]) -> [u32; 6];

fn accessors(w: [u32; 4]) -> [u32; 6] {
    let n = fgw_length_raw(w);
    [
        n,
        fgw_truncated(w).into(),
        fgw_reduced_length_valid(w).into(),
        fgw_reduced_length(w),
        fgw_retained_prefix_length(w),
        sd_last_symbol_valid(n).into(),
    ]
}

fn check_at_lengths(read: Accessors) {
    for (length, want) in AT_LENGTHS {
        assert_eq!(
            read(word_of_length(length)),
            want,
            "the accessors at length {length}"
        );
    }
}

#[test]
fn word_accessors_rust_at_lengths_0_1_76_127() {
    check_at_lengths(accessors);
}

negative_control!(
    word_accessors_rust_at_lengths_0_1_76_127,
    "a retained prefix that exposes 127 must fail",
    expected = "the accessors at length 127",
    check_at_lengths(|w| {
        let mut r = accessors(w);
        r[4] = fgw_length_raw(w);
        r
    })
);

// ── The word buffer's element (REQ-PAY-025, Rust) ─────────────────────────────────────────────────────────────────────

/// The element is `size` bytes aligned to `align`, and no `SimState` variant's fields, by their derived `Debug`
/// (`debugs`), name the word.
fn check_split(size: usize, align: usize, debugs: &[String]) {
    assert_eq!(
        (size, align),
        (16, 16),
        "the word buffer's element is one vec4<u32>, 16 B"
    );
    for d in debugs {
        assert!(
            !d.contains("word"),
            "a SimState variant holds the word, which lives in its own buffer: {d}"
        );
    }
}

#[test]
fn word_buffer_split_rust_element_and_simstate() {
    check_split(
        size_of::<FreeGroupWord>(),
        align_of::<FreeGroupWord>(),
        &[
            format!("{:?}", SimStateFTLE::default()),
            format!("{:?}", SimStateBase::default()),
        ],
    );
}

negative_control!(
    word_buffer_split_rust_element_and_simstate,
    "a SimState that holds the word must fail",
    expected = "a SimState variant holds the word",
    check_split(
        16,
        16,
        &["SimStateFTLE { free_group_word: [0, 0, 0, 0] }".to_owned()]
    )
);

// ── The two targets agree (GPU) ───────────────────────────────────────────────────────────────────────────────────────

const LAYER_WGSL: &str = include_str!("../../render/frag/generated/payload_unpack.wgsl");
const ENTRY_WGSL: &str = include_str!("../../ledger/tests/word_entry.wgsl");

/// The selector `word_entry.wgsl` reads: the word's index and what to read of it.
fn sel(case: usize, what: u32) -> u32 {
    ((case as u32) << 8) | what
}

/// What the entry reads for `what` through the Rust target: [`accessors`]' six, then `fgw_symbol(w, what − 6)`.
fn rust_read(w: [u32; 4], what: u32) -> u32 {
    match what {
        0..=5 => accessors(w)[what as usize],
        k => fgw_symbol(w, k - 6),
    }
}

/// Each word of `words`, read on the GPU through `layer` (the generated unpack layer) for each of its six accessors and
/// `fgw_symbol` at each `k` up to past its retained prefix, equals the Rust target's read.
fn check_agree(gpu: &GpuHarness, layer: &str, words: &[[u32; 4]], seed: u64) {
    let module = format!("{layer}\n{ENTRY_WGSL}");
    let mut sels = Vec::new();
    for (c, w) in words.iter().enumerate() {
        let past = fgw_retained_prefix_length(*w) + 2;
        sels.extend((0..6 + past).map(|what| sel(c, what)));
    }
    let flat: Vec<u32> = words.iter().flatten().copied().collect();
    let got = gpu.run_wgsl(&module, "t_word_read", &[&sels, &flat]);
    for (&s, &g) in sels.iter().zip(&got) {
        let (w, what) = (words[(s >> 8) as usize], s & 0xff);
        assert_eq!(
            g,
            rust_read(w, what),
            "WGSL and Rust differ reading {what} of {w:x?} (seed {seed})"
        );
    }
}

/// Random words, freely reduced and of every length up to 76, then the fixture's truncated word and the empty word.
fn agree_words(seed: u64) -> Vec<[u32; 4]> {
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        (z ^ (z >> 31)) as u32
    };
    let mut out: Vec<[u32; 4]> = (0..48)
        .map(|_| {
            let n = next() % 77;
            let raw: Vec<u32> = (0..n).map(|_| next()).collect();
            append_all(fgw_append, &reduced_word(&raw)).word
        })
        .collect();
    out.extend([fixture().word, [0; 4]]);
    out
}

#[test]
fn word_accessors_rust_and_wgsl_agree() {
    let gpu = GpuHarness::new().expect("a GPU device");
    let seed = prop::seed();
    check_agree(&gpu, LAYER_WGSL, &agree_words(seed), seed);
}

negative_control!(
    word_accessors_rust_and_wgsl_agree,
    "a WGSL fgw_symbol composing from the wrong identity must fail",
    expected = "WGSL and Rust differ",
    {
        let from = "var perm = 0xe4u;";
        assert_eq!(
            LAYER_WGSL.matches(from).count(),
            1,
            "the layer has one `{from}`"
        );
        check_agree(
            &GpuHarness::new().expect("a GPU device"),
            &LAYER_WGSL.replacen(from, "var perm = 0xe1u;", 1),
            &agree_words(1),
            1,
        )
    }
);
