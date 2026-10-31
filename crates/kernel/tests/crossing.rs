//! Branch-cut crossing detection (`kernel::word::crossing`; integrator contract Part 5, the τ tie rule; dd_simstate_payload
//! §3), on the synthetic fixtures of `fixtures/crossings/segments.json`, at f32 and f64: each case gives two cuts'
//! signed distances directly, with no segment gate (both open), so the sort is exercised on cuts with distinct
//! distances. The cuts of symbolic_dynamics_contract §1, which share one distance and choose by segment gate, are
//! checked from positions by `crossing_segment_gate`, `crossing_state_cuts` and `crossing_published_words`.
//! - REQ-INT-031: a double crossing in one substep orders by `τ`; equal `τ` uses the cut ID; an endpoint exactly on the
//!   cut follows the half-open rule (`crossing_order`).
//! - REQ-INT-046: a cross-and-recross inside one macro-step yields both crossings, and the word they leave
//!   (`crossing_every_substep`).
//! - REQ-INT-047: a segment crossing both cuts near their shared endpoint appends in `τ` order, distances below f32's
//!   product range included; the product comparison against the exact one (`crossing_tau_sort`).
//! - REQ-INT-048: endpoints exactly on a cut, `−0` and subnormals among them, give no duplicate or missed crossing
//!   (`crossing_half_open`). The GPU arm is REQ-VAL-072's at M4 (R-150).
//! - The segment gate (integrator contract Part 5): read at the substep's non-negative endpoint, exclusive gates
//!   crossing one cut at most (`crossing_segment_gate`).
//! - §1's cuts from positions: `d`'s sign against the shape sphere's `n_w`, the classifier against the exact longest
//!   side, and loops about each collision point reading `A`, `b` and Appendix B's `aB` (`crossing_state_cuts`).
//! - REQ-PAY-070's cross-check: ten orbits of the PRL's Table I read their published words up to R-630 item 5's
//!   symmetries, from `fixtures/crossings/prl_orbits.json` (`crossing_published_words`). REQ-VAL-043 is TASK-M3-30's.
//! - A march that starts on the cut, or a rounding from it, each way (`crossing_start_on_cut`), and what a coarse
//!   substep reads wrongly (`crossing_substep_limits`), from `fixtures/crossings/position_marches.json` (integrator
//!   contract Part 5).
//! - No subnormal `d` feeds the side (R-547): a distance below the normal range reads zero though its float would
//!   round up to the least normal (`crossing_no_subnormal_distance`).
//!
//! Each check takes the detector as a function, so its control runs the same check on a faulty one.

use kernel::payload::{fgw_length_raw, sd_last_symbol, FreeGroupWord, FGW_NO_SYMBOL};
use kernel::real::IeeeBits;
use kernel::word::crossing::{negative, product_less, reads_as_zero, signed_distance};
use kernel::word::{
    fgw_decode, set_last_symbol, state_cuts, substep_crossings, CrossingDetector, CrossingHook,
    CutDistances,
};
use proptest::prelude::*;
use validation::reference::free_group::reduce;
use validation::{negative_control, prop};

/// A float the fixtures are read at: f32 or f64, each fixture value exact at both.
trait Fixture: IeeeBits {
    fn of(v: f64) -> Self;
    /// `v` rounded to the precision, not required exact.
    fn of_rounded(v: f64) -> Self;
    /// The negative subnormal of least magnitude, and the negative normal of least magnitude.
    fn negative_subnormal() -> Self;
    fn negative_least_normal() -> Self;
}

impl Fixture for f32 {
    fn of(v: f64) -> f32 {
        let x = v as f32;
        assert_eq!(f64::from(x), v, "the fixture value {v} is not exact at f32");
        x
    }
    fn of_rounded(v: f64) -> f32 {
        v as f32
    }
    fn negative_subnormal() -> f32 {
        f32::from_bits(0x8000_0001)
    }
    fn negative_least_normal() -> f32 {
        -f32::MIN_POSITIVE
    }
}

impl Fixture for f64 {
    fn of(v: f64) -> f64 {
        v
    }
    fn of_rounded(v: f64) -> f64 {
        v
    }
    fn negative_subnormal() -> f64 {
        f64::from_bits(0x8000_0000_0000_0001)
    }
    fn negative_least_normal() -> f64 {
        -f64::MIN_POSITIVE
    }
}

/// A detector: one accepted substep's crossings, in order, from the cuts' distances at its start and its end, as
/// symbol codes. [`substep_crossings`], or a control's faulty one.
type Detect<T> = fn(CutDistances<T>, CutDistances<T>) -> Vec<u32>;

fn detect<T: IeeeBits>(start: CutDistances<T>, end: CutDistances<T>) -> Vec<u32> {
    let c = substep_crossings(start, end);
    c.symbols[..c.count as usize].to_vec()
}

fn symbol(s: &str) -> u32 {
    match s {
        "a" => 0,
        "A" => 1,
        "b" => 2,
        "B" => 3,
        _ => panic!("{s} is not a symbol"),
    }
}

/// One fixture case: the cuts' distances at a march's accepted endpoints, and each substep's crossings.
struct Case {
    name: String,
    fixture: String,
    endpoints: Vec<(f64, f64)>,
    crossings: Vec<Vec<u32>>,
}

