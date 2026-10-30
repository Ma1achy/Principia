//! Branch-cut crossing detection (`kernel::word::crossing`; integrator contract Part 5, the τ tie rule; dd_simstate_payload
//! §3), on the synthetic fixtures of `fixtures/crossings/segments.json`, at f32 and f64. Which cuts carry `a` and `b` is
//! RQ-632, so every case gives the cuts' signed distances directly.
//! - REQ-INT-031: a double crossing in one substep orders by `τ`; equal `τ` uses the cut ID; an endpoint exactly on the
//!   cut follows the half-open rule (`crossing_order`).
//! - REQ-INT-046: a cross-and-recross inside one macro-step yields both crossings, and the word they leave
//!   (`crossing_every_substep`).
//! - REQ-INT-047: a segment crossing both cuts near their shared endpoint appends in `τ` order, distances below f32's
//!   product range included; the product comparison against the exact one (`crossing_tau_sort`).
//! - REQ-INT-048: endpoints exactly on a cut, `−0` and subnormals among them, give no duplicate or missed crossing
//!   (`crossing_half_open`). The GPU arm is REQ-VAL-072's at M4 (R-150).
//!
//! Each check takes the detector as a function, so its control runs the same check on a faulty one.

use kernel::payload::{fgw_length_raw, sd_last_symbol, FreeGroupWord, FGW_NO_SYMBOL};
use kernel::real::IeeeBits;
use kernel::word::crossing::{negative, product_less, reads_as_zero};
use kernel::word::{
    fgw_decode, set_last_symbol, substep_crossings, CrossingDetector, CrossingHook, CutDistances,
};
use proptest::prelude::*;
use validation::reference::free_group::reduce;
use validation::{negative_control, prop};

/// A float the fixtures are read at: f32 or f64, each fixture value exact at both.
trait Fixture: IeeeBits {
    fn of(v: f64) -> Self;
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
    CutDistances {
        a: T::of(e.0),
        b: T::of(e.1),
    }
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
        let start = CutDistances {
            a: -1.0f32,
            b: -1.0,
        };
        assert_eq!(
            d32(start, CutDistances { a: x, b: up }),
            vec![2, 0],
            "crossings out of order or miscounted: an ulp-apart near-tie"
        );
        assert_eq!(
            d32(start, CutDistances { a: up, b: x }),
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
                CutDistances {
                    a: w[0],
                    b: T::default(),
                },
                CutDistances {
                    a: w[1],
                    b: T::default(),
                },
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
