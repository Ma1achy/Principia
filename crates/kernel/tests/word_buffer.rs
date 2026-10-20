//! The word buffer's binding, append and length cap (`kernel::word`), against the reference free-group reducer
//! (`validation::reference::free_group`), which shares no code with the kernel (dd_simstate_payload §0, §2, §3, §6;
//! dd_generation_root §3.3, §3.3a).
//! - REQ-PAY-036: the word buffer is a binding of its own, at the sample's own index; a substep with no crossing
//!   touches it not at all (`word_buffer_binding`).
//! - REQ-PAY-042: over a march's substeps, the word buffer's writes equal the crossings, and the per-step path reads
//!   no word outside a crossing (`word_write_count`). The crossings are a synthetic stream until TASK-M3-16's detector
//!   exists; TASK-M3-16 re-runs the check on real trajectories.
//! - REQ-PAY-048: `last_symbol` is the final symbol of the decoded word whenever 1 ≤ length ≤ 76, and every set keeps
//!   `packed_a`'s other bits (`last_symbol_mirror`, `last_symbol_mirror_set_keeps_other_bits`).
//! - REQ-PAY-052: the packing is payload §3's closed form, fitting 121 bits, with `length` in `.w` bits 25–31
//!   (`word_horner_packing`).
//! - REQ-PAY-053: the append agrees with the reference reducer on random streams, 77-symbol words with a small `W`
//!   among them (`word_vs_reference_reducer`).
//! - REQ-TOOL-035: a 77th symbol truncates the word to length 127, at a small `W` too; a pop decrements the length
//!   (`word_truncation_sentinel`).
//!
//! Each check takes the append, the hook, the substep or the setter as a function, so its control runs the same check
//! on a faulty one.

use std::cell::RefCell;
use std::mem::size_of;

use kernel::payload::{
    fgw_length_raw, fgw_mixed_radix, fgw_truncated, sd_last_symbol, FreeGroupWord, SimStateFTLE,
    FGW_CAPACITY, FGW_LENGTH_SENTINEL, FGW_NO_SYMBOL, MAX_CROSSINGS_PER_SUBSTEP, SIMSTATE_BINDING,
    SIMSTATE_GROUP, WORD_BINDING, WORD_GROUP,
};
use kernel::word::{
    fgw_append, fgw_decode, on_substep, sample_word, set_last_symbol, CrossingHook, FgwAppended,
    SubstepCrossings, WordBuffer,
};
use proptest::prelude::*;
use validation::reference::free_group::{
    closed_form, packed, reduce, Reduced, CONT_SYMBOL, INVERSE,
};
use validation::{negative_control, prop};

/// An append: [`fgw_append`], or a control's faulty one.
type Append = fn([u32; 4], u32, u32, u32) -> FgwAppended;

/// A crossing through the hook: [`CrossingHook::crossing`], or a control's faulty one.
type Crossing = fn(&mut CrossingHook, &mut Counted, u32, u32) -> u32;

/// One substep's word work: [`on_substep`], or a control's faulty one.
type Substep = fn(&mut CrossingHook, &mut Counted, u32, SubstepCrossings) -> u32;

/// A `last_symbol` setter: [`set_last_symbol`], or a control's faulty one.
type Set = fn(u32, u32) -> u32;

/// A logged access to a [`Counted`] word buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Access {
    Load,
    Store,
}

/// The most accesses a [`Counted`] buffer logs: far above any check's (at most 3 crossings on each of 600 substeps,
/// a load and a store each, or 1200 symbols through the hook), so a faulty loop that never ends (a mutant's) panics
/// here instead of growing the log without bound.
const LOG_BOUND: usize = 1 << 14;

/// A word buffer that logs each access, its kind and index, so a check can count the reads and writes. The log is
/// bounded by [`LOG_BOUND`].
struct Counted {
    words: Vec<FreeGroupWord>,
    log: RefCell<Vec<(Access, usize)>>,
}

impl Counted {
    fn new(words: Vec<FreeGroupWord>) -> Self {
        Self {
            words,
            log: RefCell::new(Vec::new()),
        }
    }

    /// Logs an access; panics past [`LOG_BOUND`] accesses.
    fn log_access(&self, kind: Access, i: usize) {
        let mut log = self.log.borrow_mut();
        assert!(
            log.len() < LOG_BOUND,
            "the word buffer's access log passed its bound, {LOG_BOUND}: a loop that does not end"
        );
        log.push((kind, i));
    }