fn cases() -> Vec<Case> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/crossings/segments.json"
    );
    let text = std::fs::read_to_string(path).expect("read the crossings fixture");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("the fixture is JSON");
    doc["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|c| Case {
            name: c["name"].as_str().expect("name").into(),
            fixture: c["fixture"].as_str().expect("fixture").into(),
            endpoints: c["endpoints"]
                .as_array()
                .expect("endpoints")
                .iter()
                .map(|e| (e[0].as_f64().expect("a"), e[1].as_f64().expect("b")))
                .collect(),
            crossings: c["crossings"]
                .as_array()
                .expect("crossings")
                .iter()
                .map(|s| {
                    s.as_array()
                        .expect("a substep")
                        .iter()
                        .map(|x| symbol(x.as_str().expect("a symbol")))
                        .collect()
                })
                .collect(),
        })
        .collect()
}

/// The fixture's cases of `fixture`; at least one.
fn fixture(fixture: &str) -> Vec<Case> {
    let cases: Vec<Case> = cases()
        .into_iter()
        .filter(|c| c.fixture == fixture)
        .collect();
    assert!(!cases.is_empty(), "no fixture case is {fixture:?}");
    cases
}

fn at<T: Fixture>(e: (f64, f64)) -> CutDistances<T> {
    CutDistances::ungated(T::of(e.0), T::of(e.1))
}

/// Each case's substeps through `detect`, at `T`: each substep's crossings are the fixture's.
fn check_cases<T: Fixture>(detect: Detect<T>, cases: &[Case]) {
    for case in cases {
        for (i, pair) in case.endpoints.windows(2).enumerate() {
            let got = detect(at::<T>(pair[0]), at::<T>(pair[1]));
            assert_eq!(
                got,
                case.crossings[i],
                "{} at {}, substep {i}: crossings out of order or miscounted",
                case.name,
                T::NAME
            );
        }
    }
}

// ── crossing_order (REQ-INT-031) ─────────────────────────────────────────────────────────────────────────────────────

fn check_order(d32: Detect<f32>, d64: Detect<f64>) {
    let mut cases = fixture("double crossing in one substep");
    cases.extend(fixture("endpoint exactly on a cut"));
    check_cases(d32, &cases);
    check_cases(d64, &cases);
}

#[test]
fn crossing_order() {
    check_order(detect::<f32>, detect::<f64>);
}

/// A detector with the fixed cut-ID priority alone: `a`'s crossing first, whatever the `τ`.
#[cfg(feature = "controls")]
fn by_cut_id<T: IeeeBits>(start: CutDistances<T>, end: CutDistances<T>) -> Vec<u32> {
    let mut out = detect(start, end);
    out.sort_by_key(|&s| s >> 1);
    out
}

negative_control!(
    crossing_order,
    "the fixed cut-ID priority alone puts a's crossing first when b's τ is less",
    expected = "crossings out of order or miscounted",
    check_order(by_cut_id::<f32>, by_cut_id::<f64>)
);

// ── crossing_every_substep (REQ-INT-046) ─────────────────────────────────────────────────────────────────────────────

/// A march: its crossings, in order, from the cuts' distances at its accepted endpoints. The detector at every
/// accepted substep, or a control's faulty one.
type March = fn(&[CutDistances<f64>]) -> Vec<Vec<u32>>;

/// The sample's `packed_a` before its march: every bit set but `last_symbol`'s, so a write outside it shows.
const PACKED_A: u32 = 0xFFFF_FCFF;

/// [`CrossingDetector`], fed each accepted substep, appending into a one-sample word buffer through its hook: each
/// substep's crossings, and the word they leave, which must be the stream freely reduced.
fn every_substep(endpoints: &[CutDistances<f64>]) -> Vec<Vec<u32>> {
    let mut detector = CrossingDetector::new(endpoints[0]);
    let mut words = [FreeGroupWord {
        free_group_word: [0; 4],
    }];
    let mut hook = CrossingHook::new(0);
    let mut out = Vec::new();
    let mut packed_a = PACKED_A;
    for &end in &endpoints[1..] {
        let mut probe = detector;
        let c = probe.accept(end);
        out.push(c.symbols[..c.count as usize].to_vec());
        packed_a = detector.accept_into(end, &mut hook, &mut words[..], packed_a);
        assert_eq!(
            detector, probe,
            "accept_into moved the detector differently"
        );
    }
    let stream: Vec<u32> = out.concat();
    let reduced = reduce(&stream);
    let word = words[0].free_group_word;
    assert_eq!(fgw_length_raw(word), reduced.length(), "the word's length");
    assert_eq!(
        fgw_decode(word).collect::<Vec<_>>(),
        reduced.symbols,
        "the word is not the crossings freely reduced"
    );
    assert_eq!(
        hook.prev,
        reduced.symbols.last().copied().unwrap_or(FGW_NO_SYMBOL),
        "the hook's prev"
    );
    assert_eq!(
        set_last_symbol(packed_a, 0),
        set_last_symbol(PACKED_A, 0),
        "accept_into changed packed_a outside last_symbol"
    );
    if !reduced.symbols.is_empty() {
        assert_eq!(
            sd_last_symbol(packed_a),
            hook.prev,
            "last_symbol is not the word's last symbol"
        );
    }
    out
}

fn check_every_substep(march: March) {
    for case in fixture("cross-and-recross inside one macro-step") {
        let endpoints: Vec<_> = case.endpoints.iter().map(|&e| at::<f64>(e)).collect();
        let got: Vec<u32> = march(&endpoints).concat();
        assert_eq!(
            got,
            case.crossings.concat(),
            "{}: a crossing inside the macro-step was missed",
            case.name
        );
    }
}

#[test]
fn crossing_every_substep() {
    check_every_substep(every_substep);
    // The first case's word: a and its recrossing cancel, though both are detected.
    let case = &fixture("cross-and-recross inside one macro-step")[0];
    let endpoints: Vec<_> = case.endpoints.iter().map(|&e| at::<f64>(e)).collect();
    assert_eq!(every_substep(&endpoints).concat(), vec![0, 1]);
}

