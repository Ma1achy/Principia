//! QA tests for TASK-M1-02, the word buffer's append, decode and Rust word accessors, written from the requirements
//! and payload §3, §5, §6 (dd_simstate_payload), not from the implementation. Every expectation comes from a reference
//! model written here from the corpus alone:
//! - the frozen table transcribed from payload §3 (`inverse = [1,0,3,2]`, `cont_symbol[e][prev]`), symbol codes
//!   `a = 0, A = 1, b = 2, B = 3`;
//! - `W` built by payload §3's Horner recurrence `W₁ = d₀, W_{k+1} = 3·W_k + e_k` in a u128, laid out as payload §3's
//!   `.w` bit map says (low 96 bits in x, y, z; the next 25 in `.w[0:24]`; length in `.w[25:31]`);
//! - payload §3's append (empty, pop, cap, push, and ignore once truncated) over a symbol vector.
//!
//! - REQ-PAY-023 (`word_roundtrip_qa_*`): random reduced words of 0…76 symbols pack bit-for-bit as the Horner reference
//!   and decode back; 76 symbols need all 121 bits and 77 cannot fit; truncation is the sentinel 127 alone (no other
//!   length and no payload bit reads as truncated); the cap, not `W`'s size, truncates; the retained prefix clamps
//!   127 → 76.
//! - REQ-PAY-029 (`word_decode_reduced_qa_*`): `decode(append(s))` is the free reduction of `s` for random streams of
//!   up to 76 symbols, cancellation-heavy; the `last_symbol` cache in `packed_a` follows `prev` and leaves every other
//!   bit alone.
//! - REQ-PAY-028 (`truncated_word_validity_qa_*`): a truncated word reads `fgw_reduced_length_valid` false,
//!   `sd_last_symbol_valid` false, retained prefix 76, and no length accessor but the raw one returns 127; the
//!   checked-in fixture `fixtures/words/truncated.txt` is recomputed from its stream by the reference.
//! - REQ-PAY-033 (`word_accessor_names_qa_*`): the Rust accessors at lengths 0, 1, 76 and 127, and the reduced
//!   crossing count equals `fgw_reduced_length` when valid.
//! - REQ-PAY-025 (`word_buffer_split_qa_*`): the stored Rust word element is exactly 16 B; the stored `SimState`
//!   variants are 144 / 96 B, aligned to 8, not 16 (payload §0).
//!
//! Each check takes the implementation under test as function pointers; its negative control (R-176) runs the same
//! check on a variant with one fault.

use kernel::payload::*;
use kernel::word::{fgw_append, fgw_decode, FgwAppended};
use proptest::prelude::*;
use validation::{negative_control, prop};

// ── The reference model, from payload §3 ──────────────────────────────────────────────────────────────────────────

/// Payload §3's frozen `inverse`.
const INV: [u32; 4] = [1, 0, 3, 2];
/// Payload §3's frozen `cont_symbol[e][prev]`.
const CONT: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];
/// Payload §3's capacity and sentinel.
const CAP: usize = 76;
const SENTINEL: u32 = 127;

/// The digit `e` with `CONT[e][prev] == next`.
fn ref_digit(prev: u32, next: u32) -> u32 {
    (0..3u32)
        .find(|&e| CONT[e as usize][prev as usize] == next)
        .expect("next is a legal continuation of prev")
}

/// `W` of a freely-reduced word by the Horner recurrence.
fn ref_w(word: &[u32]) -> u128 {
    let mut w: u128 = 0;
    for (k, &s) in word.iter().enumerate() {
        w = if k == 0 {
            s as u128
        } else {
            3 * w + ref_digit(word[k - 1], s) as u128
        };
    }
    w
}

/// The packed `vec4<u32>` of `W` and a length field, per payload §3's bit map.
fn ref_layout(w: u128, length: u32) -> [u32; 4] {
    assert!(w < 1u128 << 121, "W exceeds the 121-bit budget");
    [
        w as u32,
        (w >> 32) as u32,
        (w >> 64) as u32,
        ((w >> 96) as u32 & 0x01ff_ffff) | (length << 25),
    ]
}