    /// The accesses logged since the first `from`.
    fn since(&self, from: usize) -> Vec<(Access, usize)> {
        self.log.borrow()[from..].to_vec()
    }
}

impl WordBuffer for Counted {
    fn load(&self, i: usize) -> [u32; 4] {
        self.log_access(Access::Load, i);
        self.words[..].load(i)
    }

    fn store(&mut self, i: usize, w: [u32; 4]) {
        self.log_access(Access::Store, i);
        self.words[..].store(i, w);
    }
}

/// How many of `accesses` are of `kind`.
fn count(accesses: &[(Access, usize)], kind: Access) -> usize {
    accesses.iter().filter(|(k, _)| *k == kind).count()
}

/// The empty word, `prev` none, and `packed_a`.
fn empty(packed_a: u32) -> FgwAppended {
    FgwAppended {
        word: [0; 4],
        prev: FGW_NO_SYMBOL,
        packed_a,
    }
}

/// `a` with each symbol of `stream` appended in turn by `append`.
fn append_all(append: Append, a: FgwAppended, stream: &[u32]) -> FgwAppended {
    stream
        .iter()
        .fold(a, |a, &s| append(a.word, a.prev, a.packed_a, s))
}

/// A freely-reduced word from `raw`: its first value `& 3` the first symbol, each next value mod 3 the digit continuing
/// the symbol before, by the reference's table.
fn reduced_word(raw: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for &r in raw {
        let s = match out.last() {
            None => r & 3,
            Some(&p) => CONT_SYMBOL[(r % 3) as usize][p as usize],
        };
        out.push(s);
    }
    out
}

/// A symbol stream from `raw`, a walk that pops: a value whose `% 1000` is below `pop` is the inverse of the walk's last
/// unreduced symbol, a pop; any other continues it with the digit `(value / 1000) % 3` (the first symbol `& 3`).
fn walk(raw: &[u32], pop: u32) -> Vec<u32> {
    let mut stack: Vec<u32> = Vec::new();
    let mut out = Vec::new();
    for &r in raw {
        let s = match stack.last() {
            Some(&p) if r % 1000 < pop => {
                stack.pop();
                INVERSE[p as usize]
            }
            Some(&p) => {
                let s = CONT_SYMBOL[((r / 1000) % 3) as usize][p as usize];
                stack.push(s);
                s
            }
            None => {
                let s = (r / 1000) & 3;
                stack.push(s);
                s
            }
        };
        out.push(s);
    }
    out
}

/// Streams of up to 240 symbols: walks popping at rates from none to two in five, which reach the cap at the low rates
/// and pop deep at the high ones, and uniform draws, a quarter of whose symbols cancel.
fn streams() -> impl Strategy<Value = Vec<Vec<u32>>> {
    let walks = (0..400u32, proptest::collection::vec(any::<u32>(), 0..=240))
        .prop_map(|(pop, raw)| walk(&raw, pop));
    let uniform = proptest::collection::vec(0..4u32, 0..=240);
    proptest::collection::vec(prop_oneof![walks, uniform], 1..6)
}

/// A 77-symbol non-cancelling stream whose first 76 have a small `W`: `d₀ = a`, the first `zeros` digits 0, the rest
/// from `raw`; then a 77th symbol continuing the 76th by digit `e`.
fn small_w_77(zeros: usize, raw: &[u32], e: u32) -> Vec<u32> {
    let mut symbols = vec![0];
    for k in 1..FGW_CAPACITY as usize {
        let d = if k <= zeros { 0 } else { raw[k] % 3 };
        let p = symbols[k - 1] as usize;
        symbols.push(CONT_SYMBOL[d as usize][p]);
    }
    let last = *symbols.last().expect("76 symbols") as usize;
    symbols.push(CONT_SYMBOL[(e % 3) as usize][last]);
    symbols
}

/// [`small_w_77`]'s streams: up to 76 leading zero digits, so `W` is as small as 0.
fn small_w_streams() -> impl Strategy<Value = Vec<Vec<u32>>> {
    proptest::collection::vec(
        (
            0..=75usize,
            proptest::collection::vec(any::<u32>(), 76),
            0..3u32,
        )
            .prop_map(|(zeros, raw, e)| small_w_77(zeros, &raw, e)),
        1..6,
    )
}