/// Detection once per macro-step: its two endpoints only.
#[cfg(feature = "controls")]
fn per_macro_step(endpoints: &[CutDistances<f64>]) -> Vec<Vec<u32>> {
    vec![detect(endpoints[0], endpoints[endpoints.len() - 1])]
}

negative_control!(
    crossing_every_substep,
    "detection once per macro-step misses a cross-and-recross inside it",
    expected = "a crossing inside the macro-step was missed",
    check_every_substep(per_macro_step)
);

// ── crossing_tau_sort (REQ-INT-047) ──────────────────────────────────────────────────────────────────────────────────

/// The exact order of `|x₁|·|y₁|` and `|x₂|·|y₂|`, f32 operands, each product exact in f64 (48 significant bits).
fn exact_order(x1: f32, y1: f32, x2: f32, y2: f32) -> std::cmp::Ordering {
    let p1 = f64::from(x1.abs()) * f64::from(y1.abs());
    let p2 = f64::from(x2.abs()) * f64::from(y2.abs());
    p1.partial_cmp(&p2).expect("finite")
}

/// A normal or zero f32 of either sign, over the whole exponent range, subnormals read as zero.
fn operand() -> impl Strategy<Value = f32> {
    prop_oneof![
        1 => Just(0.0f32),
        1 => Just(-0.0f32),
        1 => (1u32..0x0080_0000).prop_map(f32::from_bits),
        4 => (0x0080_0000u32..0x7F80_0000, any::<bool>())
            .prop_map(|(bits, neg)| f32::from_bits(bits | if neg { 0x8000_0000 } else { 0 })),
        // Exponents −1 to 2 only, so the products' exponents often meet and the significands' product decides.
        4 => (0x3F00_0000u32..0x4100_0000, any::<bool>())
            .prop_map(|(bits, neg)| f32::from_bits(bits | if neg { 0x8000_0000 } else { 0 })),
    ]
}

/// The products' comparison: `less` against the exact order at f32, where a subnormal reads as zero. Exactly less is
/// less or (by the one rounding) a tie; a tie or greater is never less; and at f64, on normal products that neither
/// overflow nor underflow, it is the plain product comparison.
fn check_product_less(
    less32: fn(f32, f32, f32, f32) -> bool,
    less64: fn(f64, f64, f64, f64) -> bool,
) {
    let flush = |x: f32| if x.is_subnormal() { 0.0 } else { x };
    prop::run(
        &(operand(), operand(), operand(), operand()),
        |(x1, y1, x2, y2)| {
            let exact = exact_order(flush(x1), flush(y1), flush(x2), flush(y2));
            let less = less32(x1, y1, x2, y2);
            let more = less32(x2, y2, x1, y1);
            prop_assert!(
                !(less && more),
                "both orders hold: {x1:e}·{y1:e} against {x2:e}·{y2:e}"
            );
            match exact {
                std::cmp::Ordering::Less => {
                    prop_assert!(!more, "{x1:e}·{y1:e} < {x2:e}·{y2:e} read as greater")
                }
                std::cmp::Ordering::Equal => {
                    prop_assert!(!less && !more, "{x1:e}·{y1:e} = {x2:e}·{y2:e} not a tie")
                }
                std::cmp::Ordering::Greater => {
                    prop_assert!(!less, "{x1:e}·{y1:e} > {x2:e}·{y2:e} read as less")
                }
            }
            let w = |x: f32| f64::from(x) * 2f64.powi(100);
            let (u1, v1, u2, v2) = (w(flush(x1)), w(flush(y1)), w(flush(x2)), w(flush(y2)));
            let plain = (u1 * v1).abs() < (u2 * v2).abs();
            prop_assert_eq!(
                less64(u1, v1, u2, v2),
                plain,
                "f64 against the plain product"
            );
            Ok(())
        },
    );
    // The boundaries: equal exponents, significands one ulp apart, and one exponent apart with the significands reversed.
    let one_up = f32::from_bits(1.5f32.to_bits() + 1);
    assert!(less32(1.5, 1.0, one_up, 1.0), "one ulp apart");
    assert!(!less32(1.5, 1.0, 1.5, 1.0), "equal is not less");
    assert!(less32(1.75, 1.0, 1.0, 2.0), "one exponent apart");
    assert!(
        less32(1.0, 1.75, 1.5, 1.5),
        "1.75 < 2.25: a significands' product past 2 carries into the exponent"
    );
    assert!(!less32(1.5, 1.5, 1.0, 1.75), "2.25 > 1.75");
    assert!(
        less64(1.0, 1.75, 1.5, 1.5) && !less64(1.5, 1.5, 1.0, 1.75),
        "2.25 against 1.75 at f64"
    );
    assert!(!less32(1.0, 2.0, 1.75, 1.0), "one exponent apart, reversed");
    assert!(
        less32(0.0, 1.0, 1.0, 1.0),
        "zero is less than a nonzero product"
    );
    assert!(
        !less32(1.0, 1.0, 0.0, 1.0),
        "a nonzero product is not less than zero"
    );
    assert!(!less32(0.0, 1.0, 1.0, 0.0), "two zeros tie");
    // A zero factor beside the largest finite one is zero, whichever factor it is: never zero's exponent field plus
    // the largest exponent, which would read as about 1, above 0.5.
    let big = f32::MAX;
    assert!(
        less32(0.0, big, 0.5, 1.0) && less32(big, 0.0, 0.5, 1.0),
        "a zero factor beside a large one"
    );
    assert!(
        !less32(0.5, 1.0, 0.0, big) && !less32(0.5, 1.0, big, 0.0),
        "a zero factor beside a large one"
    );
    assert!(
        less64(0.0, f64::MAX, 0.5, 1.0) && less64(f64::MAX, 0.0, 0.5, 1.0),
        "a zero factor beside a large one at f64"
    );
}