fn ref_pack(word: &[u32]) -> [u32; 4] {
    ref_layout(ref_w(word), word.len() as u32)
}

/// Free reduction of a stream of crossings, no cap.
fn ref_reduce(stream: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for &s in stream {
        if out.last() == Some(&INV[s as usize]) {
            out.pop();
        } else {
            out.push(s);
        }
    }
    out
}

/// Payload §3's append over a stream, with the cap: the retained symbols and whether truncated.
fn ref_append(stream: &[u32]) -> (Vec<u32>, bool) {
    let mut out: Vec<u32> = Vec::new();
    let mut truncated = false;
    for &s in stream {
        if truncated {
            continue;
        }
        if out.last() == Some(&INV[s as usize]) {
            out.pop();
        } else if out.len() == CAP {
            truncated = true;
        } else {
            out.push(s);
        }
    }
    (out, truncated)
}

/// The packed word the reference append leaves.
fn ref_word(stream: &[u32]) -> [u32; 4] {
    let (word, truncated) = ref_append(stream);
    let packed = ref_pack(&word);
    if truncated {
        [
            packed[0],
            packed[1],
            packed[2],
            (packed[3] & 0x01ff_ffff) | (SENTINEL << 25),
        ]
    } else {
        packed
    }
}

// ── The implementation under test, as function pointers ───────────────────────────────────────────────────────────

type Append = fn([u32; 4], u32, u32, u32) -> FgwAppended;
type Decode = fn([u32; 4]) -> Vec<u32>;

fn decode(w: [u32; 4]) -> Vec<u32> {
    fgw_decode(w).collect()
}

/// Runs `append` over `stream` from the empty word with `packed_a`, checking after every crossing that the cache in
/// `packed_a` bits 8–9 holds the last retained symbol and every other bit of `packed_a` is untouched.
fn run(append: Append, stream: &[u32], packed_a: u32) -> FgwAppended {
    let mut st = FgwAppended {
        word: [0; 4],
        prev: 0,
        packed_a,
    };
    for (i, &s) in stream.iter().enumerate() {
        st = append(st.word, st.prev, st.packed_a, s);
        let (want, truncated) = ref_append(&stream[..=i]);
        assert_eq!(
            st.packed_a & !(0b11 << 8),
            packed_a & !(0b11 << 8),
            "the append changed packed_a outside last_symbol (payload §3, §6)"
        );
        if let Some(&last) = want.last() {
            if !truncated {
                assert_eq!(
                    st.prev, last,
                    "prev is not the last symbol after crossing {i}"
                );
                assert_eq!(
                    (st.packed_a >> 8) & 3,
                    last,
                    "last_symbol does not mirror prev after crossing {i} (payload §3)"
                );
            }
        }
    }
    st
}

// ── Strategies ────────────────────────────────────────────────────────────────────────────────────────────────────

/// A freely-reduced word: a length weighted to the edges 0, 1, 75, 76, a first symbol, then digits.
fn reduced_word() -> impl Strategy<Value = Vec<u32>> {
    let len = prop_oneof![Just(0usize), Just(1), Just(75), Just(76), 0usize..=76];
    (len, 0u32..4, proptest::collection::vec(0u32..3, 75)).prop_map(|(len, d0, es)| {
        let mut w = Vec::with_capacity(len);
        for k in 0..len {
            let s = if k == 0 {
                d0
            } else {
                CONT[es[k - 1] as usize][w[k - 1] as usize]
            };
            w.push(s);
        }
        w
    })
}

/// A raw crossing stream of up to 76 symbols, cancellation-heavy: each crossing is, by a coin, the inverse of the one
/// before it or any symbol.
fn stream(max: usize) -> impl Strategy<Value = Vec<u32>> {
    proptest::collection::vec((any::<bool>(), 0u32..4), 0..=max).prop_map(|draws| {
        let mut out: Vec<u32> = Vec::with_capacity(draws.len());
        for (cancel, s) in draws {
            let s = match out.last() {
                Some(&p) if cancel => INV[p as usize],
                _ => s,
            };
            out.push(s);
        }
        out
    })
}

