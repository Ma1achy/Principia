//! QA tests for TASK-M3-16, branch-cut crossing detection and the τ tie rule, written from the requirements and the
//! contract text, not from the implementation. Every expectation comes from a model written here:
//! - REQ-INT-031, REQ-INT-047 (`qa_m316_tau_against_the_formula`): the crossing order against the statement's own
//!   `τ = d₀/(d₀ − d₁)`, as exact rationals over integer distances, and unchanged when a cut's distances are scaled by a
//!   power of two (`τ` does not depend on the scale), over the whole exponent range.
//! - REQ-INT-046, REQ-INT-047 (`qa_m316_tau_is_chronological`): `τ` is the crossing's time: one substep's order equals
//!   the order the same straight segment gives when subdivided; and the segment run backwards reads the inverse symbols
//!   in the reverse order.
//! - REQ-INT-079 (`qa_m316_tie_tolerance_is_the_working_precision`): the tolerance is the working precision's rounding
//!   of the two products (integrator contract Part 5, "The tie tolerance").
//! - REQ-INT-048 (`qa_m316_half_open_literal`): the statement's literal rule, `d < 0` then `d ≥ 0` and the reverse, on
//!   each of two independent cuts, over marches of arbitrary doubles with zeros among them.
//! - REQ-INT-031, REQ-INT-047 (`qa_m316_double_crossing_reaches_the_word`): a double crossing is appended to the word in
//!   `τ` order through the hook, one load and one store per crossing, none on a quiet substep.
//! - REQ-PAY-070, REQ-PAY-113 (`qa_m316_syzygy_known_answers`): hand-worked collinear configurations of §1, from
//!   positions: each segment, each sense, an endpoint exactly on the equator, and the gate at the non-negative endpoint.
//! - REQ-PAY-113 (`qa_m316_cuts_invariances`): `d` and the classifier read positions only: translation, rotation, scale
//!   over the whole exponent range; exchanging bodies 0 and 2 negates `d` and exchanges the gates.
//! - REQ-PAY-070 (`qa_m316_march_reversed_reads_the_inverse`): a march run backwards reads the inverse word, symbol by
//!   symbol, endpoints on the equator included, which is what the gate at the non-negative endpoint is for.
//! - REQ-PAY-070 (`qa_m316_prl_fixture_against_appendix_c`): the fixture's published words against Table I's printed
//!   form; §1's stated tally of symmetries; the kernel's word against Appendix C's digit reading (`a = 12, A = 21,
//!   b = 32, B = 23`, `11 = 22 = 33 = empty`) done separately here by plain arithmetic; and §1's claim for the other
//!   outer-body order.
//!
//! Each check takes the code under test as a function, so its negative control (R-176) runs the same check on a faulty
//! one.

use kernel::payload::{fgw_length_raw, FreeGroupWord, FGW_NO_SYMBOL};
use kernel::real::IeeeBits;
use kernel::word::{
    fgw_decode, state_cuts, substep_crossings, CrossingDetector, CrossingHook, CutDistances,
    SubstepCrossings, WordBuffer,
};
use proptest::prelude::*;
use std::cell::Cell;
use std::cmp::Ordering;
use validation::{negative_control, prop};

const A: u32 = 0;
const INV_A: u32 = 1;
const B: u32 = 2;
const INV_B: u32 = 3;

/// A float the tests run at.
trait Q: IeeeBits {
    /// `v`, exact where the test's values are.
    fn of(v: f64) -> Self;
    /// A power of two whose factors keep every test value normal and finite, at this precision.
    const SCALE: i32;
    /// Two products apart by more than `2^−TOL_BITS` relative are not rounded together; an integer distance of up to 25
    /// bits times another is exact at f64, so its bound lies past any gap there.
    const TOL_BITS: u32;
}

impl Q for f32 {
    fn of(v: f64) -> f32 {
        v as f32
    }
    const SCALE: i32 = 100;
    const TOL_BITS: u32 = 22;
}

impl Q for f64 {
    fn of(v: f64) -> f64 {
        v
    }
    const SCALE: i32 = 900;
    const TOL_BITS: u32 = 60;
}

type Detect<T> = fn(CutDistances<T>, CutDistances<T>) -> Vec<u32>;

fn detect<T: IeeeBits>(start: CutDistances<T>, end: CutDistances<T>) -> Vec<u32> {
    let c = substep_crossings(start, end);
    c.symbols[..c.count as usize].to_vec()
}

/// `2^k`, exact for `k` within f64's normal range.
fn two_to(k: i32) -> f64 {
    2f64.powi(k)
}

/// The cuts' distances at one endpoint, integers scaled by each cut's own power of two.
fn at<T: Q>(a: i64, ka: i32, b: i64, kb: i32) -> CutDistances<T> {
    CutDistances::ungated(T::of(a as f64 * two_to(ka)), T::of(b as f64 * two_to(kb)))
}