fn check_tau_sort(d32: Detect<f32>, d64: Detect<f64>) {
    let cases = fixture("crossing both cuts near their shared endpoint");
    check_cases(d32, &cases);
    check_cases(d64, &cases);
    // A near-tie one ulp apart: `a` from −1 to `x`, `b` from −1 to the float above `x`, so `τ_b < τ_a` by an ulp and
    // `b` first; with the two swapped, `a` first. No tolerance merges them.
    for x in [0.75f32, 1.0, 3.0] {
        let up = f32::from_bits(x.to_bits() + 1);
        let start = CutDistances::ungated(-1.0f32, -1.0);
        assert_eq!(
            d32(start, CutDistances::ungated(x, up)),
            vec![2, 0],
            "crossings out of order or miscounted: an ulp-apart near-tie"
        );
        assert_eq!(
            d32(start, CutDistances::ungated(up, x)),
            vec![0, 2],
            "crossings out of order or miscounted: an ulp-apart near-tie"
        );
    }
}

#[test]
fn crossing_tau_sort() {
    // The significand, read from the bits: in [1, 2), the sign dropped.
    for (x, s) in [(1.5f64, 1.5f64), (-6.0, 1.5), (0.75, 1.5), (1.0, 1.0)] {
        assert_eq!(x.significand(), s, "significand of {x} at f64");
        assert_eq!(
            (x as f32).significand(),
            s as f32,
            "significand of {x} at f32"
        );
    }
    check_tau_sort(detect::<f32>, detect::<f64>);
    check_product_less(product_less::<f32>, product_less::<f64>);
}

/// The `τ` order by the plain cross products, which underflow at f32 for distances of order 2⁻¹⁰⁰ and tie.
#[cfg(feature = "controls")]
fn plain_products<T: IeeeBits>(start: CutDistances<T>, end: CutDistances<T>) -> Vec<u32> {
    let mut out = detect(start, end);
    if out.len() == 2 {
        let abs = |x: T| if x < T::default() { -x } else { x };
        let b_first = abs(start.b) * abs(end.a) < abs(start.a) * abs(end.b);
        out.sort_by_key(|&s| (s >> 1 == 0) == b_first);
    }
    out
}

negative_control!(
    crossing_tau_sort,
    "the plain cross products underflow at f32 and tie, putting a first",
    expected = "crossings out of order or miscounted",
    check_tau_sort(plain_products::<f32>, plain_products::<f64>)
);

// ── crossing_half_open (REQ-INT-048) ─────────────────────────────────────────────────────────────────────────────────

/// A side change of `a` alone over each substep of `ds`, as the detector counts it.
fn a_stream<T: IeeeBits>(detect: Detect<T>, ds: &[T]) -> Vec<Vec<u32>> {
    ds.windows(2)
        .map(|w| {
            detect(
                CutDistances::ungated(w[0], T::default()),
                CutDistances::ungated(w[1], T::default()),
            )
        })
        .collect()
}

/// No duplicate or missed crossing over `ds`, `a`'s distances, at most one crossing a substep: the crossings are
/// exactly the substeps whose endpoints' sides differ, alternating generator and inverse, so the stream reduces to the
/// one symbol, or none, that the first and last endpoints' sides call for.
fn check_sequence<T: Fixture>(detect: Detect<T>, ds: &[T]) {
    let side = |d: T| negative(d);
    let per_substep = a_stream(detect, ds);
    for (i, c) in per_substep.iter().enumerate() {
        let changes = side(ds[i]) != side(ds[i + 1]);
        assert_eq!(
            c.len(),
            usize::from(changes),
            "a duplicate or missed crossing at substep {i} of {ds:?}"
        );
    }
    let reduced = reduce(&per_substep.concat()).symbols;
    let expected = match (side(ds[0]), side(ds[ds.len() - 1])) {
        (true, false) => vec![0],
        (false, true) => vec![1],
        _ => vec![],
    };
    assert_eq!(
        reduced, expected,
        "a duplicate or missed crossing over {ds:?}"
    );
}

fn check_half_open<T: Fixture>(detect: Detect<T>) {
    check_cases(detect, &fixture("endpoint exactly on a cut"));
    // −0 and the subnormals read as zero, on the cut; the least negative normal is negative.
    let sub = T::negative_subnormal();
    let normal = T::negative_least_normal();
    let (neg, pos) = (T::of(-1.0), T::of(1.0));
    let zero = T::default();
    assert!(
        reads_as_zero(sub) && reads_as_zero(-sub) && reads_as_zero(-zero) && reads_as_zero(zero)
    );
    assert!(!reads_as_zero(normal) && !reads_as_zero(-normal));
    assert!(
        negative(normal) && negative(neg),
        "a negative normal is on the negative side"
    );
    assert!(
        !negative(sub)
            && !negative(-zero)
            && !negative(zero)
            && !negative(-normal)
            && !negative(pos),
        "zero, a subnormal and a positive are on the non-negative side"
    );
    for ds in [
        vec![neg, sub, pos],
        vec![neg, sub, neg],
        vec![pos, sub, neg],
        vec![neg, -zero, sub, -sub, normal],
        vec![normal, sub, normal, zero, normal],
    ] {
        check_sequence(detect, &ds);
    }
    assert_eq!(
        a_stream(detect, &[neg, sub, pos]),
        vec![vec![0], vec![]],
        "a duplicate or missed crossing"
    );
    assert_eq!(
        a_stream(detect, &[normal, -normal]),
        vec![vec![0]],
        "a duplicate or missed crossing"
    );
    let values = [neg, pos, zero, -zero, sub, -sub, normal, -normal];
    prop::run(&proptest::collection::vec(0..values.len(), 2..24), |idx| {
        let ds: Vec<T> = idx.iter().map(|&i| values[i]).collect();
        check_sequence(detect, &ds);
        Ok(())
    });
}

