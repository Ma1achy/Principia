//! Payload §3's append, run on a branch-cut crossing (dd_simstate_payload §3; dd_generation_root §3.3): the push
//! `W = 3·W + e`, the cancellation-pop by div-mod and the reverse table, and the length cap's truncation sentinel,
//! over `W`'s four u32 limbs, with no allocation and no runtime index. The march carries the live `prev`; the append
//! mirrors it into `packed_a`'s `last_symbol` (descriptor bits 8–9) wherever it changes, through [`set_last_symbol`].

use crate::payload::{
    continuation_index, fgw_div3, fgw_length_raw, fgw_mixed_radix, fgw_pack, fgw_truncated,
    inverse, predecessor_symbol, set_last_symbol, FGW_CAPACITY, FGW_LENGTH_SENTINEL, FGW_NO_SYMBOL,
};

/// One limb of [`fgw_mul3_add`]: `3 · limb + carry`, `carry` < 3, over its two 16-bit halves, so no step exceeds a
/// u32. The low 32 bits, the halves added (their bits are disjoint), and the carry out.
#[inline]
fn mul3_add_limb(limb: u32, carry: u32) -> (u32, u32) {
    let lo = 3 * (limb & 0xffff) + carry;
    let hi = 3 * (limb >> 16) + (lo >> 16);
    ((hi << 16) + (lo & 0xffff), hi >> 16)
}

/// `3 · v + e`, `v` four limbs, low first, and `e` a base-3 digit: payload §3's push, `W = 3·W + e`. Constant indices
/// only. A carry out of the top limb is dropped; the append never makes one, as 76 symbols fit 121 bits.
#[inline]
pub fn fgw_mul3_add(v: [u32; 4], e: u32) -> [u32; 4] {
    let (l0, c) = mul3_add_limb(v[0], e);
    let (l1, c) = mul3_add_limb(v[1], c);
    let (l2, c) = mul3_add_limb(v[2], c);
    let (l3, _) = mul3_add_limb(v[3], c);
    [l0, l1, l2, l3]
}

/// What a crossing leaves: the word, the live `prev` symbol the march carries, and `packed_a`, its `last_symbol`
/// mirroring `prev` (payload §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FgwAppended {
    pub word: [u32; 4],
    pub prev: u32,
    pub packed_a: u32,
}

/// Payload §3's append of the crossing symbol `s` to `word`, whose last symbol is `prev` (unread while the word is
/// empty), with `packed_a` the sample's:
/// - truncated (`length_raw` 127): the crossing is ignored, and the word, `prev` and `last_symbol` stay frozen;
/// - empty: `W = s`, length 1, `prev = s`;
/// - `s` the inverse of `prev`, free reduction: from length 1 the word is emptied and `prev` is [`FGW_NO_SYMBOL`]
///   (payload §3's `INVALID`; `last_symbol` is then unread, so left as it was); otherwise the tail digit `e` is popped,
///   `W = W div 3`, and `prev = predecessor_symbol(prev, e)`;
/// - at the capacity, 76 symbols: `length_raw = 127`, the truncation sentinel, `W` kept; the cap, not `W`'s size,
///   decides it;
/// - otherwise the push: `W = 3·W + continuation_index(prev, s)`, `prev = s`.
///
/// `packed_a`'s `last_symbol` is set to `prev` wherever `prev` changes (the push and the pop), and nowhere else
/// (payload §3). `s` is a symbol code, 0…3: `debug_assert!`ed, then masked `& 3`, as the table functions are (R-321).
#[inline]
pub fn fgw_append(word: [u32; 4], prev: u32, packed_a: u32, s: u32) -> FgwAppended {
    debug_assert!(s < 4, "s is not a symbol code (R-321)");
    let s = s & 3;
    let length = fgw_length_raw(word);
    if fgw_truncated(word) {
        FgwAppended {
            word,
            prev,
            packed_a,
        }
    } else if length == 0 {
        FgwAppended {
            word: fgw_pack([s, 0, 0, 0], 1),
            prev: s,
            packed_a: set_last_symbol(packed_a, s),
        }
    } else if s == inverse(prev) {
        if length == 1 {
            FgwAppended {
                word: fgw_pack([0; 4], 0),
                prev: FGW_NO_SYMBOL,
                packed_a,
            }
        } else {
            let (v, e) = fgw_div3(fgw_mixed_radix(word));
            let prev = predecessor_symbol(prev, e);
            FgwAppended {
                word: fgw_pack(v, length - 1),
                prev,
                packed_a: set_last_symbol(packed_a, prev),
            }
        }
    } else if length == FGW_CAPACITY {
        FgwAppended {
            word: fgw_pack(fgw_mixed_radix(word), FGW_LENGTH_SENTINEL),
            prev,
            packed_a,
        }
    } else {
        let v = fgw_mul3_add(fgw_mixed_radix(word), continuation_index(prev, s));
        FgwAppended {
            word: fgw_pack(v, length + 1),
            prev: s,
            packed_a: set_last_symbol(packed_a, s),
        }
    }
}