/// The all-`a` stream of `n` symbols: every digit 0, so `W = 0` at any length.
fn all_a(n: usize) -> Vec<u32> {
    vec![0; n]
}

// ── word_buffer_binding (REQ-PAY-036) ────────────────────────────────────────────────────────────────────────────────

/// `packed_a` of sample `j` in [`check_binding`]'s buffers, distinct per sample, `last_symbol` bits clear.
fn packed_a_of(j: usize) -> u32 {
    (0x1357_9b00 ^ (j as u32).wrapping_mul(0x9e37_79b9)) & !(3 << 8)
}

/// The word sample `j` starts with in [`check_binding`]'s buffer: nonzero, distinct per sample, so any write shows.
fn word_of(j: usize) -> [u32; 4] {
    let j = j as u32;
    [j + 1, j ^ 0xdead_beef, !j, j.wrapping_mul(7)]
}

/// Each `(n, i, stream)`: a `SimState` buffer and a word buffer, `n` samples each, every word and `packed_a` distinct,
/// sample `i`'s word empty; `stream`'s crossings through `cross` on a hook at `i`, each `packed_a` written back to
/// `SimState` `i`. Word `i` is then the reference's packing of `stream`; every other word, and every other `SimState`,
/// is as it was; `SimState` `i` differs at most in `last_symbol`; every access was at `i`.
fn check_binding(cross: Crossing, cases: &[(usize, usize, Vec<u32>)]) {
    for (n, i, stream) in cases {
        let (n, i) = (*n, *i);
        let states: Vec<SimStateFTLE> = (0..n)
            .map(|j| SimStateFTLE {
                packed_a: packed_a_of(j),
                times: j as u32,
                ..SimStateFTLE::default()
            })
            .collect();
        let mut sim = states.clone();
        let mut words = Counted::new(
            (0..n)
                .map(|j| FreeGroupWord {
                    free_group_word: if j == i { [0; 4] } else { word_of(j) },
                })
                .collect(),
        );
        let mut hook = CrossingHook::new(i);
        for &s in stream {
            sim[i].packed_a = cross(&mut hook, &mut words, sim[i].packed_a, s);
        }
        for (j, access) in words.since(0) {
            assert_eq!(
                access, i,
                "sample {i}'s crossing reached index {access} ({j:?})"
            );
        }
        assert_eq!(
            sample_word(&words.words[..], i),
            packed(&reduce(stream)),
            "sample {i}'s word is not the reference's packing of {stream:?}"
        );
        for j in (0..n).filter(|&j| j != i) {
            assert_eq!(
                sample_word(&words.words[..], j),
                word_of(j),
                "sample {i}'s crossings wrote sample {j}'s word"
            );
            assert_eq!(
                sim[j], states[j],
                "sample {i}'s crossings wrote sample {j}'s SimState"
            );
        }
        let mut kept = sim[i];
        kept.packed_a = set_last_symbol(kept.packed_a, 0);
        assert_eq!(kept, states[i], "SimState {i} changed outside last_symbol");
    }
}

/// Cases of [`check_binding`]: 1 to 8 samples, a sample index among them, a stream.
fn binding_cases() -> impl Strategy<Value = Vec<(usize, usize, Vec<u32>)>> {
    proptest::collection::vec(
        (1..=8usize, any::<usize>(), streams()).prop_map(|(n, i, s)| (n, i % n, s.concat())),
        1..6,
    )
}

#[test]
fn word_buffer_binding() {
    // A binding of its own: the word buffer's binding is not the SimState buffer's, in the same group (R-343), and its
    // element is the 16 B word, not a SimState member.
    assert_eq!(WORD_GROUP, SIMSTATE_GROUP, "the word buffer's group");
    assert_ne!(
        WORD_BINDING, SIMSTATE_BINDING,
        "the word buffer shares the SimState buffer's binding"
    );
    assert_eq!(size_of::<FreeGroupWord>(), 16, "the word buffer's element");
    // A substep with no crossing touches the word buffer not at all.
    let mut words = Counted::new(vec![FreeGroupWord::default(); 3]);
    let mut hook = CrossingHook::new(1);
    for _ in 0..64 {
        assert_eq!(
            on_substep(&mut hook, &mut words, 0x0a0b, SubstepCrossings::default()),
            0x0a0b
        );
    }
    assert!(
        words.since(0).is_empty(),
        "a substep with no crossing touched the word buffer"
    );
    check_binding(
        |h, w, pa, s| h.crossing(w, pa, s),
        &[(1, 0, all_a(77)), (5, 4, vec![0, 2, 3, 1, 1, 3])],
    );
    prop::run(&binding_cases(), |cases| {
        check_binding(|h, w, pa, s| h.crossing(w, pa, s), &cases);
        Ok(())
    });
}