#[test]
fn crossing_half_open() {
    check_half_open(detect::<f32>);
    check_half_open(detect::<f64>);
}

/// A detector where zero is neither side: a crossing counted on each strict sign change, `d < 0` to `d > 0` and back,
/// so a segment from or to the cut is never counted.
#[cfg(feature = "controls")]
fn strict_sign<T: IeeeBits>(start: CutDistances<T>, end: CutDistances<T>) -> Vec<u32> {
    let zero = T::default();
    let mut out = Vec::new();
    for (d0, d1, g) in [(start.a, end.a, 0), (start.b, end.b, 2)] {
        if d0 < zero && d1 > zero {
            out.push(g);
        } else if d0 > zero && d1 < zero {
            out.push(g + 1);
        }
    }
    out
}

negative_control!(
    crossing_half_open,
    "counting strict sign changes misses a crossing through an endpoint on the cut",
    expected = "crossings out of order or miscounted",
    check_half_open(strict_sign::<f64>)
);

// ── crossing_segment_gate (integrator contract Part 5, the segment gate) ───────────────────────────────────────────────

/// One endpoint of §1's cuts: the shared distance `d`, and the body between, which opens cut `a`'s gate (body 0) or cut
/// `b`'s (body 2), or neither (body 1).
fn shared(d: f64, between: u32) -> CutDistances<f64> {
    CutDistances {
        a: d,
        b: d,
        gate_a: between == 0,
        gate_b: between == 2,
    }
}

/// The gate read at the substep's non-negative endpoint: in the positive sense the end's, in the negative the start's;
/// the on-cut endpoint (`d = 0`, `−0`) is the non-negative one; one crossing at most, the gates being exclusive.
fn check_segment_gate(detect: Detect<f64>) {
    let (neg, pos, zero) = (-1.0, 1.0, 0.0);
    // (start d, start body, end d, end body, crossings)
    let cases: [(f64, u32, f64, u32, &[u32]); 14] = [
        // Positive sense: the end's gate.
        (neg, 1, pos, 0, &[0]),
        (neg, 1, pos, 2, &[2]),
        (neg, 0, pos, 1, &[]),
        (neg, 2, pos, 1, &[]),
        (neg, 0, zero, 2, &[2]),
        (neg, 2, -zero, 0, &[0]),
        // Negative sense: the start's gate.
        (pos, 0, neg, 1, &[1]),
        (pos, 2, neg, 1, &[3]),
        (pos, 1, neg, 0, &[]),
        (zero, 2, neg, 0, &[3]),
        (-zero, 0, neg, 2, &[1]),
        // No side change: no crossing, whatever the gates.
        (neg, 0, neg, 2, &[]),
        (pos, 0, zero, 2, &[]),
        // Both gates of one endpoint can't open together; across the two endpoints only the non-negative one counts.
        (neg, 2, pos, 0, &[0]),
    ];
    for (d0, k0, d1, k1, want) in cases {
        assert_eq!(
            detect(shared(d0, k0), shared(d1, k1)),
            want.to_vec(),
            "the segment gate: d {d0} (body {k0} between) to {d1} (body {k1})"
        );
    }
}

#[test]
fn crossing_segment_gate() {
    check_segment_gate(detect::<f64>);
}

/// A detector that reads each gate at the substep's start, whatever the sense.
#[cfg(feature = "controls")]
fn gate_at_start(start: CutDistances<f64>, end: CutDistances<f64>) -> Vec<u32> {
    detect(
        start,
        CutDistances {
            gate_a: start.gate_a,
            gate_b: start.gate_b,
            ..end
        },
    )
}

negative_control!(
    crossing_segment_gate,
    "a gate read at the start endpoint names the wrong segment in the positive sense",
    expected = "the segment gate",
    check_segment_gate(gate_at_start)
);

// ── crossing_state_cuts (symbolic_dynamics_contract §1: d, the classifier, the loops) ────────────────────────────────

/// The cuts from positions: [`state_cuts`], or a control's faulty one.
type Cuts<T> = fn(&[[T; 2]; 3]) -> CutDistances<T>;

/// The exact longest-side body of integer positions: the greatest squared separation, the lowest body on a tie.
fn exact_between(r: [[i64; 2]; 3]) -> u32 {
    let sq = |i: usize, j: usize| (r[j][0] - r[i][0]).pow(2) + (r[j][1] - r[i][1]).pow(2);
    let n = [sq(1, 2), sq(2, 0), sq(0, 1)];
    let mut k = 0;
    for j in 1..3 {
        if n[j] > n[k] {
            k = j;
        }
    }
    k as u32
}

/// The exact `d = −((r₁ − r₀) × (r₂ − r₀))` of integer positions.
fn exact_d(r: [[i64; 2]; 3]) -> i64 {
    let u = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let w = [r[2][0] - r[0][0], r[2][1] - r[0][1]];
    -(u[0] * w[1] - u[1] * w[0])
}

