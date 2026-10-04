//! The word buffer's append and decode (dd_simstate_payload §3; dd_generation_root §3.3, §3.3a): the free-group word
//! of one sample, one `vec4<u32>` in the cold word buffer at the sample's own index, as `[u32; 4]`. Shared source, no
//! float in it, so the same at f32 and f64 (R-265).
//!
//! The word packs a freely-reduced word in the four generators `a = 0, A = 1, b = 2, B = 3` as the mixed-radix integer
//! `W` of payload §3's Horner recurrence, `W₁ = d₀`, `W_{k+1} = 3·W_k + e_k`: `d₀` the first symbol, each `e_k` the digit
//! of payload §3's frozen continuation table that continues the symbol before it. `W`'s low 96 bits fill `x`, `y` and
//! `z`, its high bits `.w`'s `payload`; `.w`'s `length` holds 0…76, or 127 once truncated. The generated accessors read
//! it ([`crate::payload::fgw_mixed_radix`], [`crate::payload::fgw_length_raw`], [`crate::payload::fgw_symbol`], …).
//!
//! - [`fgw_append`] is payload §3's append, run on a branch-cut crossing: free reduction pops the tail digit and
//!   recovers the new `prev` through `predecessor_symbol`; at the cap, 76 symbols, a push truncates the word to the
//!   sentinel 127, after which every crossing is ignored. It mirrors `prev` into `packed_a`'s `last_symbol` wherever
//!   `prev` changes, through [`set_last_symbol`], so the word and its cache are written by the same crossing.
//! - [`fgw_decode`] is payload §3's decode, at resolve or inspect: O(length), sequential. The base-3 tail is popped by
//!   depth, `length − 1` times, the residue is `d₀`, and the digits are replayed forward through `continuation_symbol`.
//!   The popped digits come out last first; they are pushed, as they come, onto a second integer by the same Horner
//!   step, so its own tail holds them first first, and the replay pops it. No array holds them: no runtime index (as
//!   the table functions, which are comparison chains).

pub use crate::payload::set_last_symbol;
use crate::payload::{
    continuation_index, continuation_symbol, fgw_div3, fgw_length_raw, fgw_mixed_radix, fgw_pack,
    fgw_retained_prefix_length, fgw_truncated, inverse, predecessor_symbol, FGW_CAPACITY,
    FGW_LENGTH_SENTINEL, FGW_NO_SYMBOL,
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

/// `W`'s base-3 digits that can be nonzero: `W` < 2¹²¹ < 3⁷⁷ (payload §3's 121 bits). Any more are zero, and no more
/// are popped, so the popped ones fit [`FgwDecode`]'s `digits` (3⁷⁷ < 2¹²⁸) at every `length_raw`, 77…126 included.
const FGW_W_DIGITS: u32 = 77;

/// The symbols of a word in order, first to last, from [`fgw_decode`]: `d₀`, then `zeros` zero digits, then each digit
/// popped from `digits`, replayed through `continuation_symbol` from the symbol before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FgwDecode {
    /// `W`'s digits `e₁ … e_{ℓ−1}` past the `zeros` leading ones, as an integer whose base-3 tail is the first of them,
    /// popped one per symbol after those.
    digits: [u32; 4],
    /// The digits `e₁ …` above [`FGW_W_DIGITS`], all zero, replayed before `digits`: `length − 1 − 77` at a
    /// `length_raw` of 78…126, otherwise none.
    zeros: u32,
    /// `d₀`, the residue, masked `& 3`: 0 above [`FGW_W_DIGITS`].
    first: u32,
    /// The symbols not yet yielded.
    remaining: u32,
    /// The symbol last yielded; [`FGW_NO_SYMBOL`] before `d₀`.
    prev: u32,
}

impl Iterator for FgwDecode {
    type Item = u32;

    #[inline]
    fn next(&mut self) -> Option<u32> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        self.prev = if self.prev == FGW_NO_SYMBOL {
            self.first
        } else if self.zeros > 0 {
            self.zeros -= 1;
            continuation_symbol(self.prev, 0)
        } else {
            let (q, e) = fgw_div3(self.digits);
            self.digits = q;
            continuation_symbol(self.prev, e)
        };
        Some(self.prev)
    }
}

/// Payload §3's decode of `w`, at resolve or inspect, never in the march: its retained prefix's symbols, first to last
/// (all of them; 76 for a truncated word, whose retained prefix is not the reduced word, payload §3). The base-3 tail
/// is popped by depth, `length − 1` times, each digit pushed onto a second integer, and the residue is `d₀`; the
/// iterator replays the digits forward from it. O(length), sequential, not random-access.
///
/// Total, as [`crate::payload::fgw_symbol`] is, and equal to it at every `k` of any `[u32; 4]` (R-321): `d₀` is masked
/// `& 3`, and on a `length_raw` of 78…126, which payload §3 leaves unused, at most [`FGW_W_DIGITS`] digits are popped
/// and the rest, all zero, are counted rather than pushed, so the second integer never overflows.
#[inline]
pub fn fgw_decode(w: [u32; 4]) -> FgwDecode {
    let length = fgw_retained_prefix_length(w);
    let mut v = fgw_mixed_radix(w);
    let mut digits = [0; 4];
    let popped = length.min(FGW_W_DIGITS + 1);
    let mut i = 1;
    while i < popped {
        let (q, e) = fgw_div3(v);
        v = q;
        digits = fgw_mul3_add(digits, e);
        i += 1;
    }
    // The residue is `d₀`, a symbol code on any word `fgw_append` writes, but the decode takes any `[u32; 4]`: masked
    // as `fgw_symbol` masks it, unasserted, so the two agree on every word and no out-of-range symbol exists (R-321).
    FgwDecode {
        digits,
        zeros: length - popped,
        first: v[0] & 3,
        remaining: length,
        prev: FGW_NO_SYMBOL,
    }
}
