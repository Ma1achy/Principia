//! QA tests for TASK-M1-02, second round: the decode and `fgw_symbol` on any `[u32; 4]`, every `length_raw` 0…127,
//! each held to an independent reference written from payload §3 alone (dd_simstate_payload), not to each other.
//!
//! Payload §3's Horner recurrence `W₁ = d₀, W_{k+1} = 3·W_k + e_k` makes a word of length `L` the integer
//! `W = d₀·3^{L−1} + Σ e_k·3^{L−1−k}`: digit `e_k` is `⌊W / 3^{L−1−k}⌋ mod 3`, and `d₀` is `⌊W / 3^{L−1}⌋`, a symbol code
//! masked to 2 bits as R-321 masks every symbol input. The reference reads them that way in a u128 (`W` < 2¹²¹), and
//! replays the digits through payload §3's frozen `cont_symbol[e][prev]` table. The retained prefix is `length_raw`,
//! 127 clamped to 76 (payload §3, §6).
//!
//! - `word_any_qa_decode_matches_reference`: random words, every bit of `W` arbitrary, lengths weighted to 77…127.
//! - `word_any_qa_every_length`: fixed payloads (zero, all ones, `2¹²¹ − 1` and others) at every `length_raw` 0…127.
//!
//! In both, `fgw_decode` yields exactly the retained prefix's length, then `None`, and equals the reference symbol by
//! symbol; `fgw_symbol(w, k)` equals it at every `k` and is `FGW_NO_SYMBOL` at and past the length. Each check takes
//! the decode and the symbol accessor as function pointers; its negative control (R-176) runs it on a variant with one
//! fault.

#[cfg(feature = "controls")]
use kernel::payload::{continuation_symbol, fgw_div3, fgw_mixed_radix, fgw_retained_prefix_length};
use kernel::payload::{fgw_symbol, FGW_NO_SYMBOL};
use kernel::word::fgw_decode;
use proptest::prelude::*;
use validation::{negative_control, prop};

/// Payload §3's frozen `cont_symbol[e][prev]`.
const CONT: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];

/// `W` as payload §3's bit map lays it out: x, y, z, then `.w` bits 0–24.
fn ref_w_of(w: [u32; 4]) -> u128 {
    u128::from(w[0])
        | u128::from(w[1]) << 32
        | u128::from(w[2]) << 64
        | u128::from(w[3] & 0x01ff_ffff) << 96
}

/// The retained prefix's length: `.w` bits 25–31, 127 clamped to 76.
fn ref_len(w: [u32; 4]) -> u32 {
    match w[3] >> 25 {
        127 => 76,
        n => n,
    }
}

/// `⌊W / 3^p⌋`, 0 once `3^p` exceeds `W`'s 121 bits.
fn ref_div_pow3(w: u128, p: u32) -> u128 {
    // 3^80 < 2^127 < 3^81; W < 2^121 < 3^77, so any p ≥ 77 gives 0.
    if p >= 77 {
        0
    } else {
        w / 3u128.pow(p)
    }
}

/// The retained prefix's symbols by the Horner recurrence read backwards.
fn ref_decode(w: [u32; 4]) -> Vec<u32> {
    let len = ref_len(w);
    let big = ref_w_of(w);
    let mut out = Vec::with_capacity(len as usize);
    for k in 0..len {
        let q = ref_div_pow3(big, len - 1 - k);
        let s = if k == 0 {
            (q & 3) as u32
        } else {
            CONT[(q % 3) as usize][out[k as usize - 1] as usize]
        };
        out.push(s);
    }
    out
}

type Decode = fn([u32; 4]) -> Vec<u32>;
type Symbol = fn([u32; 4], u32) -> u32;

/// [`fgw_decode`] collected, at most a few past the retained prefix, so a decode that never ends fails, not hangs.
fn decode(w: [u32; 4]) -> Vec<u32> {
    fgw_decode(w).take(ref_len(w) as usize + 3).collect()
}