/// The bodies between at a loop's samples, and the crossings the loop appends, freely reduced: `n` positions
/// `place(θ)` for `θ` stepping once round from `θ₀`, closed by its first sample again.
fn loop_word<T: Fixture>(cuts: Cuts<T>, place: impl Fn(f64) -> [[f64; 2]; 3]) -> Vec<u32> {
    let n = 64;
    let at = |k: usize| {
        let theta = 0.01 + std::f64::consts::TAU * k as f64 / n as f64;
        let r = place(theta);
        r.map(|p| p.map(|x| T::of_rounded(x)))
    };
    let mut detector = CrossingDetector::new(cuts(&at(0)));
    let mut stream = Vec::new();
    for k in 1..=n {
        let c = detector.accept(cuts(&at(k % n)));
        stream.extend_from_slice(&c.symbols[..c.count as usize]);
    }
    reduce(&stream).symbols
}

fn check_state_cuts<T: Fixture>(cuts: Cuts<T>) {
    // Integer positions, exact at both precisions, at three scales: d against the exact cross product, the
    // classifier against the exact longest side. At f32 the smallest scale puts the squared separations near 2⁻¹⁴⁰,
    // below the normal range, where only the mantissa-and-exponent form keeps them apart.
    let coordinate = -6i64..=6;
    let point = (coordinate.clone(), coordinate);
    prop::run(&[point.clone(), point.clone(), point], |p| {
        let r = [[p[0].0, p[0].1], [p[1].0, p[1].1], [p[2].0, p[2].1]];
        for scale in [-70, 0, 40] {
            let s = 2f64.powi(scale);
            let x = r.map(|q| q.map(|v| T::of(v as f64 * s)));
            let c = cuts(&x);
            let d = exact_d(r);
            // d's float, and its side, as the half-open rule reads it: below the normal range it reads as zero.
            let want_side = d < 0 && !reads_as_zero(T::of_rounded(d as f64 * s * s));
            prop_assert_eq!(
                negative(c.a),
                want_side,
                "d's side at {:?}·2^{}: d = {}",
                r,
                scale,
                d
            );
            prop_assert!(c.a == c.b, "the cuts share d");
            let k = exact_between(r);
            prop_assert_eq!(
                (c.gate_a, c.gate_b),
                (k == 0, k == 2),
                "the classifier at {:?}·2^{}: body {} between",
                r,
                scale,
                k
            );
        }
        Ok(())
    });
    // d's value at scale 1, where it is exact: the cross product itself, negated.
    let r = [[0.0, 0.0], [3.0, 1.0], [-2.0, 5.0]];
    let c = cuts(&r.map(|p| p.map(T::of)));
    assert_eq!(c.a, T::of(-17.0), "d at {r:?}");
    // The classifier's ties and near-ties: two longest sides equal, in each pairing, and one longer by the least step
    // of the squares, in each position.
    for (r, k) in [
        ([[0, 0], [2, 0], [1, 5]], 0), // n₀ = n₁ = 26 > n₂ = 4
        ([[1, 5], [0, 0], [2, 0]], 1), // n₁ = n₂ = 26 > n₀ = 4
        ([[0, 0], [1, 5], [2, 0]], 0), // n₀ = n₂ = 26 > n₁ = 4
        ([[0, 0], [2, 0], [1, 1]], 2), // n₂ = 4 > n₀ = n₁ = 2
        ([[0, 0], [0, 0], [0, 0]], 0), // all zero
        ([[0, 0], [1, 0], [0, 5]], 0), // n₀ = 26 > n₁ = 25 > n₂ = 1
        ([[0, 0], [1, 0], [1, 5]], 1), // n₁ = 26 > n₀ = 25 > n₂ = 1
        ([[0, 0], [1, 5], [1, 0]], 2), // n₂ = 26 > n₀ = 25 > n₁ = 1
    ] {
        assert_eq!(exact_between(r), k, "the reference at {r:?}");
        let x = r.map(|q| q.map(|v| T::of(v as f64)));
        let c = cuts(&x);
        assert_eq!(
            (c.gate_a, c.gate_b),
            (k == 0, k == 2),
            "the classifier at {r:?}: body {k} between"
        );
    }
    // d against the shape sphere's n_w: d has the sign of −n_w, so it is negative on L⁺'s hemisphere.
    prop::run(
        &(
            proptest::array::uniform3(proptest::array::uniform2(-4.0f64..4.0)),
            proptest::array::uniform3(0.1f64..1.0),
        ),
        |(r, m)| {
            let total = m[0] + m[1] + m[2];
            let m = m.map(|x| x / total);
            let w = kernel::shape::shape(r, m)[2];
            prop_assume!(w.abs() > 1e-3);
            let x = r.map(|p| p.map(|v| T::of_rounded(v)));
            prop_assert_eq!(
                negative(cuts(&x).a),
                w > 0.0,
                "d's side against n_w = {} at {:?}",
                w,
                r
            );
            Ok(())
        },
    );
    // Loops about each collision point, ε = 1/8 from it, the third body at distance 1: about P₀₁ (bodies 0 and 1)
    // the loop reads A; about P₁₂, b; about P₀₂, the puncture at infinity, aB, Appendix B's class for it.
    let e = 0.125;
    let ring = move |t: f64| [e * t.cos(), e * t.sin()];
    assert_eq!(
        loop_word(cuts, |t| [[0.0, 0.0], ring(t), [1.0, 0.0]]),
        vec![1],
        "the loop about P01 reads A"
    );
    assert_eq!(
        loop_word(cuts, |t| [[1.0, 0.0], [0.0, 0.0], ring(t)]),
        vec![2],
        "the loop about P12 reads b"
    );
    assert_eq!(
        loop_word(cuts, |t| [[0.0, 0.0], [1.0, 0.0], ring(t)]),
        vec![0, 3],
        "the loop about P02 reads aB"
    );
}

