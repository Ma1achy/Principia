//! QA tests for TASK-M3-15, the word buffer's binding, the append through the crossing hook, and the length cap,
//! written from the requirements and dd_simstate_payload §0, §2, §3, §6, not from the implementation. Every
//! expectation comes from a model written here from payload §3 alone:
//! - the frozen table (`inverse = [1,0,3,2]`, `cont_symbol[e][prev]`), symbol codes `a = 0, A = 1, b = 2, B = 3`;
//! - free reduction as a symbol stack with payload §3's append rules (ignore once truncated; pop on the inverse; at 76
//!   symbols a non-cancelling symbol truncates; otherwise push);
//! - `W` by payload §3's closed form `W = d₀·3^(ℓ−1) + Σ_{k=1}^{ℓ−1} e_k·3^(ℓ−1−k)` (a sum of powers, not the Horner
//!   step), laid out by payload §3's `.w` bit map: low 96 bits in x, y, z, the next 25 in `.w[0:24]`, length in
//!   `.w[25:31]`.
//!
//! - REQ-PAY-036 / REQ-PAY-038 (`qa_m315_binding_*`): the word buffer is a binding of its own (its own binding slot,
//!   16 B entries, SimState 96 / 144 B with no word member); a crossing of sample `i` touches only index `i`; a substep
//!   with no crossing touches the buffer not at all.
//! - REQ-PAY-042 (`qa_m315_write_count_*`): over lockstep marches of several samples, word-buffer writes == crossings
//!   on every substep, and no read on a substep with no crossing.
//! - REQ-PAY-048 (`qa_m315_last_symbol_*`): after every crossing, with 1 ≤ length ≤ 76, `last_symbol` (packed_a bits
//!   8–9) is the word's final symbol; bits 0–7 and 10–31 never change; after truncation packed_a is frozen.
//! - REQ-PAY-052 (`qa_m315_packing_*`): random reduced words of 1…76 symbols pack as the closed form, within 121 bits,
//!   the length field their length; the largest 76-symbol word needs bit 120; `fgw_mul3_add` is `3v + e`.
//! - REQ-PAY-053 (`qa_m315_vs_reference_*`): random streams (cancellation-heavy, at-the-cap wobbles, small-W 77-symbol
//!   words) agree with the model after every symbol; the PR's `validation::reference::free_group` agrees with the model.
//! - REQ-TOOL-035 (`qa_m315_truncation_*`): a 77th non-cancelling symbol reads 127 for small W (W = 0 included); a
//!   cancelling one at 76 pops; pops from 76 to 0 decrement the length by one; truncated words ignore every later
//!   crossing; a 75-symbol word of large W is not truncated by its 76th.
//!
//! Every word access goes through a logging buffer whose log is capped, so a mutant that loops panics fast. Each check
//! takes the code under test as a function pointer; its negative control (R-176) runs the same check with one fault.

use std::cell::RefCell;
use std::mem::{align_of, size_of};

use kernel::payload::{
    fgw_length_raw, fgw_reduced_length_valid, fgw_retained_prefix_length, fgw_truncated,
    FreeGroupWord, SimStateBase, SimStateFTLE, MAX_CROSSINGS_PER_SUBSTEP, SIMSTATE_BINDING,
    SIMSTATE_GROUP, WORD_BINDING, WORD_GROUP,
};
use kernel::word::{
    fgw_mul3_add, on_substep, sample_word, CrossingHook, SubstepCrossings, WordBuffer,
};
use proptest::prelude::*;
use validation::reference::free_group as pr_ref;
use validation::{negative_control, prop};

// ── The model, from payload §3 ───────────────────────────────────────────────────────────────────────────────────

const INV: [u32; 4] = [1, 0, 3, 2];
const CONT: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];
const CAP: usize = 76;
const SENTINEL: u32 = 127;

fn digit(prev: u32, next: u32) -> u128 {
    (0..3usize)
        .find(|&e| CONT[e][prev as usize] == next)
        .expect("a reduced word never continues a symbol with its inverse") as u128
}

/// Payload §3's closed form, as a sum of powers of 3.
fn closed_form(syms: &[u32]) -> u128 {
    let l = syms.len() as u32;
    if l == 0 {
        return 0;
    }
    let mut w = syms[0] as u128 * 3u128.pow(l - 1);
    for k in 1..syms.len() {
        w += digit(syms[k - 1], syms[k]) * 3u128.pow(l - 1 - k as u32);
    }
    w
}