fn check(dec: Decode, sym: Symbol, w: [u32; 4]) -> Result<(), TestCaseError> {
    let want = ref_decode(w);
    prop_assert_eq!(
        dec(w),
        want.clone(),
        "fgw_decode is not payload §3's decode of {:x?} (length_raw {})",
        w,
        w[3] >> 25
    );
    for k in 0..want.len() as u32 {
        prop_assert_eq!(
            sym(w, k),
            want[k as usize],
            "fgw_symbol is not payload §3's symbol {} of {:x?}",
            k,
            w
        );
    }
    for k in [want.len() as u32, want.len() as u32 + 1, 127, u32::MAX] {
        prop_assert_eq!(
            sym(w, k),
            FGW_NO_SYMBOL,
            "fgw_symbol past the prefix is not FGW_NO_SYMBOL"
        );
    }
    Ok(())
}

/// Any `[u32; 4]`, its length field drawn from all of 0…127 or, as often, from 77…127.
fn any_word() -> impl Strategy<Value = [u32; 4]> {
    let len = prop_oneof![
        0u32..=127,
        77u32..=127,
        Just(77),
        Just(78),
        Just(126),
        Just(127)
    ];
    (any::<[u32; 4]>(), len).prop_map(|(w, n)| [w[0], w[1], w[2], (w[3] & 0x01ff_ffff) | (n << 25)])
}

#[test]
fn word_any_qa_decode_matches_reference() {
    prop::run(&any_word(), |w| check(decode, fgw_symbol, w));
}

/// The previous round's decode: every `length − 1` digit pushed onto a 128-bit integer, which wraps past 80 of them,
/// and `d₀` the residue unmasked.
#[cfg(feature = "controls")]
fn decode_uncapped(w: [u32; 4]) -> Vec<u32> {
    let length = fgw_retained_prefix_length(w);
    let mut v = fgw_mixed_radix(w);
    let mut digits = [0u32; 4];
    for _ in 1..length {
        let (q, e) = fgw_div3(v);
        v = q;
        digits = kernel::word::fgw_mul3_add(digits, e);
    }
    let mut out: Vec<u32> = Vec::new();
    if length > 0 {
        out.push(v[0]);
    }
    for _ in 1..length {
        let (q, e) = fgw_div3(digits);
        digits = q;
        out.push(continuation_symbol(out[out.len() - 1] & 3, e));
    }
    out
}

negative_control!(
    word_any_qa_decode_matches_reference,
    "the uncapped, unmasked decode must fail on arbitrary words",
    expected = "fgw_decode is not payload §3's decode",
    prop::run(&any_word(), |w| check(decode_uncapped, fgw_symbol, w))
);

/// Payloads for every length: zero, all ones (`W = 2¹²¹ − 1`), the largest 76-symbol `W`, one bit at the top, one at
/// the bottom, and a mixed pattern.
fn payloads() -> Vec<[u32; 4]> {
    let largest76 = 4u128 * 3u128.pow(75) - 1;
    vec![
        [0; 4],
        [u32::MAX, u32::MAX, u32::MAX, 0x01ff_ffff],
        [
            largest76 as u32,
            (largest76 >> 32) as u32,
            (largest76 >> 64) as u32,
            (largest76 >> 96) as u32,
        ],
        [0, 0, 0, 1 << 24],
        [1, 0, 0, 0],
        [0x9e37_79b9, 0x7f4a_7c15, 0xf39c_c060, 0x01a5_e3b1],
    ]
}

fn check_every_length(dec: Decode, sym: Symbol) {
    for p in payloads() {
        for n in 0..=127u32 {
            let w = [p[0], p[1], p[2], (p[3] & 0x01ff_ffff) | (n << 25)];
            if let Err(e) = check(dec, sym, w) {
                panic!("{e}");
            }
        }
    }
}

#[test]
fn word_any_qa_every_length() {
    check_every_length(decode, fgw_symbol);
}

/// `fgw_symbol` reading `d₀` without its 2-bit mask, `W`'s top digits beyond it leaking in: R-321's mask dropped.
#[cfg(feature = "controls")]
fn symbol_unmasked(w: [u32; 4], k: u32) -> u32 {
    let s = fgw_symbol(w, k);
    if k == 0 && s != FGW_NO_SYMBOL {
        let len = ref_len(w);
        let mut v = fgw_mixed_radix(w);
        for _ in 1..len.min(78) {
            v = fgw_div3(v).0;
        }
        v[0]
    } else {
        s
    }
}

negative_control!(
    word_any_qa_every_length,
    "an fgw_symbol whose d₀ is unmasked must fail",
    expected = "fgw_symbol is not payload §3's symbol 0",
    check_every_length(decode, symbol_unmasked)
);