// ── REQ-PAY-023 ───────────────────────────────────────────────────────────────────────────────────────────────────

fn check_roundtrip(
    append: Append,
    dec: Decode,
    word: &[u32],
    packed_a: u32,
) -> Result<(), TestCaseError> {
    let st = run(append, word, packed_a);
    let want = ref_pack(word);
    prop_assert_eq!(
        st.word,
        want,
        "the packed word is not payload §3's Horner W for {:?}",
        word
    );
    prop_assert_eq!(
        dec(st.word),
        word.to_vec(),
        "decode(append(word)) is not the word"
    );
    prop_assert_eq!(
        dec(want),
        word.to_vec(),
        "decode of the reference-packed word is not the word"
    );
    prop_assert_eq!(fgw_length_raw(st.word), word.len() as u32);
    prop_assert!(
        !fgw_truncated(st.word),
        "a word of {} symbols reads truncated",
        word.len()
    );
    prop_assert_eq!(fgw_retained_prefix_length(st.word), word.len() as u32);
    for (k, &s) in word.iter().enumerate() {
        prop_assert_eq!(fgw_symbol(st.word, k as u32), s, "fgw_symbol at {}", k);
    }
    Ok(())
}

#[test]
fn word_roundtrip_qa_random_reduced_words() {
    prop::run(&(reduced_word(), any::<u32>()), |(w, pa)| {
        check_roundtrip(fgw_append, decode, &w, pa)
    });
}

/// `decode` that drops the last base-3 digit: the word less its last symbol.
#[cfg(feature = "controls")]
fn decode_dropping_a_digit(w: [u32; 4]) -> Vec<u32> {
    let mut out = decode(w);
    if out.len() > 1 {
        let last = out.len() - 1;
        out[last] = out[last - 1];
    }
    out
}

negative_control!(
    word_roundtrip_qa_random_reduced_words,
    "a decode that replays the last digit as 0 (drops it) must fail the round trip",
    expected = "decode(append(word)) is not the word",
    prop::run(&(reduced_word(), any::<u32>()), |(w, pa)| {
        check_roundtrip(fgw_append, decode_dropping_a_digit, &w, pa)
    })
);

/// 76 symbols need all 121 bits (the largest 76-symbol `W` sets bit 120, `.w` bit 24) and 77 cannot fit; that
/// largest word round-trips, and `.w` bit 24 set does not read as truncated (no flag bit).
fn check_capacity(append: Append, dec: Decode) {
    let largest = 4u128 * 3u128.pow(75) - 1;
    assert!(largest < 1u128 << 121, "76 symbols do not fit 121 bits");
    assert!(largest >= 1u128 << 120, "76 symbols do not reach bit 120");
    // The largest 77-symbol W, 4·3^76 − 1, reaches 2^121.
    assert!(
        4u128 * 3u128.pow(76) > 1u128 << 121,
        "77 symbols fit 121 bits"
    );
    assert_eq!(
        1 + (119.0 / 3f64.log2()).floor() as usize,
        CAP,
        "payload §3's capacity formula"
    );
    // d₀ = B, then digit 2 throughout: B a b A B a … (CONT[2]).
    let mut word = vec![3u32];
    while word.len() < CAP {
        word.push(CONT[2][*word.last().unwrap() as usize]);
    }
    assert_eq!(ref_w(&word), largest);
    let st = run(append, &word, 0);
    assert_eq!(
        st.word,
        ref_pack(&word),
        "the largest 76-symbol word packs wrong"
    );
    assert_eq!(st.word[3] >> 24 & 1, 1, ".w bit 24 is payload and set here");
    assert!(
        !fgw_truncated(st.word),
        "a full 76-symbol word with .w bit 24 set reads truncated"
    );
    assert!(fgw_reduced_length_valid(st.word));
    assert_eq!(
        dec(st.word),
        word,
        "the largest 76-symbol word does not decode"
    );
}

#[test]
fn word_roundtrip_qa_capacity_needs_all_121_bits() {
    check_capacity(fgw_append, decode);
}