/// One cut crossed over a segment, as integers `(d₀, d₁)`: one side strictly negative, the other non-negative.
fn crossed_cut() -> impl Strategy<Value = (i64, i64)> {
    let m = || prop_oneof![1i64..=16, 1i64..=(1 << 24)];
    (m(), prop_oneof![1 => Just(0i64), 6 => m()], any::<bool>()).prop_map(
        |(neg, nonneg, positive)| {
            if positive {
                (-neg, nonneg)
            } else {
                (nonneg, -neg)
            }
        },
    )
}

/// `τ = d₀/(d₀ − d₁)` as a rational with a positive denominator.
fn tau(c: (i64, i64)) -> (i128, i128) {
    let n = c.0 as i128;
    let den = (c.0 - c.1) as i128;
    if den < 0 {
        (-n, -den)
    } else {
        (n, den)
    }
}

fn tau_order(a: (i64, i64), b: (i64, i64)) -> Ordering {
    let (na, da) = tau(a);
    let (nb, db) = tau(b);
    (na * db).cmp(&(nb * da))
}

/// The cut's symbol from its crossing's sense: negative → non-negative is the generator.
fn symbol(c: (i64, i64), generator: u32) -> u32 {
    if c.0 < 0 {
        generator
    } else {
        generator + 1
    }
}

// ── qa_m316_tau_against_the_formula ──────────────────────────────────────────────────────────────────────────────────

fn check_tau<T: Q>(detect: Detect<T>) {
    let k = || -T::SCALE..=T::SCALE;
    prop::run(
        &(crossed_cut(), crossed_cut(), k(), k()),
        |(ca, cb, ka, kb)| {
            let got = detect(at::<T>(ca.0, 0, cb.0, 0), at::<T>(ca.1, 0, cb.1, 0));
            let a_first = vec![symbol(ca, A), symbol(cb, B)];
            let b_first = vec![symbol(cb, B), symbol(ca, A)];
            match tau_order(ca, cb) {
                Ordering::Less | Ordering::Equal => {
                    prop_assert_eq!(&got, &a_first, "τ order, τ_a ≤ τ_b: {:?} {:?}", ca, cb)
                }
                Ordering::Greater => {
                    let p1 = (cb.0 as i128 * ca.1 as i128).abs();
                    let p2 = (ca.0 as i128 * cb.1 as i128).abs();
                    if (p2 - p1) << T::TOL_BITS > p2 {
                        prop_assert_eq!(&got, &b_first, "τ order, τ_b < τ_a: {:?} {:?}", ca, cb);
                    } else {
                        prop_assert!(got == a_first || got == b_first, "τ order: {:?}", got);
                    }
                }
            }
            // The cuts scaled apart: τ is unchanged, so the order is.
            let scaled = detect(at::<T>(ca.0, ka, cb.0, kb), at::<T>(ca.1, ka, cb.1, kb));
            prop_assert_eq!(
                scaled,
                got,
                "τ order changes under scale 2^{} and 2^{}: {:?} {:?}",
                ka,
                kb,
                ca,
                cb
            );
            Ok(())
        },
    );
}

#[test]
fn qa_m316_tau_against_the_formula() {
    check_tau::<f32>(detect::<f32>);
    check_tau::<f64>(detect::<f64>);
}

/// The fixed cut-ID priority alone.
#[cfg(feature = "controls")]
fn by_cut_id<T: IeeeBits>(start: CutDistances<T>, end: CutDistances<T>) -> Vec<u32> {
    let mut out = detect(start, end);
    out.sort_by_key(|&s| s >> 1);
    out
}

negative_control!(
    qa_m316_tau_against_the_formula,
    "the cut-ID priority alone is not the τ order",
    expected = "τ order",
    check_tau::<f64>(by_cut_id::<f64>)
);

// ── qa_m316_tau_is_chronological ─────────────────────────────────────────────────────────────────────────────────────

fn check_chronological(detect: Detect<f64>) {
    const N: i64 = 64;
    prop::run(&(crossed_cut(), crossed_cut()), |(ca, cb)| {
        prop_assume!(tau_order(ca, cb) != Ordering::Equal);
        let point = |k: i64| at::<f64>((N - k) * ca.0 + k * ca.1, 0, (N - k) * cb.0 + k * cb.1, 0);
        let whole = detect(point(0), point(N));
        prop_assert_eq!(whole.len(), 2, "both cuts are crossed: {:?} {:?}", ca, cb);
        let mut subdivided = Vec::new();
        for k in 0..N {
            subdivided.extend(detect(point(k), point(k + 1)));
        }
        prop_assert_eq!(
            &whole,
            &subdivided,
            "the order is not chronological: {:?} {:?}",
            ca,
            cb
        );
        // τ ↦ 1 − τ backwards: the inverse symbols, in the reverse order.
        let back: Vec<u32> = whole.iter().rev().map(|s| s ^ 1).collect();
        prop_assert_eq!(
            detect(point(N), point(0)),
            back,
            "the order is not chronological backwards: {:?} {:?}",
            ca,
            cb
        );
        Ok(())
    });
}

