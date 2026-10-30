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
//! - [`binding`] is the word buffer as the march and resolve reach it: a binding of its own, parallel to the
//!   `SimState` buffer at the same index; the march's per-step path reaches it only through the crossing hook.
//! - [`crossing`] is the detection that feeds the hook, at every accepted substep: the half-open count, the order by
//!   crossing fraction `τ` and the cut-ID tie-break (integrator contract Part 5, the τ tie rule).

pub mod append;
pub mod binding;
pub mod crossing;

pub use crate::payload::set_last_symbol;
use crate::payload::{
    continuation_symbol, fgw_div3, fgw_mixed_radix, fgw_retained_prefix_length, FGW_CAPACITY,
    FGW_NO_SYMBOL,
};
pub use append::{fgw_append, fgw_mul3_add, FgwAppended};
pub use binding::{on_substep, sample_word, CrossingHook, SubstepCrossings, WordBuffer};
pub use crossing::{substep_crossings, CrossingDetector, CutDistances};

/// The symbols of a word in order, first to last, from [`fgw_decode`]: `d₀`, then `zeros` zero digits, then each digit
/// popped from `digits`, replayed through `continuation_symbol` from the symbol before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FgwDecode {
    /// `W`'s digits `e₁ … e_{ℓ−1}` past the `zeros` leading ones, as an integer whose base-3 tail is the first of them,
    /// popped one per symbol after those.
    digits: [u32; 4],
    /// The digits `e₁ …` above `W`'s 77 that can be nonzero, all zero, replayed before `digits`: `length − 1 − 77` at
    /// a `length_raw` of 78…126, otherwise none.
    zeros: u32,
    /// `d₀`, the residue, masked `& 3`: 0 at a `length_raw` of 78…126.
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
/// `& 3`, and on a `length_raw` of 78…126, which payload §3 leaves unused, at most 77 digits are popped and the rest,
/// all zero, are counted rather than pushed, so the second integer never overflows.
#[inline]
pub fn fgw_decode(w: [u32; 4]) -> FgwDecode {
    let length = fgw_retained_prefix_length(w);
    let mut v = fgw_mixed_radix(w);
    let mut digits = [0; 4];
    // `W`'s base-3 digits that can be nonzero are `FGW_CAPACITY + 1`, 77: `W` < 2¹²¹ < 3⁷⁷ (payload §3's 121 bits). No
    // more are popped, so the popped ones fit `digits` (3⁷⁷ < 2¹²⁸); `popped` counts `d₀` with them.
    let popped = length.min(FGW_CAPACITY + 2);
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