/// An append that keeps only 120 bits of W: a 121-bit budget cut by one.
#[cfg(feature = "controls")]
fn append_120_bits(word: [u32; 4], prev: u32, pa: u32, s: u32) -> FgwAppended {
    let mut st = fgw_append(word, prev, pa, s);
    st.word[3] &= !(1 << 24);
    st
}

negative_control!(
    word_roundtrip_qa_capacity_needs_all_121_bits,
    "an append that loses .w bit 24 must fail",
    expected = "the largest 76-symbol word packs wrong",
    check_capacity(append_120_bits, decode)
);

/// Truncation is the sentinel alone: for every raw length 0…127, under any payload, `fgw_truncated` is true only at
/// 127, `fgw_length_raw` returns the raw field, and the retained prefix is the raw length, 127 clamped to 76.
fn check_sentinel(truncated: fn([u32; 4]) -> bool, payload: [u32; 4]) -> Result<(), TestCaseError> {
    for raw in 0..=127u32 {
        let w = [
            payload[0],
            payload[1],
            payload[2],
            (payload[3] & 0x01ff_ffff) | (raw << 25),
        ];
        prop_assert_eq!(fgw_length_raw(w), raw);
        prop_assert_eq!(
            truncated(w),
            raw == 127,
            "fgw_truncated is wrong at length_raw {}",
            raw
        );
        prop_assert_eq!(fgw_reduced_length_valid(w), raw != 127);
        prop_assert_eq!(
            fgw_retained_prefix_length(w),
            if raw == 127 { 76 } else { raw }
        );
    }
    Ok(())
}

#[test]
fn word_roundtrip_qa_truncation_is_the_sentinel_alone() {
    prop::run(&any::<[u32; 4]>(), |p| check_sentinel(fgw_truncated, p));
}

negative_control!(
    word_roundtrip_qa_truncation_is_the_sentinel_alone,
    "a truncation read as length > 76 must fail",
    expected = "fgw_truncated is wrong at length_raw",
    prop::run(&any::<[u32; 4]>(), |p| check_sentinel(
        |w| fgw_length_raw(w) > 76,
        p
    ))
);

/// The cap, not `W`'s size, truncates: 76 `a`s pack `W = 0`, and a 77th `a` sets the sentinel, keeping `W`; every
/// later crossing, the cancelling `A` among them, is ignored and leaves the word, `prev` and `packed_a` as they were.
/// A pop at 76 symbols is a pop, not a truncation.
fn check_cap(append: Append) {
    let a76 = vec![0u32; CAP];
    let st = run(append, &a76, 0xdead_0000);
    assert_eq!(
        st.word,
        [0, 0, 0, 76 << 25],
        "76 a's are not W = 0 at length 76"
    );
    let t = append(st.word, st.prev, st.packed_a, 0);
    assert_eq!(
        t.word,
        [0, 0, 0, SENTINEL << 25],
        "a push at the cap does not set the sentinel with W kept"
    );
    for s in [1u32, 0, 2, 3] {
        let u = append(t.word, t.prev, t.packed_a, s);
        assert_eq!(
            u, t,
            "a crossing after truncation ({s}) changed the word, prev or packed_a"
        );
    }
    // A full word, then the inverse of its last symbol: a pop to 75, not truncation.
    let mut word: Vec<u32> = vec![2];
    while word.len() < CAP {
        word.push(CONT[1][*word.last().unwrap() as usize]);
    }
    let st = run(append, &word, 0);
    let last = *word.last().unwrap();
    let p = append(st.word, st.prev, st.packed_a, INV[last as usize]);
    assert_eq!(
        p.word,
        ref_pack(&word[..CAP - 1]),
        "the inverse at 76 symbols is not a pop"
    );
    assert_eq!(p.prev, word[CAP - 2]);
}

#[test]
fn word_roundtrip_qa_cap_not_magnitude() {
    check_cap(fgw_append);
}