#[test]
fn qa_m316_tau_is_chronological() {
    check_chronological(detect::<f64>);
}

/// The order by `τ` descending: the products' comparison with its operands exchanged.
#[cfg(feature = "controls")]
fn descending(start: CutDistances<f64>, end: CutDistances<f64>) -> Vec<u32> {
    let mut out = detect(start, end);
    if out.len() == 2 {
        out.reverse();
    }
    out
}

negative_control!(
    qa_m316_tau_is_chronological,
    "τ descending is not the crossings' order in time",
    expected = "the order is not chronological",
    check_chronological(descending)
);

// ── qa_m316_tie_tolerance_is_the_working_precision ───────────────────────────────────────────────────────────────────

fn check_tolerance(d32: Detect<f32>, d64: Detect<f64>) {
    // |d₀,b|·|d₁,a| = 1 + 2⁻²² against |d₀,a|·|d₁,b| = (1 + 2⁻²³)² = 1 + 2⁻²² + 2⁻⁴⁶, so τ_b < τ_a by about 2⁻⁴⁶: the
    // two products round to one value at f32, a tie, a first; they stay apart at f64, b first.
    let d0a = -(1.0 + two_to(-23));
    let d1a = 1.0;
    let d0b = -(1.0 + two_to(-22));
    let d1b = 1.0 + two_to(-23);
    let s32 = CutDistances::ungated(d0a as f32, d0b as f32);
    let e32 = CutDistances::ungated(d1a as f32, d1b as f32);
    assert_eq!(
        d32(s32, e32),
        vec![A, B],
        "the products round together at f32: the tie-break, a first"
    );
    assert_eq!(
        d64(
            CutDistances::ungated(d0a, d0b),
            CutDistances::ungated(d1a, d1b)
        ),
        vec![B, A],
        "the products stay apart at f64: b first"
    );
    // One ulp of a product apart at f32 is not a tie: 1 against 1 + 2⁻²³.
    let up = 1.0 + two_to(-23);
    assert_eq!(
        d32(
            CutDistances::ungated(-up as f32, -1.0),
            CutDistances::ungated(1.0, 1.0)
        ),
        vec![B, A],
        "a product an ulp past the other at f32 is not a tie"
    );
}

#[test]
fn qa_m316_tie_tolerance_is_the_working_precision() {
    check_tolerance(detect::<f32>, detect::<f64>);
}

/// A tolerance read at f64 from f32's distances: no rounding merges the products, so no tie at f32.
#[cfg(feature = "controls")]
fn wide_products(start: CutDistances<f32>, end: CutDistances<f32>) -> Vec<u32> {
    let wide = |c: CutDistances<f32>| CutDistances::ungated(f64::from(c.a), f64::from(c.b));
    detect(wide(start), wide(end))
}

negative_control!(
    qa_m316_tie_tolerance_is_the_working_precision,
    "products formed past the working precision do not tie",
    expected = "the products round together at f32",
    check_tolerance(wide_products, detect::<f64>)
);

// ── qa_m316_half_open_literal ────────────────────────────────────────────────────────────────────────────────────────

/// A normal float of either sign or a zero of either sign: the domain on which "`d < 0`" is the literal comparison.
fn distance() -> impl Strategy<Value = f64> {
    prop_oneof![
        1 => Just(0.0f64),
        1 => Just(-0.0f64),
        4 => (-1e30f64..1e30).prop_filter("normal at f32", |x| x.abs() > 1e-30),
        4 => (1u32..=64, any::<bool>()).prop_map(|(x, n)| if n { -f64::from(x) } else { f64::from(x) }),
    ]
}