fn layout(w: u128, length: u32) -> [u32; 4] {
    assert!(w < 1u128 << 121, "the model's W exceeds 121 bits");
    [
        w as u32,
        (w >> 32) as u32,
        (w >> 64) as u32,
        (w >> 96) as u32 | (length << 25),
    ]
}

#[derive(Clone, Debug, Default)]
struct Model {
    syms: Vec<u32>,
    truncated: bool,
}

impl Model {
    fn push(&mut self, s: u32) {
        if self.truncated {
        } else if self.syms.last() == Some(&INV[s as usize]) {
            self.syms.pop();
        } else if self.syms.len() == CAP {
            self.truncated = true;
        } else {
            self.syms.push(s);
        }
    }
    fn length(&self) -> u32 {
        if self.truncated {
            SENTINEL
        } else {
            self.syms.len() as u32
        }
    }
    fn packed(&self) -> [u32; 4] {
        layout(closed_form(&self.syms), self.length())
    }
}

// ── The logging word buffer ──────────────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Load(usize),
    Store(usize),
}

/// Far above any check here (each check has at most ~1500 crossings); a looping mutant panics here.
const LOG_CAP: usize = 20_000;

struct Logged {
    words: Vec<FreeGroupWord>,
    log: RefCell<Vec<Op>>,
}

impl Logged {
    fn new(n: usize) -> Self {
        Self {
            words: vec![FreeGroupWord::default(); n],
            log: RefCell::new(Vec::new()),
        }
    }
    fn record(&self, op: Op) {
        let mut log = self.log.borrow_mut();
        assert!(log.len() < LOG_CAP, "qa: the access log passed its cap");
        log.push(op);
    }
    fn ops_since(&self, from: usize) -> Vec<Op> {
        self.log.borrow()[from..].to_vec()
    }
    fn len(&self) -> usize {
        self.log.borrow().len()
    }
    fn raw(&self, i: usize) -> [u32; 4] {
        self.words[i].free_group_word
    }
}

impl WordBuffer for Logged {
    fn load(&self, i: usize) -> [u32; 4] {
        self.record(Op::Load(i));
        self.words[..].load(i)
    }
    fn store(&mut self, i: usize, w: [u32; 4]) {
        self.record(Op::Store(i));
        self.words[..].store(i, w);
    }
}

type Substep = fn(&mut CrossingHook, &mut Logged, u32, SubstepCrossings) -> u32;

/// A substep's crossings, in order, as the march hands them to `on_substep`: at most
/// [`MAX_CROSSINGS_PER_SUBSTEP`] (one per cut, payload §3), the unused slots zero.
fn substep_crossings(c: &[u32]) -> SubstepCrossings {
    assert!(
        c.len() <= MAX_CROSSINGS_PER_SUBSTEP as usize,
        "qa: a substep drew more crossings than the capacity"
    );
    let mut out = SubstepCrossings::default();
    out.symbols[..c.len()].copy_from_slice(c);
    out.count = c.len() as u32;
    out
}
type Cross = fn(&mut CrossingHook, &mut Logged, u32, u32) -> u32;

fn real_substep(h: &mut CrossingHook, b: &mut Logged, pa: u32, c: SubstepCrossings) -> u32 {
    on_substep(h, b, pa, c)
}

fn real_cross(h: &mut CrossingHook, b: &mut Logged, pa: u32, s: u32) -> u32 {
    h.crossing(b, pa, s)
}

// ── Strategies ────────────────────────────────────────────────────────────────────────────────────────────────────

/// A reduced word of `1..=max` symbols from a first symbol and digits.
fn reduced_word(min: usize, max: usize) -> impl Strategy<Value = Vec<u32>> {
    (
        0u32..4,
        proptest::collection::vec(0usize..3, (min - 1)..max),
    )
        .prop_map(|(d0, es)| {
            let mut w = vec![d0];
            for e in es {
                let p = *w.last().unwrap();
                w.push(CONT[e][p as usize]);
            }
            w
        })
}

/// A stream where, at each symbol, the inverse of the model's last symbol is likely (cancellation-heavy).
fn cancelling_stream() -> impl Strategy<Value = Vec<u32>> {
    proptest::collection::vec((0u32..4, 0u32..3), 0..240).prop_map(|draws| {
        let mut m = Model::default();
        let mut out = Vec::new();
        for (s, coin) in draws {
            let s = match (coin, m.syms.last()) {
                (0, Some(&p)) => INV[p as usize],
                _ => s,
            };
            m.push(s);
            out.push(s);
        }
        out
    })
}