/// A crossing that writes the word one index up: the next sample's word.
#[cfg(feature = "controls")]
fn cross_next_index(hook: &mut CrossingHook, words: &mut Counted, packed_a: u32, s: u32) -> u32 {
    let n = words.words.len();
    let a = fgw_append(words.load(hook.index), hook.prev, packed_a, s);
    words.store((hook.index + 1) % n, a.word);
    hook.prev = a.prev;
    a.packed_a
}

negative_control!(
    word_buffer_binding,
    "a crossing writing the next sample's word fails the same index",
    expected = "reached index",
    check_binding(cross_next_index, &[(4, 1, vec![2, 0])])
);

// ── word_write_count (REQ-PAY-042) ───────────────────────────────────────────────────────────────────────────────────

/// `crossings` (at most [`MAX_CROSSINGS_PER_SUBSTEP`]) as one substep's [`SubstepCrossings`].
fn substep(crossings: &[u32]) -> SubstepCrossings {
    let mut out = SubstepCrossings::default();
    assert!(
        crossings.len() <= MAX_CROSSINGS_PER_SUBSTEP as usize,
        "{crossings:?}: more crossings than one substep holds"
    );
    out.symbols[..crossings.len()].copy_from_slice(crossings);
    out.count = crossings.len() as u32;
    out
}

/// Each march `(i, substeps)` of a sample at index `i` of 4, each substep its crossings' symbols in order, run through
/// `step`: each substep writes the word once per crossing and reads it once per crossing, which the hook's append
/// needs, and not otherwise, all at `i`; the writes over the march equal its crossings; the word is the reference's
/// packing of the crossings.
fn check_write_count(step: Substep, marches: &[(usize, Vec<Vec<u32>>)]) {
    for (i, substeps) in marches {
        let i = *i;
        let mut words = Counted::new(vec![FreeGroupWord::default(); 4]);
        let mut hook = CrossingHook::new(i);
        let mut packed_a = 0;
        for (k, crossings) in substeps.iter().enumerate() {
            let from = words.log.borrow().len();
            packed_a = step(&mut hook, &mut words, packed_a, substep(crossings));
            let accesses = words.since(from);
            assert_eq!(
                count(&accesses, Access::Store),
                crossings.len(),
                "substep {k}, crossings {crossings:?}: word-buffer writes differ from the crossings"
            );
            assert_eq!(
                count(&accesses, Access::Load),
                crossings.len(),
                "substep {k}, crossings {crossings:?}: a word read in the per-step path"
            );
            assert!(
                accesses.iter().all(|&(_, j)| j == i),
                "substep {k}: an access off index {i}: {accesses:?}"
            );
        }
        let all: Vec<u32> = substeps.concat();
        assert_eq!(
            count(&words.since(0), Access::Store),
            all.len(),
            "the march's word-buffer writes differ from its crossing count"
        );
        assert_eq!(
            words.words[i].free_group_word,
            packed(&reduce(&all)),
            "the march's word is not the reference's packing of its crossings"
        );
    }
}

/// A synthetic march: up to 600 substeps, most with no crossing, some with one, a few with two, the capacity (a
/// segment's crossings, one per cut, in order).
fn marches() -> impl Strategy<Value = Vec<(usize, Vec<Vec<u32>>)>> {
    let substep = prop_oneof![
        20 => Just(Vec::new()),
        4 => proptest::collection::vec(0..4u32, 1..=1),
        1 => proptest::collection::vec(0..4u32, 2..=2),
    ];
    proptest::collection::vec(
        (0..4usize, proptest::collection::vec(substep, 0..=600)),
        1..4,
    )
}

/// The fixed march: quiet stretches, a crossing, a cancelling pair within one substep, and 80 crossings past the cap.
fn fixed_march() -> Vec<Vec<u32>> {
    let mut m = vec![vec![], vec![], vec![2], vec![], vec![0, 1], vec![], vec![3]];
    m.extend((0..80).map(|_| vec![0]));
    m.push(vec![]);
    m
}