/// An append that truncates on the inverse at the cap (cap tested before free reduction).
#[cfg(feature = "controls")]
fn append_cap_first(word: [u32; 4], prev: u32, pa: u32, s: u32) -> FgwAppended {
    if fgw_length_raw(word) == 76 {
        return FgwAppended {
            word: [
                word[0],
                word[1],
                word[2],
                (word[3] & 0x01ff_ffff) | (127 << 25),
            ],
            prev,
            packed_a: pa,
        };
    }
    fgw_append(word, prev, pa, s)
}

negative_control!(
    word_roundtrip_qa_cap_not_magnitude,
    "an append that checks the cap before the free reduction must fail",
    expected = "the inverse at 76 symbols is not a pop",
    check_cap(append_cap_first)
);

// ── REQ-PAY-029 ───────────────────────────────────────────────────────────────────────────────────────────────────

fn check_decode_reduced(
    append: Append,
    dec: Decode,
    s: &[u32],
    pa: u32,
) -> Result<(), TestCaseError> {
    let st = run(append, s, pa);
    let want = ref_reduce(s);
    prop_assert_eq!(
        dec(st.word),
        want.clone(),
        "decode(append(s)) is not the free reduction of {:?}",
        s
    );
    prop_assert_eq!(
        st.word,
        ref_pack(&want),
        "append(s) is not the Horner W of the reduced s"
    );
    prop_assert_eq!(fgw_length_raw(st.word), want.len() as u32);
    Ok(())
}

#[test]
fn word_decode_reduced_qa_random_streams() {
    prop::run(&(stream(76), any::<u32>()), |(s, pa)| {
        check_decode_reduced(fgw_append, decode, &s, pa)
    });
}

/// An append whose continuation index for `prev = a` swaps digits 1 and 2 (a corrupted table cell).
#[cfg(feature = "controls")]
fn append_corrupt_table(word: [u32; 4], prev: u32, pa: u32, s: u32) -> FgwAppended {
    let st = fgw_append(word, prev, pa, s);
    let len = fgw_length_raw(word);
    if (1..76).contains(&len) && prev == 0 && (s == 2 || s == 3) {
        let other = if s == 2 { 3 } else { 2 };
        let alt = fgw_append(word, prev, pa, other);
        return FgwAppended {
            word: alt.word,
            prev: st.prev,
            packed_a: st.packed_a,
        };
    }
    st
}

negative_control!(
    word_decode_reduced_qa_random_streams,
    "an append with a corrupted continuation-table cell must fail",
    expected = "decode(append(s)) is not the free reduction",
    prop::run(&(stream(76), any::<u32>()), |(s, pa)| {
        check_decode_reduced(append_corrupt_table, decode, &s, pa)
    })
);

/// Longer streams, past the cap: the append matches the reference append, truncation and all, and decode gives the
/// retained prefix.
fn check_long(append: Append, s: &[u32]) -> Result<(), TestCaseError> {
    let st = run(append, s, 0);
    prop_assert_eq!(
        st.word,
        ref_word(s),
        "the append over a long stream is not payload §3's"
    );
    let (kept, _) = ref_append(s);
    prop_assert_eq!(decode(st.word), kept);
    Ok(())
}

#[test]
fn word_decode_reduced_qa_long_streams_with_cap() {
    // Biased away from cancellation so the cap is reached: any symbol, the inverse never forced.
    let s = proptest::collection::vec(0u32..4, 0..=300);
    prop::run(&s, |s| check_long(fgw_append, &s));
}

/// An append that drops a push at the cap without setting the sentinel (the cap ignored as truncation).
#[cfg(feature = "controls")]
fn append_no_cap(word: [u32; 4], prev: u32, pa: u32, s: u32) -> FgwAppended {
    if fgw_length_raw(word) == 76 && s != INV[prev as usize & 3] {
        return FgwAppended {
            word,
            prev,
            packed_a: pa,
        };
    }
    fgw_append(word, prev, pa, s)
}

negative_control!(
    word_decode_reduced_qa_long_streams_with_cap,
    "an append that ignores the cap must fail",
    expected = "the append over a long stream is not payload",
    prop::run(
        &proptest::collection::vec(0u32..4, 0..=300),
        |s| check_long(append_no_cap, &s)
    )
);

// ── REQ-PAY-028 ───────────────────────────────────────────────────────────────────────────────────────────────────