fn check_half_open<T: Q>(detect: Detect<T>) {
    let march = proptest::collection::vec((distance(), distance()), 2..32);
    prop::run(&march, |march| {
        let ds: Vec<(T, T)> = march.iter().map(|d| (T::of(d.0), T::of(d.1))).collect();
        let zero = T::default();
        let mut net = [0i32; 2];
        for w in ds.windows(2) {
            let got = detect(
                CutDistances::ungated(w[0].0, w[0].1),
                CutDistances::ungated(w[1].0, w[1].1),
            );
            let mut want = Vec::new();
            for (d0, d1, generator, slot) in [(w[0].0, w[1].0, A, 0), (w[0].1, w[1].1, B, 1)] {
                if d0 < zero && d1 >= zero {
                    want.push(generator);
                    net[slot] += 1;
                } else if d0 >= zero && d1 < zero {
                    want.push(generator + 1);
                    net[slot] -= 1;
                }
            }
            let mut sorted = got.clone();
            sorted.sort_unstable();
            prop_assert_eq!(sorted, want, "the half-open rule at {:?}", w);
        }
        // Telescoping: no duplicate and no miss, from the march's two ends alone.
        let side = |d: T| i32::from(d < zero);
        let (first, last) = (ds[0], ds[ds.len() - 1]);
        prop_assert_eq!(
            net[0],
            side(first.0) - side(last.0),
            "the half-open rule: a's net"
        );
        prop_assert_eq!(
            net[1],
            side(first.1) - side(last.1),
            "the half-open rule: b's net"
        );
        Ok(())
    });
}

#[test]
fn qa_m316_half_open_literal() {
    check_half_open::<f32>(detect::<f32>);
    check_half_open::<f64>(detect::<f64>);
}

/// The other half-open convention: `d ≤ 0` is the negative side, so a touch of the cut from above counts.
#[cfg(feature = "controls")]
fn closed_below(start: CutDistances<f64>, end: CutDistances<f64>) -> Vec<u32> {
    let mut out = Vec::new();
    for (d0, d1, g) in [(start.a, end.a, A), (start.b, end.b, B)] {
        if d0 <= 0.0 && d1 > 0.0 {
            out.push(g);
        } else if d0 > 0.0 && d1 <= 0.0 {
            out.push(g + 1);
        }
    }
    out
}

negative_control!(
    qa_m316_half_open_literal,
    "d ≤ 0 as the negative side is the other half-open rule",
    expected = "the half-open rule",
    check_half_open::<f64>(closed_below)
);

// ── qa_m316_double_crossing_reaches_the_word ─────────────────────────────────────────────────────────────────────────

/// One sample's word, counting its accesses.
struct Counting {
    word: FreeGroupWord,
    loads: Cell<u32>,
    stores: u32,
}

impl WordBuffer for Counting {
    fn load(&self, i: usize) -> [u32; 4] {
        assert_eq!(i, 0, "the hook's index");
        self.loads.set(self.loads.get() + 1);
        self.word.free_group_word
    }
    fn store(&mut self, i: usize, w: [u32; 4]) {
        assert_eq!(i, 0, "the hook's index");
        self.stores += 1;
        self.word.free_group_word = w;
    }
}

type Feed =
    fn(&mut CrossingDetector<f64>, CutDistances<f64>, &mut CrossingHook, &mut Counting, u32) -> u32;

fn check_word(feed: Feed) {
    let at = |a: f64, b: f64| CutDistances::ungated(a, b);
    let mut detector = CrossingDetector::new(at(-3.0, -1.0));
    let mut hook = CrossingHook::new(0);
    let mut words = Counting {
        word: FreeGroupWord {
            free_group_word: [0; 4],
        },
        loads: Cell::new(0),
        stores: 0,
    };
    // τ_b = 1/4 before τ_a = 3/4.
    let packed = feed(&mut detector, at(1.0, 1.0), &mut hook, &mut words, 0);
    let got: Vec<u32> = fgw_decode(words.word.free_group_word).collect();
    assert_eq!(got, vec![B, A], "the word is not in τ order");
    assert_eq!(
        (words.loads.get(), words.stores),
        (2, 2),
        "two crossings, two appends"
    );
    assert_eq!(hook.prev, A, "prev is the word's last symbol");
    // A quiet substep neither reads nor writes.
    let packed = feed(&mut detector, at(2.0, 2.0), &mut hook, &mut words, packed);
    assert_eq!(
        (words.loads.get(), words.stores),
        (2, 2),
        "a quiet substep touched the word"
    );
    // Back over both, τ_a = 2/5 before τ_b = 2/3: A cancels the a on top, then B the b, and the word is empty.
    feed(&mut detector, at(-3.0, -1.0), &mut hook, &mut words, packed);
    assert_eq!(
        fgw_length_raw(words.word.free_group_word),
        0,
        "the word is not in τ order: the recrossing does not cancel it"
    );
    assert_eq!(hook.prev, FGW_NO_SYMBOL, "the empty word's prev");
    assert_eq!(
        (words.loads.get(), words.stores),
        (4, 4),
        "two crossings, two appends"
    );
}

#[test]
fn qa_m316_double_crossing_reaches_the_word() {
    check_word(|d, e, h, w, p| d.accept_into(e, h, w, p));
}