#[test]
fn word_write_count() {
    check_write_count(on_substep, &[(2, fixed_march())]);
    prop::run(&marches(), |marches| {
        check_write_count(on_substep, &marches);
        Ok(())
    });
}

/// A substep that reads the word every step, as a march checking for truncation before it detects would.
#[cfg(feature = "controls")]
fn substep_reading(
    hook: &mut CrossingHook,
    words: &mut Counted,
    packed_a: u32,
    crossings: SubstepCrossings,
) -> u32 {
    let _ = words.load(hook.index);
    on_substep(hook, words, packed_a, crossings)
}

negative_control!(
    word_write_count,
    "a substep reading the word every step reads it in the per-step path",
    expected = "a word read in the per-step path",
    check_write_count(substep_reading, &[(0, vec![vec![], vec![1]])])
);

/// Marches whose every substep holds [`MAX_CROSSINGS_PER_SUBSTEP`] crossings, the capacity: two, one per cut.
fn full_marches() -> impl Strategy<Value = Vec<(usize, Vec<Vec<u32>>)>> {
    let substep = proptest::collection::vec(0..4u32, MAX_CROSSINGS_PER_SUBSTEP as usize);
    proptest::collection::vec(
        (0..4usize, proptest::collection::vec(substep, 1..=60)),
        1..4,
    )
}

#[test]
fn word_write_count_at_capacity() {
    assert_eq!(
        MAX_CROSSINGS_PER_SUBSTEP, 2,
        "one crossing per cut, two cuts"
    );
    // Full substeps: a cancelling pair, a push pair, a pair crossing the cap.
    let mut full = vec![vec![0, 1], vec![2, 0], vec![3, 1]];
    full.extend((0..40).map(|_| vec![0, 0]));
    check_write_count(on_substep, &[(1, full)]);
    prop::run(&full_marches(), |marches| {
        check_write_count(on_substep, &marches);
        Ok(())
    });
}

/// A substep that appends only its first crossing.
#[cfg(feature = "controls")]
fn substep_first_only(
    hook: &mut CrossingHook,
    words: &mut Counted,
    packed_a: u32,
    crossings: SubstepCrossings,
) -> u32 {
    let first = SubstepCrossings {
        count: crossings.count.min(1),
        ..crossings
    };
    on_substep(hook, words, packed_a, first)
}

negative_control!(
    word_write_count_at_capacity,
    "a substep appending only its first crossing drops the second at the capacity",
    expected = "word-buffer writes differ from the crossings",
    check_write_count(substep_first_only, &[(3, vec![vec![2, 0]])])
);

// ── last_symbol_mirror (REQ-PAY-048) ─────────────────────────────────────────────────────────────────────────────────

/// Each `(packed_a, stream)`: after each crossing of `stream` appended from the empty word with `packed_a`, where
/// 1 ≤ length ≤ 76, `last_symbol` is the final symbol of `decode(W)`; at every length, `packed_a`'s other bits are its
/// first ones.
fn check_mirror(append: Append, cases: &[(u32, Vec<u32>)]) {
    for (pa, stream) in cases {
        let mut a = empty(*pa);
        for (k, &s) in stream.iter().enumerate() {
            a = append(a.word, a.prev, a.packed_a, s);
            let length = fgw_length_raw(a.word);
            if (1..=FGW_CAPACITY).contains(&length) {
                let last = fgw_decode(a.word)
                    .take(FGW_CAPACITY as usize)
                    .last()
                    .expect("a word of length ≥ 1");
                assert_eq!(
                    sd_last_symbol(a.packed_a),
                    last,
                    "{stream:?}[..={k}], length {length}: last_symbol is not the final symbol of decode(W)"
                );
            }
            assert_eq!(
                a.packed_a & !(3 << 8),
                pa & !(3 << 8),
                "{stream:?}[..={k}]: packed_a's other bits changed"
            );
        }
    }
}

/// Cases of [`check_mirror`]: any `packed_a`, a stream.
fn mirror_cases() -> impl Strategy<Value = Vec<(u32, Vec<u32>)>> {
    proptest::collection::vec(
        (any::<u32>(), streams()).prop_map(|(pa, s)| (pa, s.concat())),
        1..6,
    )
}