fn check_truncated(last_valid: fn(u32) -> bool, w: [u32; 4]) {
    assert!(fgw_truncated(w), "the word is not truncated");
    assert!(
        !fgw_reduced_length_valid(w),
        "fgw_reduced_length_valid is true for a truncated word"
    );
    assert!(
        !last_valid(fgw_length_raw(w)),
        "sd_last_symbol_valid is true for a truncated word"
    );
    assert_eq!(
        fgw_retained_prefix_length(w),
        76,
        "the retained prefix length is not 76"
    );
    assert_eq!(
        decode(w).len(),
        76,
        "decode of a truncated word is not its 76 retained symbols"
    );
}

/// A truncated word built from a stream of our own: `ab` repeated to the cap, one more push, then crossings.
fn truncated_stream() -> Vec<u32> {
    let mut s: Vec<u32> = (0..CAP).map(|k| if k % 2 == 0 { 0 } else { 2 }).collect();
    s.extend([0, 1, 3, 2, 1]);
    s
}

#[test]
fn truncated_word_validity_qa_rust() {
    let s = truncated_stream();
    let st = run(fgw_append, &s, 0);
    assert_eq!(st.word, ref_word(&s));
    check_truncated(sd_last_symbol_valid, st.word);
    // sd_last_symbol_valid's whole truth table: valid iff 1 ≤ len and len ≠ 127.
    for len in 0..=127u32 {
        assert_eq!(
            sd_last_symbol_valid(len),
            len >= 1 && len != 127,
            "sd_last_symbol_valid at {len}"
        );
    }
}

negative_control!(
    truncated_word_validity_qa_rust,
    "an sd_last_symbol_valid that only tests len >= 1 must fail on a truncated word",
    expected = "sd_last_symbol_valid is true for a truncated word",
    check_truncated(|len| len >= 1, run(fgw_append, &truncated_stream(), 0).word)
);

/// `fixtures/words/truncated.txt`, parsed: (stream, word, prefix).
fn fixture(text: &str) -> (Vec<u32>, [u32; 4], Vec<u32>) {
    let field = |key: &str| -> Vec<String> {
        text.lines()
            .filter(|l| !l.starts_with('#'))
            .find_map(|l| {
                let (k, v) = l.split_once('=')?;
                (k.trim() == key).then(|| v.split_whitespace().map(str::to_owned).collect())
            })
            .unwrap_or_else(|| panic!("the fixture has no `{key}`"))
    };
    let sym = |s: &String| {
        ["a", "A", "b", "B"]
            .iter()
            .position(|c| c == s)
            .expect("a symbol") as u32
    };
    let word: Vec<u32> = field("word")
        .iter()
        .map(|h| u32::from_str_radix(h.trim_start_matches("0x"), 16).expect("hex"))
        .collect();
    (
        field("stream").iter().map(sym).collect(),
        word.try_into().expect("four u32s"),
        field("prefix").iter().map(sym).collect(),
    )
}

fn fixture_text() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/words/truncated.txt"
    );
    std::fs::read_to_string(path).expect("fixtures/words/truncated.txt")
}

/// The fixture recomputed by the reference: its stream truncates, its word is the reference's word, its prefix the
/// reference's retained symbols; the read accessors hold REQ-PAY-028 on it.
fn check_fixture(text: &str) {
    let (stream, word, prefix) = fixture(text);
    let (kept, truncated) = ref_append(&stream);
    assert!(truncated, "the fixture stream does not truncate");
    assert_eq!(
        word,
        ref_word(&stream),
        "the fixture word is not payload §3's append of its stream"
    );
    assert_eq!(
        prefix, kept,
        "the fixture prefix is not the retained symbols"
    );
    check_truncated(sd_last_symbol_valid, word);
    assert_eq!(decode(word), prefix);
}

#[test]
fn truncated_word_validity_qa_fixture_recomputed() {
    check_fixture(&fixture_text());
}

negative_control!(
    truncated_word_validity_qa_fixture_recomputed,
    "a fixture word with one payload bit flipped must fail",
    expected = "the fixture word is not payload",
    check_fixture(&fixture_text().replace("word = 0xf9839ad5", "word = 0xf9839ad4"))
);