/// The substep's two crossings appended in the opposite order.
#[cfg(feature = "controls")]
fn reversed_feed(
    d: &mut CrossingDetector<f64>,
    e: CutDistances<f64>,
    h: &mut CrossingHook,
    w: &mut Counting,
    p: u32,
) -> u32 {
    let c: SubstepCrossings = d.accept(e);
    let order: Vec<u32> = c.symbols[..c.count as usize]
        .iter()
        .rev()
        .copied()
        .collect();
    let mut p = p;
    for s in order {
        p = h.crossing(w, p, s);
    }
    p
}

negative_control!(
    qa_m316_double_crossing_reaches_the_word,
    "the crossings appended against τ make another word",
    expected = "the word is not in τ order",
    check_word(reversed_feed)
);

// ── Positions: §1's cuts ─────────────────────────────────────────────────────────────────────────────────────────────

type Cuts<T> = fn(&[[T; 2]; 3]) -> CutDistances<T>;

/// The named body at `(0, y)`, the two others at `(−1, 0)` and `(1, 0)`, the lower index on the left; `d` is worked by
/// hand for each in `check_syzygies`.
fn bridge(between: usize, y: f64) -> [[f64; 2]; 3] {
    match between {
        0 => [[0.0, y], [-1.0, 0.0], [1.0, 0.0]],
        1 => [[-1.0, 0.0], [0.0, y], [1.0, 0.0]],
        _ => [[-1.0, 0.0], [1.0, 0.0], [0.0, y]],
    }
}

fn march<T: Q>(cuts: Cuts<T>, states: &[[[f64; 2]; 3]]) -> Vec<Vec<u32>> {
    let of = |r: &[[f64; 2]; 3]| cuts(&r.map(|p| p.map(T::of)));
    let mut detector = CrossingDetector::new(of(&states[0]));
    states[1..]
        .iter()
        .map(|r| {
            let c: SubstepCrossings = detector.accept(of(r));
            c.symbols[..c.count as usize].to_vec()
        })
        .collect()
}

// ── qa_m316_syzygy_known_answers ─────────────────────────────────────────────────────────────────────────────────────

fn check_syzygies<T: Q>(cuts: Cuts<T>) {
    // Worked from d = −((r₁ − r₀) × (r₂ − r₀)) by hand. bridge(0, y): u = (−1, −y), w = (1, −y), u × w = 2y, d = −2y.
    // bridge(2, y): u = (2, 0), w = (1, y), u × w = 2y, d = −2y. bridge(1, y): u = (1, y), w = (2, 0), u × w = −2y, d = 2y.
    // So y > 0 is d < 0 (L⁺'s hemisphere) for bodies 0 and 2. Body 0 between is cut a, body 2 cut b, body 1 neither.
    let e: &[u32] = &[];
    for (body, gen, inv) in [(0usize, A, INV_A), (2, B, INV_B)] {
        let path = |ys: &[f64]| -> Vec<_> { ys.iter().map(|&y| bridge(body, y)).collect() };
        let cases: [(&[f64], &[&[u32]]); 6] = [
            (&[1.0, -1.0], &[&[gen]]),
            (&[-1.0, 1.0], &[&[inv]]),
            // An endpoint exactly on the equator is on the non-negative side, counted once.
            (&[1.0, 0.0, -1.0], &[&[gen], e]),
            (&[-1.0, 0.0, 1.0], &[e, &[inv]]),
            // A touch from L⁺'s side crosses and recrosses; a touch from L⁻'s side does not cross.
            (&[1.0, 0.0, 1.0], &[&[gen], &[inv]]),
            (&[-1.0, 0.0, -1.0], &[e, e]),
        ];
        for (ys, want) in cases {
            let want: Vec<Vec<u32>> = want.iter().map(|s| s.to_vec()).collect();
            assert_eq!(
                march(cuts, &path(ys)),
                want,
                "the syzygy of body {body} between, y through {ys:?} at {}",
                T::NAME
            );
        }
    }
    // Body 1 between: the segment that is no cut (Appendix C's 22 = empty).
    for ys in [[1.0, -1.0], [-1.0, 1.0]] {
        let path = [bridge(1, ys[0]), bridge(1, ys[1])];
        assert_eq!(
            march(cuts, &path),
            vec![Vec::<u32>::new()],
            "the syzygy of body 1 between is not a cut"
        );
    }
    // The other outer order, bodies 1 and 2 exchanged: u × w changes sign, so the sense is the other way round.
    let swapped = |y: f64| [[0.0, y], [1.0, 0.0], [-1.0, 0.0]];
    assert_eq!(
        march(cuts, &[swapped(-1.0), swapped(1.0)]),
        vec![vec![A]],
        "the syzygy of body 0 between, orientation from the labels, not the plane"
    );
    // Across a substep the classifier differs: the gate is the non-negative endpoint's. d = −2 at bridge(0, 1), +2 at
    // bridge(2, −1), so positive sense, the end reads body 2; and backwards, negative sense, the start reads body 2.
    assert_eq!(
        march(cuts, &[bridge(0, 1.0), bridge(2, -1.0)]),
        vec![vec![B]],
        "the gate at the non-negative endpoint"
    );
    assert_eq!(
        march(cuts, &[bridge(2, -1.0), bridge(0, 1.0)]),
        vec![vec![INV_B]],
        "the gate at the non-negative endpoint"
    );
}