#[test]
fn last_symbol_mirror() {
    // Lengths 1 and 76, the ends of the range, and a pop from each.
    let mut to_76 = reduced_word(&(0..76).map(|k| k * 7 + 1).collect::<Vec<u32>>());
    to_76.push(INVERSE[to_76[75] as usize]);
    check_mirror(
        fgw_append,
        &[
            (u32::MAX, to_76),
            (0, vec![3, 0, 1, 2, 2]),
            (0xffff_fcff, all_a(80)),
        ],
    );
    prop::run(&mirror_cases(), |cases| {
        check_mirror(fgw_append, &cases);
        Ok(())
    });
}

/// An append whose cancellation-pop leaves `last_symbol` at the popped symbol.
#[cfg(feature = "controls")]
fn append_pop_unmirrored(word: [u32; 4], prev: u32, packed_a: u32, s: u32) -> FgwAppended {
    let a = fgw_append(word, prev, packed_a, s);
    if fgw_length_raw(a.word) < fgw_length_raw(word) {
        FgwAppended { packed_a, ..a }
    } else {
        a
    }
}

negative_control!(
    last_symbol_mirror,
    "a pop that does not mirror the new prev leaves last_symbol stale",
    expected = "last_symbol is not the final symbol of decode(W)",
    check_mirror(append_pop_unmirrored, &[(0, vec![0, 2, 3])])
);

/// Each `(packed_a, sym)`, `sym` a symbol code: `set(packed_a, sym)` holds `sym` in `last_symbol` and every other bit of
/// `packed_a`.
fn check_set(set: Set, cases: &[(u32, u32)]) {
    for &(pa, sym) in cases {
        let out = set(pa, sym);
        assert_eq!(sd_last_symbol(out), sym, "set({pa:#x}, {sym}): last_symbol");
        assert_eq!(
            out & !(3 << 8),
            pa & !(3 << 8),
            "set({pa:#x}, {sym}): packed_a's other bits changed"
        );
    }
}

#[test]
fn last_symbol_mirror_set_keeps_other_bits() {
    check_set(
        set_last_symbol,
        &[(0, 3), (u32::MAX, 0), (0x0000_0300, 1), (0xffff_fcff, 2)],
    );
    prop::run(
        &proptest::collection::vec((any::<u32>(), 0..4u32), 1..32),
        |cases| {
            check_set(set_last_symbol, &cases);
            Ok(())
        },
    );
}

/// A setter that also clears descriptor bit 10, a reserved bit next to `last_symbol`.
#[cfg(feature = "controls")]
fn set_clearing_bit_10(pa: u32, sym: u32) -> u32 {
    set_last_symbol(pa, sym) & !(1 << 10)
}

negative_control!(
    last_symbol_mirror_set_keeps_other_bits,
    "a setter clearing bit 10 changes packed_a's other bits",
    expected = "packed_a's other bits changed",
    check_set(set_clearing_bit_10, &[(u32::MAX, 1)])
);

// ── word_horner_packing (REQ-PAY-052) ────────────────────────────────────────────────────────────────────────────────

/// `W` as the word's 121 payload bits hold it: `x`, `y`, `z`, then `.w` bits 0–24.
fn payload_bits(w: [u32; 4]) -> u128 {
    u128::from(w[0])
        | u128::from(w[1]) << 32
        | u128::from(w[2]) << 64
        | u128::from(w[3] & 0x01ff_ffff) << 96
}

/// Each freely-reduced word of `words`, appended from empty one symbol at a time: after `k` symbols the word's 121
/// payload bits hold payload §3's closed form of those `k`, which fits 121 bits, its `length` field (`.w` bits 25–31)
/// holds `k`, and the whole word is the reference's packing.
fn check_packing(append: Append, words: &[Vec<u32>]) {
    for symbols in words {
        let mut a = empty(0);
        assert_eq!(fgw_length_raw(a.word), 0, "the empty word's length");
        for k in 0..symbols.len() {
            a = append(a.word, a.prev, a.packed_a, symbols[k]);
            let prefix = &symbols[..=k];
            let w = closed_form(prefix);
            assert!(w < 1 << 121, "{prefix:?}: W = {w} does not fit 121 bits");
            assert_eq!(
                payload_bits(a.word),
                w,
                "{prefix:?} packs otherwise than the closed form"
            );
            assert_eq!(
                a.word[3] >> 25,
                k as u32 + 1,
                "{prefix:?}: the length field"
            );
            let reduced = Reduced {
                symbols: prefix.to_vec(),
                truncated: false,
            };
            assert_eq!(a.word, packed(&reduced), "{prefix:?}: the word");
        }
    }
}