// ── REQ-PAY-033 (Rust target) ─────────────────────────────────────────────────────────────────────────────────────

/// The accessors on words of length 0, 1, 76 and 127, against payload §6's definitions; and on random appended words
/// the reduced crossing count (the reference reduction's length) equals `fgw_reduced_length` when valid.
fn check_lengths(prefix: fn([u32; 4]) -> u32) {
    let w76: Vec<u32> = (0..CAP).map(|k| if k % 2 == 0 { 2 } else { 0 }).collect();
    let cases: [([u32; 4], u32, bool, u32); 4] = [
        (ref_pack(&[]), 0, true, 0),
        (ref_pack(&[3]), 1, true, 1),
        (ref_pack(&w76), 76, true, 76),
        (ref_word(&truncated_stream()), 127, false, 76),
    ];
    for (w, raw, valid, retained) in cases {
        assert_eq!(fgw_length_raw(w), raw, "fgw_length_raw at {raw}");
        assert_eq!(fgw_truncated(w), raw == 127, "fgw_truncated at {raw}");
        assert_eq!(
            fgw_reduced_length_valid(w),
            valid,
            "fgw_reduced_length_valid at {raw}"
        );
        if valid {
            assert_eq!(fgw_reduced_length(w), raw, "fgw_reduced_length at {raw}");
        }
        assert_eq!(prefix(w), retained, "fgw_retained_prefix_length at {raw}");
        assert_eq!(
            sd_last_symbol_valid(raw),
            raw >= 1 && raw != 127,
            "sd_last_symbol_valid at {raw}"
        );
    }
}

#[test]
fn word_accessor_names_qa_rust_lengths_0_1_76_127() {
    check_lengths(fgw_retained_prefix_length);
    prop::run(&stream(76), |s| {
        let st = run(fgw_append, &s, 0);
        prop_assert!(fgw_reduced_length_valid(st.word));
        prop_assert_eq!(fgw_reduced_length(st.word), ref_reduce(&s).len() as u32);
        Ok(())
    });
}

negative_control!(
    word_accessor_names_qa_rust_lengths_0_1_76_127,
    "a retained prefix that does not clamp 127 must fail",
    expected = "fgw_retained_prefix_length at 127",
    check_lengths(fgw_length_raw)
);

// ── REQ-PAY-025 (Rust target) ─────────────────────────────────────────────────────────────────────────────────────

fn check_split(word: (usize, usize), ftle: (usize, usize), base: (usize, usize)) {
    assert_eq!(
        word,
        (16, 16),
        "the word buffer element is not exactly 16 B"
    );
    assert_eq!(
        ftle,
        (144, 8),
        "SimStateFTLE is not 144 B at align 8 (payload §0)"
    );
    assert_eq!(
        base,
        (96, 8),
        "SimStateBase is not 96 B at align 8 (payload §0)"
    );
}

#[test]
fn word_buffer_split_qa_rust_sizes() {
    use std::mem::{align_of, size_of};
    check_split(
        (size_of::<FreeGroupWord>(), align_of::<FreeGroupWord>()),
        (size_of::<SimStateFTLE>(), align_of::<SimStateFTLE>()),
        (size_of::<SimStateBase>(), align_of::<SimStateBase>()),
    );
}

/// A `SimStateFTLE` with the word inline: 16-aligned, padded.
#[cfg(feature = "controls")]
#[repr(C)]
struct InlineWord {
    _s: SimStateFTLE,
    _w: FreeGroupWord,
}

negative_control!(
    word_buffer_split_qa_rust_sizes,
    "a SimState with the word inline must fail",
    expected = "SimStateFTLE is not 144 B at align 8",
    {
        use std::mem::{align_of, size_of};
        check_split(
            (size_of::<FreeGroupWord>(), align_of::<FreeGroupWord>()),
            (size_of::<InlineWord>(), align_of::<InlineWord>()),
            (size_of::<SimStateBase>(), align_of::<SimStateBase>()),
        )
    }
);