/// A stream that reaches 76 symbols and then wobbles at the cap: a reduced 76-symbol word, then a random tail.
fn at_cap_stream() -> impl Strategy<Value = Vec<u32>> {
    (
        reduced_word(76, 76),
        proptest::collection::vec(0u32..4, 0..16),
    )
        .prop_map(|(mut w, tail)| {
            w.extend(tail);
            w
        })
}

/// A 77-symbol stream of small W: `a` (d₀ = 0), `zeros` zero digits (more `a`), then a short reduced tail, so the first
/// 76 have W < 3^(76 − zeros); then a 77th non-cancelling symbol.
fn small_w_77() -> impl Strategy<Value = Vec<u32>> {
    (
        60usize..=75,
        proptest::collection::vec(0usize..3, 0..16),
        0usize..3,
    )
        .prop_map(|(zeros, es, e77)| {
            let mut w = vec![0u32; zeros.min(CAP)];
            let mut i = 0;
            while w.len() < CAP {
                let p = *w.last().unwrap();
                w.push(CONT[*es.get(i).unwrap_or(&0)][p as usize]);
                i += 1;
            }
            let p = *w.last().unwrap();
            w.push(CONT[e77][p as usize]);
            w
        })
}

fn any_stream() -> impl Strategy<Value = Vec<u32>> {
    prop_oneof![
        proptest::collection::vec(0u32..4, 0..240),
        cancelling_stream(),
        at_cap_stream(),
        small_w_77(),
    ]
}

/// A lockstep march: `n` samples, each a list of substeps, each with 0 to `MAX_CROSSINGS_PER_SUBSTEP` (2, one per cut)
/// crossings (mostly none).
fn march() -> impl Strategy<Value = Vec<Vec<Vec<u32>>>> {
    let substep = prop_oneof![
        6 => Just(Vec::new()),
        2 => proptest::collection::vec(0u32..4, 1..=1),
        1 => proptest::collection::vec(0u32..4, 2..=MAX_CROSSINGS_PER_SUBSTEP as usize),
    ];
    proptest::collection::vec(proptest::collection::vec(substep, 0..120), 1..=5)
}

// ── REQ-PAY-036 / REQ-PAY-038 / REQ-PAY-042: the binding, its index, the write count ──────────────────────────────

/// Runs a lockstep march: for each substep index, each sample in turn. On every substep: writes == crossings; no access
/// at all with no crossing; reads ≤ crossings; every access at the sample's own index; and every word in the buffer is
/// the model's (compared at once, never accumulated).
fn check_march(step: Substep, march: &[Vec<Vec<u32>>]) -> Result<(), TestCaseError> {
    let n = march.len();
    let mut buf = Logged::new(n);
    let mut hooks: Vec<CrossingHook> = (0..n).map(CrossingHook::new).collect();
    let mut models = vec![Model::default(); n];
    let mut pa = vec![0u32; n];
    let mut crossings = 0usize;
    let steps = march.iter().map(Vec::len).max().unwrap_or(0);
    for t in 0..steps {
        for i in 0..n {
            let Some(c) = march[i].get(t) else { continue };
            let from = buf.len();
            pa[i] = step(&mut hooks[i], &mut buf, pa[i], substep_crossings(c));
            let ops = buf.ops_since(from);
            for op in &ops {
                let (Op::Load(j) | Op::Store(j)) = *op;
                prop_assert_eq!(
                    j,
                    i,
                    "a crossing of sample {} touched the word at index {}",
                    i,
                    j
                );
            }
            if c.is_empty() {
                prop_assert!(
                    ops.is_empty(),
                    "the word buffer was touched on a substep with no crossing: {:?}",
                    ops
                );
            }
            let stores = ops.iter().filter(|o| matches!(o, Op::Store(_))).count();
            let loads = ops.len() - stores;
            prop_assert_eq!(
                stores,
                c.len(),
                "word-buffer writes != crossings on a substep"
            );
            prop_assert!(
                loads <= c.len(),
                "more word reads than crossings on a substep"
            );
            crossings += c.len();
            for &s in c {
                models[i].push(s);
            }
            for (j, m) in models.iter().enumerate() {
                prop_assert_eq!(
                    buf.raw(j),
                    m.packed(),
                    "the word at index {} is not the model's",
                    j
                );
            }
        }
    }
    let stores = buf
        .log
        .borrow()
        .iter()
        .filter(|o| matches!(o, Op::Store(_)))
        .count();
    prop_assert_eq!(
        stores,
        crossings,
        "word-buffer writes over the march != crossings"
    );
    for (i, m) in models.iter().enumerate() {
        prop_assert_eq!(
            sample_word(&buf, i),
            m.packed(),
            "sample_word({}) is not the model's word",
            i
        );
    }
    Ok(())
}