#[test]
fn word_horner_packing() {
    // The largest W, d₀ = B and every digit 2, is 4·3⁷⁵ − 1, above 2¹²⁰: the payload's top bit, `.w` bit 24, is used
    // next to the length field.
    let mut largest = vec![3];
    for k in 1..76 {
        let p = largest[k - 1] as usize;
        largest.push(CONT_SYMBOL[2][p]);
    }
    let w = closed_form(&largest);
    assert_eq!(w, 4 * 3u128.pow(75) - 1, "the largest word's W");
    assert!(w >= 1 << 120, "the largest W uses bit 120");
    check_packing(fgw_append, &[largest, all_a(76), vec![1; 76]]);
    prop::run(
        &proptest::collection::vec(proptest::collection::vec(any::<u32>(), 76), 1..6),
        |raws| {
            let words: Vec<Vec<u32>> = raws.iter().map(|r| reduced_word(r)).collect();
            check_packing(fgw_append, &words);
            Ok(())
        },
    );
}

/// An append whose first symbol is not stored: `W₁ = 0`, whatever `d₀`.
#[cfg(feature = "controls")]
fn append_dropping_d0(word: [u32; 4], prev: u32, packed_a: u32, s: u32) -> FgwAppended {
    let a = fgw_append(word, prev, packed_a, s);
    if fgw_length_raw(word) == 0 && fgw_length_raw(a.word) == 1 {
        FgwAppended {
            word: kernel::payload::fgw_pack([0; 4], 1),
            ..a
        }
    } else {
        a
    }
}

negative_control!(
    word_horner_packing,
    "an append dropping d₀ packs otherwise than the closed form",
    expected = "packs otherwise than the closed form",
    check_packing(append_dropping_d0, &[vec![3, 3, 1]])
);

// ── word_vs_reference_reducer (REQ-PAY-053) ──────────────────────────────────────────────────────────────────────────

/// Each stream of `streams`, appended from empty: after every symbol, the word is the reference reducer's packing of
/// the stream so far, and `prev` its last symbol (none for the empty word, the 76th once truncated).
fn check_vs_reference(append: Append, streams: &[Vec<u32>]) {
    for stream in streams {
        let mut a = empty(0);
        for (k, &s) in stream.iter().enumerate() {
            a = append(a.word, a.prev, a.packed_a, s);
            let r = reduce(&stream[..=k]);
            assert_eq!(
                a.word,
                packed(&r),
                "{stream:?}[..={k}]: the word differs from the reference reducer's"
            );
            assert_eq!(
                a.prev,
                r.symbols.last().copied().unwrap_or(FGW_NO_SYMBOL),
                "{stream:?}[..={k}]: prev differs from the reference reducer's last symbol"
            );
        }
    }
}

#[test]
fn word_vs_reference_reducer() {
    let mut cap_pop = all_a(76);
    cap_pop.extend([1, 0, 0]);
    check_vs_reference(fgw_append, &[all_a(77), all_a(90), cap_pop, vec![1; 77]]);
    prop::run(&streams(), |streams| {
        check_vs_reference(fgw_append, &streams);
        Ok(())
    });
    prop::run(&small_w_streams(), |streams| {
        check_vs_reference(fgw_append, &streams);
        Ok(())
    });
}

/// An append whose capacity is numeric overflow: at 76 symbols a push still fits 121 bits pushes on, to length 77.
#[cfg(feature = "controls")]
fn append_overflow_cap(word: [u32; 4], prev: u32, packed_a: u32, s: u32) -> FgwAppended {
    use kernel::payload::{continuation_index, fgw_pack, inverse};
    let v = fgw_mixed_radix(word);
    if fgw_length_raw(word) == FGW_CAPACITY && s != inverse(prev) && v[3] < 1 << 23 {
        FgwAppended {
            word: fgw_pack(
                kernel::word::fgw_mul3_add(v, continuation_index(prev, s)),
                FGW_CAPACITY + 1,
            ),
            prev: s,
            packed_a: set_last_symbol(packed_a, s),
        }
    } else {
        fgw_append(word, prev, packed_a, s)
    }
}