#[test]
fn crossing_state_cuts() {
    check_state_cuts::<f32>(state_cuts::<f32>);
    check_state_cuts::<f64>(state_cuts::<f64>);
}

/// The cuts with `d`'s sign flipped, `+(r₁ − r₀) × (r₂ − r₀)`: positive on `L⁺`'s hemisphere.
#[cfg(feature = "controls")]
fn flipped<T: IeeeBits>(r: &[[T; 2]; 3]) -> CutDistances<T> {
    let c = state_cuts(r);
    CutDistances {
        a: -c.a,
        b: -c.b,
        ..c
    }
}

negative_control!(
    crossing_state_cuts,
    "d signed as +n_w puts L⁺'s hemisphere on the non-negative side",
    expected = "d's side",
    check_state_cuts::<f64>(flipped::<f64>)
);

// ── crossing_published_words (REQ-PAY-070's cross-check) ─────────────────────────────────────────────────────────────

/// A word of letters `a`, `A`, `b`, `B` as symbol codes.
fn codes(word: &str) -> Vec<u32> {
    word.chars().map(|c| symbol(&c.to_string())).collect()
}

/// `w` freely and cyclically reduced: a symbol beside its inverse cancels, then the ends while they are inverses.
fn cyclic(w: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for &s in w {
        if out.last() == Some(&(s ^ 1)) {
            out.pop();
        } else {
            out.push(s);
        }
    }
    while out.len() > 1 && out[0] == out[out.len() - 1] ^ 1 {
        out.remove(0);
        out.pop();
    }
    out
}

/// R-630 item 5's comparison of `got` against `published`, both cyclically reduced: the first of conjugacy, the
/// inverse, the `(a, b) ↔ (A, B)` exchange and both that makes `got` a cyclic rotation of `published`.
fn symmetry(got: &[u32], published: &[u32]) -> Option<&'static str> {
    let rotation = |w: &[u32]| {
        w.len() == published.len()
            && (0..published.len().max(1))
                .any(|i| published[i..].iter().chain(&published[..i]).eq(w.iter()))
    };
    let inverse: Vec<u32> = got.iter().rev().map(|s| s ^ 1).collect();
    let exchange: Vec<u32> = got.iter().map(|s| s ^ 1).collect();
    let both: Vec<u32> = inverse.iter().map(|s| s ^ 1).collect();
    [
        ("conjugacy", got.to_vec()),
        ("inverse", inverse),
        ("exchange", exchange),
        ("inverse and exchange", both),
    ]
    .into_iter()
    .find(|(_, w)| rotation(w))
    .map(|(name, _)| name)
}

/// The cuts from each orbit's kept states, through the detector, at f64: the cyclically reduced word is the fixture's,
/// and it matches the published word up to the symmetry the fixture records.
fn check_published_words(cuts: Cuts<f64>) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/crossings/prl_orbits.json"
    );
    let text = std::fs::read_to_string(path).expect("read the PRL orbits fixture");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("the fixture is JSON");
    let orbits = doc["orbits"].as_array().expect("orbits");
    assert_eq!(orbits.len(), 10, "the fixture's ten orbits");
    for orbit in orbits {
        let name = orbit["name"].as_str().expect("name");
        let states: Vec<[[f64; 2]; 3]> = orbit["states"]
            .as_array()
            .expect("states")
            .iter()
            .map(|s| {
                let body = |k: usize| [s[k][0].as_f64().expect("x"), s[k][1].as_f64().expect("y")];
                [body(0), body(1), body(2)]
            })
            .collect();
        let mut detector = CrossingDetector::new(cuts(&states[0]));
        let mut stream = Vec::new();
        for r in &states[1..] {
            let c = detector.accept(cuts(r));
            stream.extend_from_slice(&c.symbols[..c.count as usize]);
        }
        let got = cyclic(&stream);
        let want = codes(orbit["word"].as_str().expect("word"));
        assert_eq!(got, want, "{name}: the word read is not the fixture's");
        let published = cyclic(&codes(orbit["published"].as_str().expect("published")));
        assert_eq!(
            symmetry(&got, &published),
            Some(orbit["symmetry"].as_str().expect("symmetry")),
            "{name}: the word read is not the published one up to its symmetry"
        );
    }
}

#[test]
fn crossing_published_words() {
    check_published_words(state_cuts::<f64>);
    // The comparison itself, on a word none of whose symmetries is a rotation of it: each symmetry once, and a word
    // that none reaches.
    let w = codes("aabAB");
    assert_eq!(symmetry(&codes("abABa"), &w), Some("conjugacy"));
    assert_eq!(symmetry(&codes("baBAA"), &w), Some("inverse"));
    assert_eq!(symmetry(&codes("AABab"), &w), Some("exchange"));
    assert_eq!(symmetry(&codes("BAbaa"), &w), Some("inverse and exchange"));
    assert_eq!(symmetry(&codes("aabab"), &w), None);
    assert_eq!(symmetry(&codes("aab"), &w), None);
    assert_eq!(cyclic(&codes("BaAab")), codes("a"), "cyclic reduction");
}

/// The cuts with the outer bodies' segments swapped: cut `a` on body 2's segment, `b` on body 0's.
#[cfg(feature = "controls")]
fn swapped_segments(r: &[[f64; 2]; 3]) -> CutDistances<f64> {
    let c = state_cuts(r);
    CutDistances {
        gate_a: c.gate_b,
        gate_b: c.gate_a,
        ..c
    }
}