#[test]
fn qa_m315_binding_same_index() {
    prop::run(&march(), |m| check_march(real_substep, &m));
}

/// A substep whose hook points at the next sample's word.
#[cfg(feature = "controls")]
fn substep_next_index(h: &mut CrossingHook, b: &mut Logged, pa: u32, c: SubstepCrossings) -> u32 {
    let mut other = *h;
    other.index = (h.index + 1) % b.words.len().max(1);
    let out = on_substep(&mut other, b, pa, c);
    h.prev = other.prev;
    out
}

negative_control!(
    qa_m315_binding_same_index,
    "a hook writing the next sample's word must fail (needs a march of 2+ samples with a crossing)",
    expected = "touched the word at index",
    check_march(substep_next_index, &[vec![vec![0]], vec![vec![]]]).unwrap()
);

#[test]
fn qa_m315_write_count_equals_crossings() {
    prop::run(&march(), |m| check_march(real_substep, &m));
}

/// A substep that reads the word every substep, crossing or not.
#[cfg(feature = "controls")]
fn substep_reading_always(
    h: &mut CrossingHook,
    b: &mut Logged,
    pa: u32,
    c: SubstepCrossings,
) -> u32 {
    let _ = b.load(h.index);
    on_substep(h, b, pa, c)
}

negative_control!(
    qa_m315_write_count_equals_crossings,
    "a per-step path that reads the word on every substep must fail",
    expected = "touched on a substep with no crossing",
    prop::run(&march(), |m| check_march(substep_reading_always, &m))
);

#[test]
fn qa_m315_write_count_one_store_per_crossing() {
    // A fixed march: a sample with two crossings (the capacity, one per cut) in one substep, then a pop, empty
    // substeps, a push, a full substep that pushes and pops, beside an idle sample.
    let m = vec![
        vec![vec![0, 2], vec![3], vec![], vec![2], vec![2, 3], vec![]],
        vec![vec![], vec![], vec![]],
    ];
    check_march(real_substep, &m).unwrap();
}

/// A substep that applies only the first of its crossings (one write per substep, not per crossing).
#[cfg(feature = "controls")]
fn substep_first_only(h: &mut CrossingHook, b: &mut Logged, pa: u32, c: SubstepCrossings) -> u32 {
    let first = SubstepCrossings {
        count: c.count.min(1),
        ..c
    };
    on_substep(h, b, pa, first)
}

negative_control!(
    qa_m315_write_count_one_store_per_crossing,
    "a substep that writes once however many crossings it has must fail",
    expected = "word-buffer writes != crossings",
    check_march(
        substep_first_only,
        &[
            vec![vec![0, 2], vec![3], vec![], vec![2], vec![2, 3], vec![]],
            vec![vec![], vec![], vec![]]
        ]
    )
    .unwrap()
);

/// The binding's layout: the word buffer has a slot of its own, apart from SimState's; its entry is one 16 B
/// `vec4<u32>`; SimState is 96 / 144 B at align 8, so it holds no 16-aligned word member (payload §0).
fn check_layout(word_slot: (u32, u32), state_slot: (u32, u32), sizes: [(usize, usize); 3]) {
    assert_ne!(
        word_slot, state_slot,
        "the word buffer shares SimState's binding slot"
    );
    assert_eq!(
        sizes[0],
        (16, 16),
        "the word entry is not one 16 B vec4<u32>"
    );
    assert_eq!(
        sizes[1],
        (96, 8),
        "SimStateBase is not 96 B at align 8 (no word member)"
    );
    assert_eq!(
        sizes[2],
        (144, 8),
        "SimStateFTLE is not 144 B at align 8 (no word member)"
    );
}

fn layout_sizes() -> [(usize, usize); 3] {
    [
        (size_of::<FreeGroupWord>(), align_of::<FreeGroupWord>()),
        (size_of::<SimStateBase>(), align_of::<SimStateBase>()),
        (size_of::<SimStateFTLE>(), align_of::<SimStateFTLE>()),
    ]
}