#[test]
fn qa_m316_syzygy_known_answers() {
    check_syzygies::<f32>(state_cuts::<f32>);
    check_syzygies::<f64>(state_cuts::<f64>);
}

/// Cut `a` where body 1 is between (`P₀₁`–`P₁₂`), not body 0.
#[cfg(feature = "controls")]
fn wrong_cut(r: &[[f64; 2]; 3]) -> CutDistances<f64> {
    let mid = [r[1], r[0], r[2]];
    let c = state_cuts(r);
    CutDistances {
        gate_a: state_cuts(&mid).gate_a,
        ..c
    }
}

negative_control!(
    qa_m316_syzygy_known_answers,
    "the middle body's segment as cut a",
    expected = "the syzygy of body",
    check_syzygies::<f64>(wrong_cut)
);

// ── qa_m316_cuts_invariances ─────────────────────────────────────────────────────────────────────────────────────────

/// The classifier's body from integer positions, and whether its greatest squared separation is unique.
fn longest(r: [[i64; 2]; 3]) -> (u32, bool) {
    let sq = |i: usize, j: usize| (r[j][0] - r[i][0]).pow(2) + (r[j][1] - r[i][1]).pow(2);
    let n = [sq(1, 2), sq(2, 0), sq(0, 1)];
    let top = *n.iter().max().expect("three");
    let k = n.iter().position(|&x| x == top).expect("the greatest");
    (k as u32, n.iter().filter(|&&x| x == top).count() == 1)
}

fn doubled_area(r: [[i64; 2]; 3]) -> i64 {
    let u = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let w = [r[2][0] - r[0][0], r[2][1] - r[0][1]];
    u[0] * w[1] - u[1] * w[0]
}

fn check_invariances<T: Q>(cuts: Cuts<T>) {
    let c = || -6i64..=6;
    let point = || (c(), c());
    prop::run(
        &((point(), point(), point()), (c(), c())),
        |((p0, p1, p2), (tx, ty))| {
            let r = [[p0.0, p0.1], [p1.0, p1.1], [p2.0, p2.1]];
            let real = |r: [[i64; 2]; 3], s: f64| r.map(|q| q.map(|v| T::of(v as f64 * s)));
            let base = cuts(&real(r, 1.0));
            let (k, unique) = longest(r);
            let gates = (k == 0, k == 2);

            let moved = r.map(|q| [q[0] + tx, q[1] + ty]);
            let turned = r.map(|q| [-q[1], q[0]]);
            for (name, image) in [("translation", moved), ("rotation", turned)] {
                let x = cuts(&real(image, 1.0));
                prop_assert_eq!(x.a, base.a, "d under {} of {:?}", name, r);
                prop_assert_eq!(
                    (x.gate_a, x.gate_b),
                    (base.gate_a, base.gate_b),
                    "the classifier under {} of {:?}",
                    name,
                    r
                );
            }

            // The whole exponent range: the classifier at every scale; d's side where d is not below the normal range.
            for s in [T::SCALE, 40, 1, -40, -T::SCALE] {
                let x = cuts(&real(r, two_to(s)));
                prop_assert_eq!(
                    (x.gate_a, x.gate_b),
                    gates,
                    "the classifier at scale 2^{} of {:?}",
                    s,
                    r
                );
                if s >= 0 {
                    let area = doubled_area(r);
                    let negative = kernel::word::crossing::negative(x.a);
                    prop_assert_eq!(negative, area > 0, "d's side at scale 2^{} of {:?}", s, r);
                }
            }

            // Bodies 0 and 2 exchanged: the triangle's orientation reverses, and the two named segments exchange.
            let swap = cuts(&real([r[2], r[1], r[0]], 1.0));
            prop_assert_eq!(
                swap.a,
                T::of(doubled_area(r) as f64),
                "d under bodies 0 and 2 exchanged, {:?}",
                r
            );
            prop_assert_eq!(
                base.a,
                T::of(-doubled_area(r) as f64),
                "d from positions, {:?}",
                r
            );
            if unique {
                prop_assert_eq!(
                    (swap.gate_a, swap.gate_b),
                    (base.gate_b, base.gate_a),
                    "the gates under bodies 0 and 2 exchanged, {:?}",
                    r
                );
            }
            Ok(())
        },
    );
}