negative_control!(
    crossing_published_words,
    "cut a on body 2's segment and b on body 0's reads another word",
    expected = "the word read is not the fixture's",
    check_published_words(swapped_segments)
);

// ── crossing_start_on_cut, crossing_substep_limits (integrator contract Part 5) ──────────────────────────────────────

/// The cases of `fixtures/crossings/position_marches.json` named `fixture`, at the precision `T`: the march from each
/// case's states appends its letters, in order, the first state only seeding the detector.
fn check_position_marches<T: Fixture>(cuts: Cuts<T>, fixture: &str) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/crossings/position_marches.json"
    );
    let text = std::fs::read_to_string(path).expect("read the position marches fixture");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("the fixture is JSON");
    let cases: Vec<&serde_json::Value> = doc["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .filter(|c| c["fixture"] == fixture)
        .collect();
    assert!(!cases.is_empty(), "no case of fixture {fixture}");
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let states: Vec<[[T; 2]; 3]> = case["states"]
            .as_array()
            .expect("states")
            .iter()
            .map(|s| {
                let body = |k: usize| {
                    [
                        T::of(s[k][0].as_f64().expect("x")),
                        T::of(s[k][1].as_f64().expect("y")),
                    ]
                };
                [body(0), body(1), body(2)]
            })
            .collect();
        let mut detector = CrossingDetector::new(cuts(&states[0]));
        let mut got = Vec::new();
        for r in &states[1..] {
            let c = detector.accept(cuts(r));
            got.extend_from_slice(&c.symbols[..c.count as usize]);
        }
        assert_eq!(
            got,
            codes(case["letters"].as_str().expect("letters")),
            "{name}: the march's letters"
        );
    }
}

#[test]
fn crossing_start_on_cut() {
    check_position_marches::<f32>(state_cuts::<f32>, "start");
    check_position_marches::<f64>(state_cuts::<f64>, "start");
}

negative_control!(
    crossing_start_on_cut,
    "d signed the other way reads the start's other direction",
    expected = "the march's letters",
    check_position_marches::<f64>(flipped::<f64>, "start")
);

#[test]
fn crossing_substep_limits() {
    check_position_marches::<f32>(state_cuts::<f32>, "limit");
    check_position_marches::<f64>(state_cuts::<f64>, "limit");
}

negative_control!(
    crossing_substep_limits,
    "d signed the other way reads a coarse substep's letter inverted",
    expected = "the march's letters",
    check_position_marches::<f64>(flipped::<f64>, "limit")
);

// ── crossing_no_subnormal_distance (R-547; gpu_determinism_note, rule 6) ─────────────────────────────────────────────

/// A distance from positions: [`signed_distance`], or a control's faulty one.
type Distance<T> = fn(&[[T; 2]; 3]) -> T;

/// `d = −y·|x|` at positions on the axes, `r₁ = (0, y)`, `r₂ = (−|x|, 0)`, its operands exact, against the side and
/// value it must read: at the normal range's foot the least normal, negative; one ulp of `y` below it zero, though
/// its float rounds up to the least normal at a backend keeping subnormals (`round_up`); a whole binade below, zero.
fn check_no_subnormal_distance<T: Fixture>(
    distance: Distance<T>,
    half_range: f64,
    below_two: f64,
    least: f64,
) {
    let zero = T::of(0.0);
    let at = |y: f64, x: f64| distance(&[[zero, zero], [zero, T::of(y)], [T::of(-x), zero]]);
    let x = half_range;
    let foot = at(half_range, x);
    assert_eq!(foot, T::of(-least), "the least normal is exact");
    assert!(negative(foot), "the least normal is on the negative side");
    let round_up = at(below_two * half_range / 2.0, x);
    assert_eq!(
        round_up.exponent(),
        zero.exponent(),
        "a distance below the normal range does not read as zero"
    );
    assert!(
        !negative(round_up),
        "a distance below the normal range is on the cut"
    );
    assert!(
        !negative(at(half_range / 2.0, x)),
        "a binade below is on the cut"
    );
    assert!(negative(at(1.0, 1.0)), "an ordinary distance");
}

#[test]
fn crossing_no_subnormal_distance() {
    // f32: `(2 − 2⁻²³)·2⁻⁶⁴ × 2⁻⁶³`, the code review's value; f64: `(2 − 2⁻⁵²)·2⁻⁵¹² × 2⁻⁵¹¹`.
    check_no_subnormal_distance::<f32>(
        signed_distance::<f32>,
        2f64.powi(-63),
        2.0 - 2f64.powi(-23),
        f64::from(f32::MIN_POSITIVE),
    );
    check_no_subnormal_distance::<f64>(
        signed_distance::<f64>,
        2f64.powi(-511),
        2.0 - 2f64.powi(-52),
        f64::MIN_POSITIVE,
    );
}

/// `d` as plain float arithmetic: the product rounds to the least normal where the exact value is below it, so a
/// keeping backend reads the negative side and a flushing one would not.
#[cfg(feature = "controls")]
fn rounded_distance(r: &[[f32; 2]; 3]) -> f32 {
    (r[1][1] - r[0][1]) * (r[2][0] - r[0][0]) - (r[1][0] - r[0][0]) * (r[2][1] - r[0][1])
}

negative_control!(
    crossing_no_subnormal_distance,
    "plain arithmetic's product rounds up to the least normal, so a branch reads a backend-dependent side",
    expected = "does not read as zero",
    check_no_subnormal_distance::<f32>(
        rounded_distance,
        2f64.powi(-63),
        2.0 - 2f64.powi(-23),
        f64::from(f32::MIN_POSITIVE),
    )
);