#[test]
fn qa_m315_binding_is_its_own() {
    check_layout(
        (WORD_GROUP, WORD_BINDING),
        (SIMSTATE_GROUP, SIMSTATE_BINDING),
        layout_sizes(),
    );
}

negative_control!(
    qa_m315_binding_is_its_own,
    "a word inline in SimState (its size grown by 16 B, its slot SimState's) must fail",
    expected = "shares SimState's binding slot",
    check_layout(
        (SIMSTATE_GROUP, SIMSTATE_BINDING),
        (SIMSTATE_GROUP, SIMSTATE_BINDING),
        layout_sizes()
    )
);

// ── REQ-PAY-048: last_symbol mirrors prev ─────────────────────────────────────────────────────────────────────────

/// Streams through the hook one crossing at a time from `pa0`. After every crossing: with 1 ≤ length ≤ 76,
/// `last_symbol` is the model's final symbol; bits other than 8–9 are `pa0`'s; once truncated, packed_a is unchanged.
fn check_mirror(cross: Cross, stream: &[u32], pa0: u32) -> Result<(), TestCaseError> {
    let mut buf = Logged::new(1);
    let mut hook = CrossingHook::new(0);
    let mut m = Model::default();
    let mut pa = pa0;
    for &s in stream {
        let was_truncated = m.truncated;
        let before = pa;
        pa = cross(&mut hook, &mut buf, pa, s);
        m.push(s);
        prop_assert_eq!(buf.raw(0), m.packed(), "the word is not the model's");
        prop_assert_eq!(
            pa & !(0b11 << 8),
            pa0 & !(0b11 << 8),
            "a packed_a bit other than 8–9 changed"
        );
        let len = fgw_length_raw(buf.raw(0));
        if (1..=76).contains(&len) {
            prop_assert_eq!(
                (pa >> 8) & 3,
                *m.syms.last().unwrap(),
                "last_symbol is not the word's final symbol"
            );
        }
        if was_truncated {
            prop_assert_eq!(
                pa,
                before,
                "packed_a changed on a crossing after truncation"
            );
        }
    }
    Ok(())
}

fn mirror_cases() -> impl Strategy<Value = (Vec<u32>, u32)> {
    (any_stream(), any::<u32>())
}

#[test]
fn qa_m315_last_symbol_mirrors_prev() {
    prop::run(&mirror_cases(), |(s, pa)| check_mirror(real_cross, &s, pa));
}

/// A crossing that leaves last_symbol as it was on a pop (mirrors only the push).
#[cfg(feature = "controls")]
fn cross_pop_unmirrored(h: &mut CrossingHook, b: &mut Logged, pa: u32, s: u32) -> u32 {
    let before = fgw_length_raw(b.raw(h.index));
    let out = h.crossing(b, pa, s);
    let after = fgw_length_raw(b.raw(h.index));
    if after < before {
        (out & !(0b11 << 8)) | (pa & (0b11 << 8))
    } else {
        out
    }
}

negative_control!(
    qa_m315_last_symbol_mirrors_prev,
    "a pop that does not mirror the new prev must fail",
    expected = "last_symbol is not the word's final symbol",
    // a b A: after the pop of b by B, last_symbol must be a (0), not b (2).
    check_mirror(cross_pop_unmirrored, &[0, 2, 3], 0).unwrap()
);

#[test]
fn qa_m315_last_symbol_keeps_other_bits() {
    prop::run(&mirror_cases(), |(s, pa)| check_mirror(real_cross, &s, pa));
}

/// A crossing whose set also clears bit 10, the first reserved descriptor bit.
#[cfg(feature = "controls")]
fn cross_clearing_bit_10(h: &mut CrossingHook, b: &mut Logged, pa: u32, s: u32) -> u32 {
    h.crossing(b, pa, s) & !(1 << 10)
}

negative_control!(
    qa_m315_last_symbol_keeps_other_bits,
    "a set that clobbers a reserved bit must fail",
    expected = "a packed_a bit other than 8–9 changed",
    check_mirror(cross_clearing_bit_10, &[0], u32::MAX).unwrap()
);

// ── REQ-PAY-052: the Horner packing in 121 bits ───────────────────────────────────────────────────────────────────