#[test]
fn qa_m316_cuts_invariances() {
    check_invariances::<f32>(state_cuts::<f32>);
    check_invariances::<f64>(state_cuts::<f64>);
}

/// The classifier on the plain float squares, which underflow together at a small enough scale and tie.
#[cfg(feature = "controls")]
fn plain_gates(r: &[[f32; 2]; 3]) -> CutDistances<f32> {
    let sq = |i: usize, j: usize| {
        let (x, y) = (r[j][0] - r[i][0], r[j][1] - r[i][1]);
        x * x + y * y
    };
    let n = [sq(1, 2), sq(2, 0), sq(0, 1)];
    let k = (0..3).fold(0, |k, j| if n[j] > n[k] { j } else { k });
    CutDistances {
        gate_a: k == 0,
        gate_b: k == 2,
        ..state_cuts(r)
    }
}

negative_control!(
    qa_m316_cuts_invariances,
    "the plain squares tie below the normal range",
    expected = "the classifier at scale",
    check_invariances::<f32>(plain_gates)
);

// ── qa_m316_march_reversed_reads_the_inverse ─────────────────────────────────────────────────────────────────────────

/// A reader of a march from positions: the crossings it appends, in order.
type Read = fn(&[[[f64; 2]; 3]]) -> Vec<u32>;

fn read_at<T: Q>(states: &[[[f64; 2]; 3]]) -> Vec<u32> {
    march(state_cuts::<T>, states).concat()
}

fn check_reversal(read: Read) {
    // Small integer coordinates, so an endpoint exactly on the equator is common.
    let c = || (-3i32..=3).prop_map(f64::from);
    let state = || proptest::array::uniform3((c(), c()).prop_map(|(x, y)| [x, y]));
    prop::run(&proptest::collection::vec(state(), 2..24), |states| {
        let mut back = states.clone();
        back.reverse();
        let inverse: Vec<u32> = read(&states).iter().rev().map(|s| s ^ 1).collect();
        prop_assert_eq!(
            read(&back),
            inverse,
            "the march backwards does not read the inverse: {:?}",
            states
        );
        Ok(())
    });
}

#[test]
fn qa_m316_march_reversed_reads_the_inverse() {
    check_reversal(read_at::<f32>);
    check_reversal(read_at::<f64>);
}

/// The gate read at the start of every substep, not at its non-negative endpoint.
#[cfg(feature = "controls")]
fn gate_at_start(states: &[[[f64; 2]; 3]]) -> Vec<u32> {
    states
        .windows(2)
        .flat_map(|w| {
            let (s, e) = (state_cuts(&w[0]), state_cuts(&w[1]));
            detect(
                s,
                CutDistances {
                    gate_a: s.gate_a,
                    gate_b: s.gate_b,
                    ..e
                },
            )
        })
        .collect()
}

negative_control!(
    qa_m316_march_reversed_reads_the_inverse,
    "a gate read at each substep's start depends on the march's direction",
    expected = "the march backwards does not read the inverse",
    check_reversal(gate_at_start)
);

// ── qa_m316_prl_fixture_against_appendix_c ───────────────────────────────────────────────────────────────────────────

fn codes(w: &str) -> Vec<u32> {
    w.chars()
        .map(|c| match c {
            'a' => A,
            'A' => INV_A,
            'b' => B,
            'B' => INV_B,
            _ => panic!("{c} is no letter"),
        })
        .collect()
}

fn cyclic(w: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for &s in w {
        if out.last() == Some(&(s ^ 1)) {
            out.pop();
        } else {
            out.push(s);
        }
    }
    let mut lo = 0;
    let mut hi = out.len();
    while hi - lo > 1 && out[lo] == out[hi - 1] ^ 1 {
        lo += 1;
        hi -= 1;
    }
    out[lo..hi].to_vec()
}

fn is_rotation(w: &[u32], of: &[u32]) -> bool {
    w.len() == of.len() && (0..of.len().max(1)).any(|i| of[i..].iter().chain(&of[..i]).eq(w.iter()))
}