negative_control!(
    word_vs_reference_reducer,
    "an overflow-capped append keeps a small-W 77-symbol word",
    expected = "the word differs from the reference reducer's",
    check_vs_reference(append_overflow_cap, &[all_a(77)])
);

// ── word_truncation_sentinel (REQ-TOOL-035) ──────────────────────────────────────────────────────────────────────────

/// Each `(symbols, e, later)`, `symbols` a 76-symbol freely-reduced word: appended from empty it reads length 76; a
/// pop there reads 75, as each pop on down to the empty word decrements the length; a 77th symbol continuing it by
/// digit `e` reads length 127, `fgw_truncated`, `W` kept; after it, the inverse of its 76th symbol and each of `later`
/// leave the word as it is.
fn check_sentinel(append: Append, cases: &[(Vec<u32>, u32, Vec<u32>)]) {
    for (symbols, e, later) in cases {
        let full = append_all(append, empty(0), symbols);
        assert_eq!(
            fgw_length_raw(full.word),
            76,
            "{symbols:?}: the length at the cap"
        );
        let mut popped = full;
        for n in (0..76).rev() {
            let s = INVERSE[popped.prev as usize];
            popped = append(popped.word, popped.prev, popped.packed_a, s);
            assert_eq!(
                fgw_length_raw(popped.word),
                n,
                "{symbols:?}: a pop does not decrement the length to {n}"
            );
        }
        let last = symbols[75] as usize;
        let t = append(
            full.word,
            full.prev,
            full.packed_a,
            CONT_SYMBOL[*e as usize][last],
        );
        assert_eq!(
            (fgw_length_raw(t.word), fgw_truncated(t.word)),
            (FGW_LENGTH_SENTINEL, true),
            "{symbols:?}: a 77th symbol does not read length 127"
        );
        assert_eq!(
            fgw_mixed_radix(t.word),
            fgw_mixed_radix(full.word),
            "{symbols:?}: truncation changed W"
        );
        let after = append_all(append, t, &[&[INVERSE[last]][..], later].concat());
        assert_eq!(
            after.word, t.word,
            "{symbols:?}: a crossing after truncation changed the word"
        );
    }
}

/// Cases of [`check_sentinel`]: a random 76-symbol word or a small-`W` one, a digit, a later stream.
fn sentinel_cases() -> impl Strategy<Value = Vec<(Vec<u32>, u32, Vec<u32>)>> {
    let word = prop_oneof![
        proptest::collection::vec(any::<u32>(), 76).prop_map(|r| reduced_word(&r)),
        (0..=75usize, proptest::collection::vec(any::<u32>(), 76))
            .prop_map(|(z, r)| small_w_77(z, &r, 0)[..76].to_vec()),
    ];
    proptest::collection::vec(
        (word, 0..3u32, proptest::collection::vec(0..4u32, 0..40)),
        1..6,
    )
}

#[test]
fn word_truncation_sentinel() {
    // `W = 0` (all `a`) and `W = 3⁷⁵` (all `A`): small numbers, 76 symbols.
    check_sentinel(
        fgw_append,
        &[(all_a(76), 0, vec![1, 1, 0]), (vec![1; 76], 2, vec![0])],
    );
    prop::run(&sentinel_cases(), |cases| {
        check_sentinel(fgw_append, &cases);
        Ok(())
    });
}

/// An append whose cap is one late: it pushes a 77th symbol, and truncates only a 78th.
#[cfg(feature = "controls")]
fn append_late_cap(word: [u32; 4], prev: u32, packed_a: u32, s: u32) -> FgwAppended {
    use kernel::payload::{continuation_index, fgw_pack, inverse};
    if fgw_length_raw(word) == FGW_CAPACITY && s != inverse(prev) {
        FgwAppended {
            word: fgw_pack(
                kernel::word::fgw_mul3_add(fgw_mixed_radix(word), continuation_index(prev, s)),
                FGW_CAPACITY + 1,
            ),
            prev: s,
            packed_a: set_last_symbol(packed_a, s),
        }
    } else {
        fgw_append(word, prev, packed_a, s)
    }
}

negative_control!(
    word_truncation_sentinel,
    "an append capping at 77 does not read 127 on the 77th symbol",
    expected = "a 77th symbol does not read length 127",
    check_sentinel(append_late_cap, &[(all_a(76), 0, vec![])])
);