/// Pushes the reduced `word` through the hook and checks the stored word bit for bit against the closed form, W within
/// 121 bits, the length field `word.len()`.
fn check_packing(cross: Cross, word: &[u32]) -> Result<(), TestCaseError> {
    let mut buf = Logged::new(1);
    let mut hook = CrossingHook::new(0);
    let mut pa = 0;
    for &s in word {
        pa = cross(&mut hook, &mut buf, pa, s);
    }
    let w = closed_form(word);
    prop_assert!(w < 1u128 << 121, "a 76-symbol word does not fit 121 bits");
    let got = buf.raw(0);
    prop_assert_eq!(
        got,
        layout(w, word.len() as u32),
        "packing is not the closed form for {:?}",
        word
    );
    prop_assert_eq!(
        got[3] >> 25,
        word.len() as u32,
        "the length field is not .w bits 25–31"
    );
    Ok(())
}

#[test]
fn qa_m315_packing_closed_form() {
    prop::run(
        &prop_oneof![reduced_word(76, 76), reduced_word(1, 76)],
        |w| check_packing(real_cross, &w),
    );
    // The extremes: the largest 76-symbol W (B, then digit 2 throughout) needs bit 120; all-`a` is W = 0.
    let mut big = vec![3u32];
    while big.len() < CAP {
        let p = *big.last().unwrap();
        big.push(CONT[2][p as usize]);
    }
    assert!(
        closed_form(&big) >= 1u128 << 120,
        "the largest word does not reach bit 120"
    );
    assert_eq!(closed_form(&big), 4 * 3u128.pow(75) - 1);
    check_packing(real_cross, &big).unwrap();
    check_packing(real_cross, &[0; CAP]).unwrap();
}

/// A crossing that loses `.w` bit 24, W's bit 120.
#[cfg(feature = "controls")]
fn cross_losing_bit_120(h: &mut CrossingHook, b: &mut Logged, pa: u32, s: u32) -> u32 {
    let out = h.crossing(b, pa, s);
    let mut w = b.raw(h.index);
    w[3] &= !(1 << 24);
    b.words[h.index].free_group_word = w;
    out
}

negative_control!(
    qa_m315_packing_closed_form,
    "a packing without W's top bit must fail on the largest word",
    expected = "packing is not the closed form",
    {
        let mut big = vec![3u32];
        while big.len() < CAP {
            let p = *big.last().unwrap();
            big.push(CONT[2][p as usize]);
        }
        check_packing(cross_losing_bit_120, &big).unwrap()
    }
);

type Mul3 = fn([u32; 4], u32) -> [u32; 4];

fn check_mul3(mul3: Mul3, v: u128, e: u32) -> Result<(), TestCaseError> {
    let limbs = [
        v as u32,
        (v >> 32) as u32,
        (v >> 64) as u32,
        (v >> 96) as u32,
    ];
    let r = mul3(limbs, e);
    let got = r[0] as u128 | (r[1] as u128) << 32 | (r[2] as u128) << 64 | (r[3] as u128) << 96;
    prop_assert_eq!(got, 3 * v + e as u128, "fgw_mul3_add is not 3v + e");
    Ok(())
}

#[test]
fn qa_m315_packing_mul3_add() {
    prop::run(&(0u128..(1u128 << 126), 0u32..3), |(v, e)| {
        check_mul3(fgw_mul3_add, v, e)
    });
    // Carries through every limb.
    check_mul3(fgw_mul3_add, (1u128 << 126) - 1, 2).unwrap();
    check_mul3(
        fgw_mul3_add,
        0x5555_5555_5555_5555_5555_5555_5555_5555 >> 1,
        2,
    )
    .unwrap();
}

/// `fgw_mul3_add` dropping the carry into the top limb.
#[cfg(feature = "controls")]
fn mul3_no_top_carry(v: [u32; 4], e: u32) -> [u32; 4] {
    let mut r = fgw_mul3_add(v, e);
    r[3] = v[3].wrapping_mul(3);
    r
}

negative_control!(
    qa_m315_packing_mul3_add,
    "a multiply losing the carry into limb 3 must fail",
    expected = "fgw_mul3_add is not 3v + e",
    check_mul3(mul3_no_top_carry, (1u128 << 126) - 1, 2).unwrap()
);

// ── REQ-PAY-053: against the reference reducer ────────────────────────────────────────────────────────────────────