/// Table I's printed form written out: `(u)n` is `u` `n` times, a bare `(u)` once, letters themselves.
fn expand(printed: &str) -> String {
    let mut out = String::new();
    let mut rest = printed;
    while let Some(c) = rest.chars().next() {
        if c == '(' {
            let close = rest.find(')').expect("a closing bracket");
            let body = &rest[1..close];
            rest = &rest[close + 1..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            rest = &rest[digits.len()..];
            out.push_str(&body.repeat(digits.parse().unwrap_or(1)));
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

/// Appendix C's reading, done by plain arithmetic: at each sign change of the orientation the digit of the body between
/// (ours 0, 1, 2 its 1, 2, 3), the segment read at the non-negative endpoint; digits paired from a crossing out of `L⁺`'s
/// side; `a = 12, A = 21, b = 32, B = 23`, and `22 = empty` so that `pq = (p2)(2q)`.
fn appendix_c(states: &[[[f64; 2]; 3]]) -> Vec<u32> {
    let area = |r: &[[f64; 2]; 3]| {
        (r[1][0] - r[0][0]) * (r[2][1] - r[0][1]) - (r[1][1] - r[0][1]) * (r[2][0] - r[0][0])
    };
    let digit = |r: &[[f64; 2]; 3]| {
        let sq = |i: usize, j: usize| (r[j][0] - r[i][0]).powi(2) + (r[j][1] - r[i][1]).powi(2);
        let n = [sq(1, 2), sq(2, 0), sq(0, 1)];
        (0..3).fold(0, |k, j| if n[j] > n[k] { j } else { k }) + 1
    };
    let mut crossings: Vec<(bool, usize)> = Vec::new();
    for w in states.windows(2) {
        let (s0, s1) = (area(&w[0]), area(&w[1]));
        if (s0 > 0.0) != (s1 > 0.0) {
            let out_of_l_plus = s0 > 0.0;
            crossings.push((
                out_of_l_plus,
                digit(if out_of_l_plus { &w[1] } else { &w[0] }),
            ));
        }
    }
    if !crossings[0].0 {
        crossings.rotate_left(1);
    }
    assert_eq!(crossings.len() % 2, 0, "the orbit's syzygies pair up");
    let mut word = Vec::new();
    for pair in crossings.chunks(2) {
        assert!(
            pair[0].0 && !pair[1].0,
            "the syzygies alternate hemispheres"
        );
        match pair[0].1 {
            1 => word.push(A),
            3 => word.push(B),
            _ => {}
        }
        match pair[1].1 {
            1 => word.push(INV_A),
            3 => word.push(INV_B),
            _ => {}
        }
    }
    cyclic(&word)
}

fn read(cuts: Cuts<f64>, states: &[[[f64; 2]; 3]]) -> Vec<u32> {
    cyclic(&march(cuts, states).concat())
}

fn check_fixture(cuts: Cuts<f64>) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/crossings/prl_orbits.json"
    );
    let text = std::fs::read_to_string(path).expect("read the fixture");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("the fixture is JSON");
    let orbits = doc["orbits"].as_array().expect("orbits");
    let mut tally = [0usize; 2];
    for orbit in orbits {
        let name = orbit["name"].as_str().expect("name");
        let published = orbit["published"].as_str().expect("published");
        assert_eq!(
            expand(orbit["table_i"]["word"].as_str().expect("printed")),
            published,
            "{name}: the published word is not Table I's printed form"
        );
        match orbit["symmetry"].as_str().expect("symmetry") {
            "conjugacy" => tally[0] += 1,
            "inverse" => tally[1] += 1,
            other => panic!("{name}: §1 names no orbit read up to {other}"),
        }
        let states: Vec<[[f64; 2]; 3]> = orbit["states"]
            .as_array()
            .expect("states")
            .iter()
            .map(|s| {
                let body = |k: usize| [s[k][0].as_f64().expect("x"), s[k][1].as_f64().expect("y")];
                [body(0), body(1), body(2)]
            })
            .collect();
        let got = read(cuts, &states);
        assert!(
            is_rotation(&got, &appendix_c(&states)),
            "{name}: the word read is not Appendix C's reading"
        );
        // §1: "the other outer-body order reads each of the ten orbits below within those symmetries too".
        let other: Vec<_> = states.iter().map(|r| [r[2], r[1], r[0]]).collect();
        let got = read(cuts, &other);
        let want = cyclic(&codes(published));
        let inverse: Vec<u32> = got.iter().rev().map(|s| s ^ 1).collect();
        let exchange: Vec<u32> = got.iter().map(|s| s ^ 1).collect();
        let both: Vec<u32> = inverse.iter().map(|s| s ^ 1).collect();
        assert!(
            [&got, &inverse, &exchange, &both]
                .iter()
                .any(|w| is_rotation(w, &want)),
            "{name}: the other outer-body order is not within the symmetries"
        );
    }
    assert_eq!(
        tally,
        [8, 2],
        "§1's tally: eight up to cyclic permutation, two up to the inverse"
    );
}

#[test]
fn qa_m316_prl_fixture_against_appendix_c() {
    check_fixture(state_cuts::<f64>);
}

/// Cut `a` read on body 1's segment.
#[cfg(feature = "controls")]
fn body_one_is_a(r: &[[f64; 2]; 3]) -> CutDistances<f64> {
    wrong_cut(r)
}

negative_control!(
    qa_m316_prl_fixture_against_appendix_c,
    "another segment as cut a is not Appendix C's reading",
    expected = "the word read is not Appendix C's reading",
    check_fixture(body_one_is_a)
);
