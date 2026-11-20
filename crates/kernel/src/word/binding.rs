//! The word buffer as a binding of its own (dd_simstate_payload §0, §3; dd_generation_root §3.3a; render contract Part
//! 1): one [`FreeGroupWord`] per sample, per copy, a parallel array to the `SimState` buffer, `words[i]` the word of the
//! sample whose state is `simstate[i]`, the same index. It is never inline in `SimState` and never touched per step:
//! the word is mutated only on a branch-cut crossing and read only at resolve and inspect.
//!
//! - [`WordBuffer`] is the binding: a load and a store at a sample's index, and nothing else.
//! - [`CrossingHook`] is all the march carries of the word between crossings: the sample's index and the live `prev`
//!   (payload §3), no word. Its [`CrossingHook::crossing`] is the one place the march touches the buffer: it loads the
//!   word at the index, appends the crossing's symbol ([`fgw_append`]) and stores it back, once per crossing.
//! - [`on_substep`] is the per-step path's whole word work: the substep's crossings ([`SubstepCrossings`], at most one
//!   per cut), in the order given, each through the hook, each slot by a constant index; with none, the buffer is
//!   neither read nor written.
//! - [`sample_word`] is the read at resolve and inspect, the cold path (render contract's `sample_word(i)`).

use super::append::fgw_append;
use crate::payload::{FreeGroupWord, FGW_NO_SYMBOL, MAX_CROSSINGS_PER_SUBSTEP};

/// The word buffer, a binding of its own: the word at index `i` is that of the sample whose `SimState` is at `i`.
/// The march reaches it only through [`CrossingHook`]; resolve and inspect through [`sample_word`].
pub trait WordBuffer {
    /// The word at `i`.
    fn load(&self, i: usize) -> [u32; 4];
    /// Writes `w` as the word at `i`.
    fn store(&mut self, i: usize, w: [u32; 4]);
}

impl WordBuffer for [FreeGroupWord] {
    #[inline]
    fn load(&self, i: usize) -> [u32; 4] {
        self[i].free_group_word
    }

    #[inline]
    fn store(&mut self, i: usize, w: [u32; 4]) {
        self[i].free_group_word = w;
    }
}

/// What the march carries of one sample's word between crossings: the sample's index in both buffers and the live
/// `prev`, a kernel-local (payload §3). No word: the per-step path has none to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossingHook {
    /// The sample's index, its `SimState`'s and its word's.
    pub index: usize,
    /// The word's last symbol, [`FGW_NO_SYMBOL`] while the word is empty.
    pub prev: u32,
}

impl CrossingHook {
    /// The hook of the sample at `index`, whose word is empty: `prev` none.
    #[inline]
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            prev: FGW_NO_SYMBOL,
        }
    }

    /// A branch-cut crossing with symbol `s`: the word at the hook's index loaded, `s` appended ([`fgw_append`]) and
    /// the word stored back, one load and one store; `prev` follows. Returns `packed_a` with `last_symbol` mirroring
    /// `prev` wherever it changed, for the caller to write into the same sample's `SimState` (payload §3: the word and
    /// its cache are written by the same crossing).
    #[inline]
    pub fn crossing<B: WordBuffer + ?Sized>(
        &mut self,
        words: &mut B,
        packed_a: u32,
        s: u32,
    ) -> u32 {
        let appended = fgw_append(words.load(self.index), self.prev, packed_a, s);
        words.store(self.index, appended.word);
        self.prev = appended.prev;
        appended.packed_a
    }
}

/// One accepted substep's branch-cut crossings, in order (chronological, by the detector): `count` of them, at most
/// [`MAX_CROSSINGS_PER_SUBSTEP`], one per cut (payload §3; symbolic_dynamics_contract §1), in `symbols[0..count]`. A
/// fixed capacity, so [`on_substep`] reaches each slot by a constant index: a runtime index in a kernel loop is a
/// bounds check rust-gpu lowers to a multi-level exit (gpu_determinism_note § "The discipline", rule 5; wgpu#4449).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubstepCrossings {
    /// The crossings' symbols, the first `count` used.
    pub symbols: [u32; MAX_CROSSINGS_PER_SUBSTEP as usize],
    /// How many of `symbols` are crossings: 0 to [`MAX_CROSSINGS_PER_SUBSTEP`].
    pub count: u32,
}

/// One accepted substep's word work, the march's per-step path: each of the substep's branch-cut crossings, in order,
/// through [`CrossingHook::crossing`]; nothing else. A substep with no crossing neither reads nor writes the word
/// buffer. Each slot is reached by a constant index, unrolled, one test of `count` each; a `count` past the capacity
/// is `debug_assert!`ed, and in a release build appends the capacity's crossings. Returns `packed_a` as the last
/// crossing left it.
#[inline]
pub fn on_substep<B: WordBuffer + ?Sized>(
    hook: &mut CrossingHook,
    words: &mut B,
    packed_a: u32,
    crossings: SubstepCrossings,
) -> u32 {
    debug_assert!(
        crossings.count <= MAX_CROSSINGS_PER_SUBSTEP,
        "more crossings than cuts in one substep"
    );
    let mut packed_a = packed_a;
    if crossings.count > 0 {
        packed_a = hook.crossing(words, packed_a, crossings.symbols[0]);
    }
    if crossings.count > 1 {
        packed_a = hook.crossing(words, packed_a, crossings.symbols[1]);
    }
    packed_a
}

/// The word of the sample at `i`, read at resolve or inspect, never in the march (render contract's `sample_word(i)`).
#[inline]
pub fn sample_word<B: WordBuffer + ?Sized>(words: &B, i: usize) -> [u32; 4] {
    words.load(i)
}