/// Streams through the hook; after every symbol, the stored word is the model's, bit for bit.
fn check_vs_model(cross: Cross, stream: &[u32]) -> Result<(), TestCaseError> {
    let mut buf = Logged::new(1);
    let mut hook = CrossingHook::new(0);
    let mut m = Model::default();
    let mut pa = 0;
    for (k, &s) in stream.iter().enumerate() {
        pa = cross(&mut hook, &mut buf, pa, s);
        m.push(s);
        prop_assert_eq!(
            buf.raw(0),
            m.packed(),
            "the word is not the reference's after symbol {}",
            k
        );
    }
    Ok(())
}

#[test]
fn qa_m315_vs_reference_streams() {
    prop::run(&any_stream(), |s| check_vs_model(real_cross, &s));
}

/// A crossing that checks the cap before the free reduction: a cancelling symbol at 76 truncates.
#[cfg(feature = "controls")]
fn cross_cap_first(h: &mut CrossingHook, b: &mut Logged, pa: u32, s: u32) -> u32 {
    let w = b.raw(h.index);
    if fgw_length_raw(w) == 76 {
        let mut t = w;
        t[3] = (t[3] & 0x01ff_ffff) | (SENTINEL << 25);
        b.store(h.index, t);
        return pa;
    }
    h.crossing(b, pa, s)
}

negative_control!(
    qa_m315_vs_reference_streams,
    "an append checking the cap before the pop must fail",
    expected = "the word is not the reference's",
    prop::run(&at_cap_stream(), |s| {
        // Force a cancelling symbol right at the cap.
        let mut s = s;
        s.truncate(CAP);
        let last = s[CAP - 1];
        s.push(INV[last as usize]);
        check_vs_model(cross_cap_first, &s)
    })
);

#[test]
fn qa_m315_vs_reference_small_w_77() {
    prop::run(&small_w_77(), |s| check_vs_model(real_cross, &s));
}

/// A crossing that truncates on numeric overflow (W ≥ 2^119 before the push), not the length cap.
#[cfg(feature = "controls")]
fn cross_overflow_rule(h: &mut CrossingHook, b: &mut Logged, pa: u32, s: u32) -> u32 {
    let w = b.raw(h.index);
    let len = fgw_length_raw(w);
    if len == 76 && w[3] & 0x01ff_ffff < (1 << 23) {
        // W < 2^119: 3W + e fits, so the overflow rule would push a 77th; show that as length 77.
        let mut t = w;
        t[3] = (t[3] & 0x01ff_ffff) | (77 << 25);
        b.store(h.index, t);
        return pa;
    }
    h.crossing(b, pa, s)
}

negative_control!(
    qa_m315_vs_reference_small_w_77,
    "a truncation by numeric overflow instead of the length cap must fail on a small-W 77-symbol word",
    expected = "the word is not the reference's",
    prop::run(&small_w_77(), |s| check_vs_model(cross_overflow_rule, &s))
);

type Reduce = fn(&[u32]) -> (Vec<u32>, bool, [u32; 4]);

fn pr_reduce(stream: &[u32]) -> (Vec<u32>, bool, [u32; 4]) {
    let r = pr_ref::reduce(stream);
    let p = pr_ref::packed(&r);
    (r.symbols, r.truncated, p)
}

/// The PR's reference reducer (a deliverable) against the model here.
fn check_pr_reference(reduce: Reduce, stream: &[u32]) -> Result<(), TestCaseError> {
    let mut m = Model::default();
    for &s in stream {
        m.push(s);
    }
    let (syms, truncated, packed) = reduce(stream);
    prop_assert_eq!(
        &syms,
        &m.syms,
        "the reference reducer's symbols are not the model's"
    );
    prop_assert_eq!(
        truncated,
        m.truncated,
        "the reference reducer's truncation is not the model's"
    );
    prop_assert_eq!(
        packed,
        m.packed(),
        "the reference reducer's packing is not the model's"
    );
    Ok(())
}

#[test]
fn qa_m315_vs_reference_pr_reducer() {
    prop::run(&any_stream(), |s| check_pr_reference(pr_reduce, &s));
}

#[cfg(feature = "controls")]
fn pr_reduce_never_truncated(stream: &[u32]) -> (Vec<u32>, bool, [u32; 4]) {
    let (s, _, p) = pr_reduce(stream);
    (s, false, p)
}

negative_control!(
    qa_m315_vs_reference_pr_reducer,
    "a reducer that never reports truncation must fail",
    expected = "the reference reducer's truncation is not the model's",
    check_pr_reference(pr_reduce_never_truncated, &[0; 77]).unwrap()
);

// ── REQ-TOOL-035: the truncation sentinel ─────────────────────────────────────────────────────────────────────────

/// Unit checks of the length cap through the hook:
/// - `a`×76 (W = 0) and `a`×75 `b` (W = 1) plus each non-cancelling 77th symbol: length 127, `fgw_truncated`,
///   reduced length invalid, retained prefix 76, W's bits kept; then every symbol (inverse included) is ignored;
/// - the inverse at 76 pops to 75, not truncated;
/// - pops from 76 down to 0 decrement the length by one each;
/// - a 75-symbol word of large W takes its 76th symbol without truncating.
fn check_truncation(cross: Cross) {
    let feed = |stream: &[u32]| {
        let mut buf = Logged::new(1);
        let mut hook = CrossingHook::new(0);
        let mut pa = 0;
        let mut lens = Vec::new();
        for &s in stream {
            pa = cross(&mut hook, &mut buf, pa, s);
            lens.push(fgw_length_raw(buf.raw(0)));
        }
        (buf.raw(0), lens)
    };
    let mut w1 = vec![0u32; 75];
    w1.push(2); // a→b is digit 1: W = 1
    for base in [vec![0u32; 76], w1] {
        let last = *base.last().unwrap();
        let w76 = layout(closed_form(&base), 76);
        assert!(closed_form(&base) <= 1, "the fixture's W is not small");
        for s in (0..4).filter(|&s| s != INV[last as usize]) {
            let mut stream = base.clone();
            stream.push(s);
            let (w, _) = feed(&stream);
            assert_eq!(
                fgw_length_raw(w),
                127,
                "a 77th symbol does not read length 127 (W small)"
            );
            assert!(fgw_truncated(w), "a 77th symbol does not read truncated");
            assert!(
                !fgw_reduced_length_valid(w),
                "a truncated word's reduced length reads valid"
            );
            assert_eq!(
                fgw_retained_prefix_length(w),
                76,
                "the retained prefix is not 76"
            );
            assert_eq!(w[..3], w76[..3], "truncation changed W's low bits");
            assert_eq!(
                w[3] & 0x01ff_ffff,
                w76[3] & 0x01ff_ffff,
                "truncation changed W's high bits"
            );
            for later in [0u32, 1, 2, 3, INV[last as usize], INV[s as usize]] {
                let mut more = stream.clone();
                more.push(later);
                assert_eq!(
                    feed(&more).0,
                    w,
                    "a crossing after truncation changed the word"
                );
            }
        }
        let mut pop = base.clone();
        pop.push(INV[last as usize]);
        let (w, _) = feed(&pop);
        assert_eq!(
            fgw_length_raw(w),
            75,
            "the inverse at 76 symbols is not a pop"
        );
    }
    // Pops from 76 down to 0.
    let mut word = vec![1u32];
    while word.len() < CAP {
        let p = *word.last().unwrap();
        word.push(CONT[word.len() % 3][p as usize]);
    }
    let mut stream = word.clone();
    for &s in word.iter().rev() {
        stream.push(INV[s as usize]);
    }
    let (w, lens) = feed(&stream);
    for k in 0..CAP {
        assert_eq!(
            lens[CAP + k],
            (CAP - 1 - k) as u32,
            "a reduction pop does not decrement the length"
        );
    }
    assert_eq!(w, [0; 4], "popping every symbol does not clear the word");
    // 75 symbols of large W, then a 76th: not truncated.
    let mut big = vec![3u32];
    while big.len() < CAP {
        let p = *big.last().unwrap();
        big.push(CONT[2][p as usize]);
    }
    let (w, _) = feed(&big);
    assert_eq!(
        fgw_length_raw(w),
        76,
        "the 76th symbol of a large-W word truncated it"
    );
}

#[test]
fn qa_m315_truncation_sentinel() {
    check_truncation(real_cross);
}

/// A crossing whose cap is one late: the 77th symbol reads length 77.
#[cfg(feature = "controls")]
fn cross_late_cap(h: &mut CrossingHook, b: &mut Logged, pa: u32, s: u32) -> u32 {
    let out = h.crossing(b, pa, s);
    let mut w = b.raw(h.index);
    if fgw_length_raw(w) == 127 {
        w[3] = (w[3] & 0x01ff_ffff) | (77 << 25);
        b.words[h.index].free_group_word = w;
    }
    out
}

negative_control!(
    qa_m315_truncation_sentinel,
    "a cap that lets a 77th symbol in must fail",
    expected = "a 77th symbol does not read length 127",
    check_truncation(cross_late_cap)
);
