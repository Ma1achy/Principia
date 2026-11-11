//! QA tests for TASK-M3-07, the regularisation axis, `none`, the pair table and the `dt_max` diagnostic, written from
//! the requirements and the contract text (integrator_contract Part 2a, Part 2b § "Every stepper × regularisation pair
//! runs; …", Part 2b's step 8, Part 2c rule 4 and its `dt_max` definition), not from the implementation:
//! - REQ-TOOL-039, REQ-TOOL-125 (`qa_m307_dt_max_edges`, `qa_m307_dt_max_property_f64`, `qa_m307_dt_max_property_f32`):
//!   the diagnostic against a model written here from Part 2c's definition: a step is usable when its exponent field
//!   is not all ones, decided on its bits; every usable step is reported as it stands (zero, negative, subnormal, the
//!   largest finite); the first usable step seeds the fold and a later one replaces it only where it compares greater;
//!   the non-usable steps are counted; no usable step is the explicit sentinel. Over hand-picked edges and over
//!   arbitrary bit patterns (NaN payloads of both signs, `±∞`, subnormals, signed zeros) at f64 and f32.
//! - REQ-TOOL-039 (`qa_m307_no_usable_step_reads_apart_from_zero`): "no usable step" reads apart from a step of zero,
//!   `+0.0` or `−0.0`, with or without skipped steps beside it (pitfalls §9).
//! - REQ-TOOL-039 (`qa_m307_overflow_2_209e128`): the `2.209e128` overflow is reported as itself at f64, wherever it
//!   falls in the march, and at f32, where it is `+∞`, skipped and counted.
//! - REQ-INT-017's arm here (`qa_m307_pair_table_part_2b`, `qa_m307_cell_follows_time_transformation`): every pair is
//!   selectable; the shipped table is Part 2b's, transcribed here, the measured-bad cells with note 2's warning and the
//!   others none; on a synthetic table whose Heggie Eq. 20 and Eq. 22 cells differ for one pair (and logH's two modes),
//!   the warning follows the time transformation in force.
//! - REQ-INT-021 (`qa_m307_owns_time_mapping`): over every pair, and over the composed types, `owns_time_mapping ==
//!   (regularisation != none)`.
//! - `none` (`qa_m307_none_is_the_stepper`): a stepper under `none` is the stepper's own `ADVANCE`, bit for bit, with a
//!   cadence that stops the march early and over an empty span, at f64 and f32.
//!
//! Each check takes the code under test as a function, so its negative control (R-176) runs the same check on a faulty
//! one.

use kernel::driver::advance::{Advance, AdvanceParams, AdvanceReal, Advanced, Cadence};
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{
    CapabilityProfile, DebugOccupant, Geom, Occupant, Selection, State, Stepper,
};
use kernel::regularisation::dt_max::{DtMax, DtMaxReport};
use kernel::regularisation::pairs::{Choice, HeggieTime, LogHMode, Pair, PairTable, Status};
use kernel::regularisation::{Composed, Kind, NoRegularisation, Regularisation};
use proptest::prelude::*;
use validation::{negative_control, prop};

// ── dt_max: the model, from Part 2c ────────────────────────────────────────────────────────────────────────────────

/// A step as the model sees it: whether its exponent field is all ones (read from the bits), its value widened to f64
/// (exact and order-preserving from f32, signed zeros kept), and its bits, for comparing the reported value.
#[derive(Clone, Copy, Debug)]
struct Step {
    exponent_all_ones: bool,
    value: f64,
    bits: u64,
}

fn step64(x: f64) -> Step {
    let b = x.to_bits();
    Step {
        exponent_all_ones: (b >> 52) & 0x7ff == 0x7ff,
        value: x,
        bits: b,
    }
}

fn step32(x: f32) -> Step {
    let b = x.to_bits();
    Step {
        exponent_all_ones: (b >> 23) & 0xff == 0xff,
        value: f64::from(x),
        bits: u64::from(b),
    }
}

/// Part 2c's `dt_max`: the index of the step reported (`None`: no usable step) and the count of skipped steps.
fn model(steps: &[Step]) -> (Option<usize>, u32) {
    let mut best: Option<usize> = None;
    let mut skipped = 0u32;
    for (i, s) in steps.iter().enumerate() {
        if s.exponent_all_ones {
            skipped += 1;
        } else {
            match best {
                None => best = Some(i),
                Some(k) if s.value > steps[k].value => best = Some(i),
                Some(_) => {}
            }
        }
    }
    (best, skipped)
}

/// A report reduced to `(bits of dt_max, skipped)`, `None` for the sentinel, so signed zeros and payloads compare.
type Key = (Option<u64>, u32);

const MSG: &str = "the report is not the contract's dt_max";

/// Checks that `got` is the model's report of `steps`.
fn check_report(steps: &[Step], got: Key) {
    let (best, skipped) = model(steps);
    let want: Key = (best.map(|k| steps[k].bits), skipped);
    assert_eq!(got, want, "{MSG}: steps {steps:?}");
}

fn key64(r: DtMaxReport<f64>) -> Key {
    match r {
        DtMaxReport::NoUsableStep { skipped } => (None, skipped),
        DtMaxReport::Largest { dt_max, skipped } => (Some(dt_max.to_bits()), skipped),
    }
}

fn key32(r: DtMaxReport<f32>) -> Key {
    match r {
        DtMaxReport::NoUsableStep { skipped } => (None, skipped),
        DtMaxReport::Largest { dt_max, skipped } => (Some(u64::from(dt_max.to_bits())), skipped),
    }
}

/// The kernel's diagnostic fed `steps` one at a time, in march order.
fn kernel64(steps: &[f64]) -> Key {
    key64(
        steps
            .iter()
            .fold(DtMax::new(), |d, &x| d.observe(x))
            .report(),
    )
}

fn kernel32(steps: &[f32]) -> Key {
    key32(
        steps
            .iter()
            .fold(DtMax::new(), |d, &x| d.observe(x))
            .report(),
    )
}

/// The overflowed physical step of pitfalls §9.
const OVERFLOW: f64 = 2.209e128;

/// Hand-picked f64 marches: every edge Part 2c names, and the bit patterns at the finite/non-finite border.
fn edges64() -> Vec<Vec<f64>> {
    let snan = f64::from_bits(0x7ff0_0000_0000_0001);
    let neg_nan = f64::from_bits(0xfff8_0000_0000_0dad);
    let tiny = f64::from_bits(1);
    vec![
        vec![],
        vec![f64::NAN],
        vec![snan, neg_nan, f64::INFINITY, f64::NEG_INFINITY],
        vec![0.0],
        vec![-0.0],
        vec![-0.0, 0.0],
        vec![0.0, -0.0],
        vec![f64::NAN, 0.0, f64::INFINITY],
        vec![-5.0, -3.0, -4.0],
        vec![-3.0, f64::NEG_INFINITY, -4.0],
        vec![tiny],
        vec![-tiny, -0.0, -tiny],
        vec![0.0, tiny, f64::NAN],
        vec![f64::MIN_POSITIVE, tiny],
        vec![f64::MAX, f64::INFINITY],
        vec![f64::INFINITY, f64::MAX],
        vec![-f64::MAX, f64::NEG_INFINITY],
        vec![1e-3, OVERFLOW, 1e-3],
        vec![OVERFLOW],
        vec![snan, OVERFLOW, neg_nan],
        vec![0.5, 0.25, 0.5, 0.125],
        vec![1.0, 2.0, 3.0, 4.0],
        vec![4.0, 3.0, 2.0, 1.0],
    ]
}

/// The same edges at f32.
fn edges32() -> Vec<Vec<f32>> {
    let snan = f32::from_bits(0x7f80_0001);
    let neg_nan = f32::from_bits(0xffc0_0bad);
    let tiny = f32::from_bits(1);
    vec![
        vec![],
        vec![f32::NAN],
        vec![snan, neg_nan, f32::INFINITY, f32::NEG_INFINITY],
        vec![0.0],
        vec![-0.0],
        vec![-0.0, 0.0],
        vec![0.0, -0.0],
        vec![-5.0, -3.0, -4.0],
        vec![tiny, f32::NAN],
        vec![-tiny, -0.0],
        vec![f32::MAX, f32::INFINITY],
        vec![-f32::MAX, f32::NEG_INFINITY],
        vec![1e-3, OVERFLOW as f32, 1e-3],
        vec![OVERFLOW as f32, OVERFLOW as f32],
        vec![4.0, 3.0, 2.0, 1.0],
    ]
}

/// Checks a fold over the f64 edges and the f32 edges against the model.
fn check_edges(fold64: impl Fn(&[f64]) -> Key, fold32: impl Fn(&[f32]) -> Key) {
    for steps in edges64() {
        let model_steps: Vec<Step> = steps.iter().copied().map(step64).collect();
        check_report(&model_steps, fold64(&steps));
    }
    for steps in edges32() {
        let model_steps: Vec<Step> = steps.iter().copied().map(step32).collect();
        check_report(&model_steps, fold32(&steps));
    }
}

#[test]
fn qa_m307_dt_max_edges() {
    // The model's own premises, so a mistake in it cannot pass silently.
    assert!(step64(f64::from_bits(0x7ff0_0000_0000_0001)).exponent_all_ones);
    assert!(!step64(f64::MAX).exponent_all_ones);
    assert!(!step64(f64::from_bits(1)).exponent_all_ones);
    assert!(step32(f32::from_bits(0xffc0_0bad)).exponent_all_ones);
    assert!(!step32(f32::MAX).exponent_all_ones);
    check_edges(kernel64, kernel32);
}

// The faulty folds the controls run the same check on.

/// The fold seeded at `0.0`, `driver.rs`'s, which Part 2c rule 4 forbids.
#[cfg(feature = "controls")]
fn seeded_at_zero(steps: &[f64]) -> Key {
    let finite: Vec<f64> = steps.iter().copied().filter(|x| x.is_finite()).collect();
    let skipped = (steps.len() - finite.len()) as u32;
    let m = finite.iter().copied().fold(0.0, f64::max);
    (Some(m.to_bits()), skipped)
}

#[cfg(feature = "controls")]
fn seeded_at_zero32(steps: &[f32]) -> Key {
    let wide: Vec<f64> = steps.iter().map(|&x| f64::from(x)).collect();
    let (m, s) = seeded_at_zero(&wide);
    (
        m.map(|b| u64::from((f64::from_bits(b) as f32).to_bits())),
        s,
    )
}

/// A fold that replaces on an equal step (`>=`), which Part 2c's "only where it compares greater" forbids.
#[cfg(feature = "controls")]
fn replaces_on_equal(steps: &[f64]) -> Key {
    let mut best: Option<f64> = None;
    let mut skipped = 0;
    for &x in steps {
        if !x.is_finite() {
            skipped += 1;
        } else if best.is_none_or(|b| x >= b) {
            best = Some(x);
        }
    }
    (best.map(f64::to_bits), skipped)
}

/// A fold that flushes subnormal steps to zero, where Part 2c reports every finite step as it stands.
#[cfg(feature = "controls")]
fn flushes_subnormals(steps: &[f64]) -> Key {
    let flushed: Vec<f64> = steps
        .iter()
        .map(|&x| if x.is_subnormal() { 0.0 } else { x })
        .collect();
    kernel64(&flushed)
}

/// A fold that takes `±∞` as usable, skipping NaN alone.
#[cfg(feature = "controls")]
fn infinity_usable(steps: &[f64]) -> Key {
    let mut best: Option<f64> = None;
    let mut skipped = 0;
    for &x in steps {
        if x.is_nan() {
            skipped += 1;
        } else if best.is_none_or(|b| x > b) {
            best = Some(x);
        }
    }
    (best.map(f64::to_bits), skipped)
}

negative_control!(
    qa_m307_dt_max_edges,
    "the fold seeded at 0.0 (Part 2c rule 4)",
    expected = "the report is not the contract's dt_max",
    check_edges(seeded_at_zero, seeded_at_zero32)
);

#[cfg(feature = "controls")]
mod equal_replaces {
    use super::*;
    negative_control!(
        qa_m307_dt_max_edges,
        "a fold that replaces the maximum on an equal step",
        expected = "the report is not the contract's dt_max",
        check_edges(replaces_on_equal, kernel32)
    );
}

#[cfg(feature = "controls")]
mod subnormals_flushed {
    use super::*;
    negative_control!(
        qa_m307_dt_max_edges,
        "a fold that flushes subnormal steps to zero",
        expected = "the report is not the contract's dt_max",
        check_edges(flushes_subnormals, kernel32)
    );
}

#[cfg(feature = "controls")]
mod infinity_kept {
    use super::*;
    negative_control!(
        qa_m307_dt_max_edges,
        "a fold that takes an infinite step as usable",
        expected = "the report is not the contract's dt_max",
        check_edges(infinity_usable, kernel32)
    );
}

/// Arbitrary f64 steps, weighted towards the border: any bit pattern, any NaN payload of either sign, `±∞`, signed
/// zeros, subnormals, and `2.209e128`.
fn any_step64() -> impl Strategy<Value = f64> {
    prop_oneof![
        4 => any::<u64>().prop_map(f64::from_bits),
        2 => (any::<bool>(), 1u64..(1 << 52)).prop_map(|(s, f)| {
            f64::from_bits((u64::from(s) << 63) | (0x7ff << 52) | f)
        }),
        1 => Just(f64::INFINITY),
        1 => Just(f64::NEG_INFINITY),
        1 => Just(0.0),
        1 => Just(-0.0),
        1 => (any::<bool>(), 1u64..(1 << 52)).prop_map(|(s, f)| f64::from_bits((u64::from(s) << 63) | f)),
        1 => Just(OVERFLOW),
        2 => -1e3f64..1e3,
    ]
}

fn any_step32() -> impl Strategy<Value = f32> {
    prop_oneof![
        4 => any::<u32>().prop_map(f32::from_bits),
        2 => (any::<bool>(), 1u32..(1 << 23)).prop_map(|(s, f)| {
            f32::from_bits((u32::from(s) << 31) | (0xff << 23) | f)
        }),
        1 => Just(f32::INFINITY),
        1 => Just(f32::NEG_INFINITY),
        1 => Just(0.0f32),
        1 => Just(-0.0f32),
        1 => (any::<bool>(), 1u32..(1 << 23)).prop_map(|(s, f)| f32::from_bits((u32::from(s) << 31) | f)),
        2 => -1e3f32..1e3,
    ]
}

#[test]
fn qa_m307_dt_max_property_f64() {
    prop::run(&proptest::collection::vec(any_step64(), 0..48), |steps| {
        let model_steps: Vec<Step> = steps.iter().copied().map(step64).collect();
        let (best, skipped) = model(&model_steps);
        let want: Key = (best.map(|k| model_steps[k].bits), skipped);
        prop_assert_eq!(kernel64(&steps), want);
        // Feeding the steps in reverse changes at most which of two equal zeros is reported, never the value.
        let rev: Vec<f64> = steps.iter().rev().copied().collect();
        let (a, b) = (kernel64(&steps), kernel64(&rev));
        prop_assert_eq!(a.1, b.1);
        prop_assert_eq!(a.0.map(f64::from_bits), b.0.map(f64::from_bits));
        Ok(())
    });
}

#[test]
fn qa_m307_dt_max_property_f32() {
    prop::run(&proptest::collection::vec(any_step32(), 0..48), |steps| {
        let model_steps: Vec<Step> = steps.iter().copied().map(step32).collect();
        let (best, skipped) = model(&model_steps);
        let want: Key = (best.map(|k| model_steps[k].bits), skipped);
        prop_assert_eq!(kernel32(&steps), want);
        Ok(())
    });
}

// ── REQ-TOOL-039: no usable step reads apart from a step of zero ───────────────────────────────────────────────────

const APART: &str = "no usable step must read apart from a step of zero";

/// Checks that the reports of marches with no usable step are the sentinel, and differ from every march whose
/// largest usable step is a zero.
fn check_apart(fold: impl Fn(&[f64]) -> Key) {
    let nan = f64::NAN;
    let none = [&[][..], &[nan], &[nan, f64::INFINITY], &[f64::NEG_INFINITY]];
    let zero = [
        &[0.0][..],
        &[-0.0],
        &[nan, 0.0],
        &[-1.0, 0.0, f64::INFINITY],
        &[-0.0, nan],
    ];
    for n in none {
        assert_eq!(fold(n).0, None, "{APART}: {n:?} reported a value");
        for z in zero {
            assert_ne!(fold(n), fold(z), "{APART}: {n:?} and {z:?}");
            assert!(
                fold(z).0.is_some_and(|b| f64::from_bits(b) == 0.0),
                "{APART}: {z:?}"
            );
        }
    }
}

#[test]
fn qa_m307_no_usable_step_reads_apart_from_zero() {
    check_apart(kernel64);
    // The sentinel is not a number: its arm holds the count only, and an empty march's count is zero.
    assert_eq!(
        DtMax::<f64>::new().report(),
        DtMaxReport::NoUsableStep { skipped: 0 }
    );
}

negative_control!(
    qa_m307_no_usable_step_reads_apart_from_zero,
    "the fold seeded at 0.0, which reports no usable step as zero",
    expected = "no usable step must read apart from a step of zero",
    check_apart(seeded_at_zero)
);

// ── REQ-TOOL-039: the 2.209e128 overflow is reported ───────────────────────────────────────────────────────────────

const OVER: &str = "the 2.209e128 overflow must be reported";

/// Checks the overflow case at f64 (reported as itself, wherever it falls) and at f32 (`+∞`, skipped and counted).
fn check_overflow(fold64: impl Fn(&[f64]) -> Key, fold32: impl Fn(&[f32]) -> Key) {
    assert!(OVERFLOW.is_finite(), "the premise: finite at f64");
    assert_eq!(OVERFLOW as f32, f32::INFINITY, "the premise: +∞ at f32");
    let small = [1e-3, 0.5, 2.0];
    for at in 0..=small.len() {
        let mut steps = small.to_vec();
        steps.insert(at, OVERFLOW);
        assert_eq!(
            fold64(&steps),
            (Some(OVERFLOW.to_bits()), 0),
            "{OVER} at f64, step {at}"
        );
        let s32: Vec<f32> = steps.iter().map(|&x| x as f32).collect();
        assert_eq!(
            fold32(&s32),
            (Some(u64::from(2.0f32.to_bits())), 1),
            "{OVER} at f32, step {at}"
        );
    }
    assert_eq!(
        fold64(&[OVERFLOW, f64::NAN]),
        (Some(OVERFLOW.to_bits()), 1),
        "{OVER}"
    );
    assert_eq!(
        fold32(&[OVERFLOW as f32]),
        (None, 1),
        "{OVER}: all overflow at f32"
    );
}

#[test]
fn qa_m307_overflow_2_209e128() {
    check_overflow(kernel64, kernel32);
}

/// A fold that judges an implausibly large step unusable, where Part 2c reports every finite step as it stands.
#[cfg(feature = "controls")]
fn drops_huge(steps: &[f64]) -> Key {
    let mut best: Option<f64> = None;
    let mut skipped = 0;
    for &x in steps {
        if !x.is_finite() || x.abs() > 1e100 {
            skipped += 1;
        } else if best.is_none_or(|b| x > b) {
            best = Some(x);
        }
    }
    (best.map(f64::to_bits), skipped)
}

negative_control!(
    qa_m307_overflow_2_209e128,
    "a fold that discards an implausibly large finite step",
    expected = "the 2.209e128 overflow must be reported",
    check_overflow(drops_huge, kernel32)
);

// ── REQ-INT-017's arm: the pair table ──────────────────────────────────────────────────────────────────────────────

const EULER: Selection = Selection::Debug(DebugOccupant::Euler);
const KDK: Selection = Selection::Production(Occupant::Kdk);
const Y4: Selection = Selection::Production(Occupant::Yoshida4);
const Y6: Selection = Selection::Production(Occupant::Yoshida6);
const RK4: Selection = Selection::Production(Occupant::Rk4);
const STEPPERS: [Selection; 5] = [EULER, KDK, Y4, Y6, RK4];

const NONE: Choice = Choice::None;
const AZ: Choice = Choice::AarsethZare;
const H20: Choice = Choice::Heggie(HeggieTime::Eq20);
const H22: Choice = Choice::Heggie(HeggieTime::Eq22);
const LOGH: Choice = Choice::LogH(LogHMode::LogH);
const TTL: Choice = Choice::LogH(LogHMode::Ttl);
const CHOICES: [Choice; 6] = [NONE, AZ, H20, H22, LOGH, TTL];

/// Part 2b's note 2, as the contract prints it.
const NOTE_2: &str = "Heggie's Hamiltonian is not separable, so a leapfrog step is first order here and can diverge \
                      (figure-eight: order about 1, diverged within 100 orbits).";

/// Part 2b's table: the cells it marks, every other one not measured.
fn part_2b_cell(s: Selection, c: Choice) -> Status {
    match (c, s) {
        (AZ, RK4) | (H20, RK4) | (H22, RK4) => Status::MeasuredGood,
        (H20, KDK) | (H22, KDK) => Status::MeasuredBad { warning: NOTE_2 },
        _ => Status::NotMeasured,
    }
}

const TABLE: &str = "the pair table is not Part 2b's";

/// Checks every pair: selectable, as asked, with Part 2b's status, the contract's warning on a measured-bad cell and
/// none on any other. `select` and `warning` are the code under test.
fn check_part_2b(
    select: impl Fn(Selection, Choice) -> Option<Pair>,
    warning: impl Fn(Pair) -> Option<&'static str>,
) {
    let table = PairTable::part_2b();
    for c in CHOICES {
        for s in STEPPERS {
            let pair = select(s, c).unwrap_or_else(|| panic!("{TABLE}: {s:?} × {c:?} is blocked"));
            assert_eq!(
                (pair.stepper, pair.regularisation),
                (s, c),
                "{TABLE}: the pair asked"
            );
            let want = part_2b_cell(s, c);
            assert_eq!(pair.status(&table), want, "{TABLE}: {s:?} × {c:?}");
            match want {
                Status::MeasuredBad { .. } => {
                    assert_eq!(
                        warning(pair),
                        Some(NOTE_2),
                        "{TABLE}: {s:?} × {c:?} has no warning"
                    )
                }
                _ => assert_eq!(warning(pair), None, "{TABLE}: {s:?} × {c:?} warns"),
            }
        }
    }
}

#[test]
fn qa_m307_pair_table_part_2b() {
    check_part_2b(
        |s, c| Some(Pair::select(s, c)),
        |p| p.warning(&PairTable::part_2b()),
    );
    // `none` is always available: under every stepper, with no warning.
    for s in STEPPERS {
        assert_eq!(Pair::select(s, NONE).warning(&PairTable::part_2b()), None);
    }
}

#[cfg(feature = "controls")]
mod blocks_bad {
    use super::*;
    negative_control!(
        qa_m307_pair_table_part_2b,
        "a selection that blocks the measured-bad pairs",
        expected = "the pair table is not Part 2b's",
        check_part_2b(
            |s, c| {
                let p = Pair::select(s, c);
                p.warning(&PairTable::part_2b()).is_none().then_some(p)
            },
            |p| p.warning(&PairTable::part_2b()),
        )
    );
}

#[cfg(feature = "controls")]
mod bad_without_warning {
    use super::*;
    negative_control!(
        qa_m307_pair_table_part_2b,
        "a measured-bad pair shown with no warning",
        expected = "the pair table is not Part 2b's",
        check_part_2b(|s, c| Some(Pair::select(s, c)), |_| None)
    );
}

const FOLLOWS: &str = "the warning must follow the time transformation in force";

/// Checks, on a synthetic table whose Heggie Eq. 20 and Eq. 22 cells differ for Yoshida-4 (and logH's two modes for
/// Yoshida-6), that the warning shown is the selected cell's. `lookup` is the code under test.
fn check_follows(lookup: impl Fn(&PairTable, Selection, Choice) -> Option<&'static str>) {
    let synth = PairTable::not_measured()
        .with(Y4, H20, Status::MeasuredBad { warning: "eq20" })
        .with(Y4, H22, Status::MeasuredGood)
        .with(Y6, LOGH, Status::NotMeasured)
        .with(Y6, TTL, Status::MeasuredBad { warning: "ttl" });
    assert_eq!(lookup(&synth, Y4, H20), Some("eq20"), "{FOLLOWS}: Eq. 20");
    assert_eq!(lookup(&synth, Y4, H22), None, "{FOLLOWS}: Eq. 22");
    assert_eq!(lookup(&synth, Y6, LOGH), None, "{FOLLOWS}: logH mode");
    assert_eq!(lookup(&synth, Y6, TTL), Some("ttl"), "{FOLLOWS}: TTL mode");
    // The reverse assignment, so neither transformation is favoured.
    let flipped = PairTable::not_measured()
        .with(Y4, H22, Status::MeasuredBad { warning: "eq22" })
        .with(Y6, LOGH, Status::MeasuredBad { warning: "logh" });
    assert_eq!(
        lookup(&flipped, Y4, H20),
        None,
        "{FOLLOWS}: Eq. 20, flipped"
    );
    assert_eq!(
        lookup(&flipped, Y4, H22),
        Some("eq22"),
        "{FOLLOWS}: Eq. 22, flipped"
    );
    assert_eq!(
        lookup(&flipped, Y6, LOGH),
        Some("logh"),
        "{FOLLOWS}: logH, flipped"
    );
    assert_eq!(lookup(&flipped, Y6, TTL), None, "{FOLLOWS}: TTL, flipped");
    // Every other cell of both tables is unwarned.
    for c in CHOICES {
        for s in STEPPERS {
            if !matches!((s, c), (Y4, H20) | (Y4, H22) | (Y6, LOGH) | (Y6, TTL)) {
                assert_eq!(lookup(&synth, s, c), None, "{FOLLOWS}: {s:?} × {c:?}");
            }
        }
    }
}

#[test]
fn qa_m307_cell_follows_time_transformation() {
    check_follows(|t, s, c| Pair::select(s, c).warning(t));
}

/// A lookup keyed on the pair alone: the regularisation's default time transformation, whatever is in force.
#[cfg(feature = "controls")]
fn by_pair_alone(t: &PairTable, s: Selection, c: Choice) -> Option<&'static str> {
    let keyed = match c.kind() {
        Kind::Heggie => H22,
        Kind::LogH => LOGH,
        _ => c,
    };
    Pair::select(s, keyed).warning(t)
}

negative_control!(
    qa_m307_cell_follows_time_transformation,
    "a lookup keyed on the stepper and regularisation alone",
    expected = "the warning must follow the time transformation in force",
    check_follows(by_pair_alone)
);

// ── REQ-INT-021: owns_time_mapping ─────────────────────────────────────────────────────────────────────────────────

const OWNS: &str = "owns_time_mapping must be (regularisation != none)";

/// A stand-in regularisation of the given kind whose own profile denies the time mapping, as a stepper's does.
macro_rules! denying {
    ($name:ident, $kind:expr) => {
        struct $name;
        impl Regularisation for $name {
            const KIND: Kind = $kind;
            fn profile<S: Stepper>() -> CapabilityProfile {
                CapabilityProfile {
                    owns_time_mapping: false,
                    ..S::profile()
                }
            }
            fn advance<S: Stepper, R: AdvanceReal, C: Cadence<R>>(
                state: &State<R>,
                geom: &Geom<R>,
                t_now: u32,
                t_target: u32,
                params: &AdvanceParams<R>,
                cadence: &mut C,
            ) -> Advanced<R> {
                <S as Advance>::advance(state, geom, t_now, t_target, params, cadence)
            }
        }
    };
}

denying!(AzStandIn, Kind::AarsethZare);
denying!(HeggieStandIn, Kind::Heggie);
denying!(LogHStandIn, Kind::LogH);

/// Checks `owns(s, c)` over every pair.
fn check_owns(owns: impl Fn(Selection, Choice) -> bool) {
    for c in CHOICES {
        for s in STEPPERS {
            assert_eq!(
                owns(s, c),
                !matches!(c, Choice::None),
                "{OWNS}: {s:?} × {c:?}"
            );
        }
    }
}

/// The composed type's flag for `S` under each kind.
fn composed_flag<S: Stepper>(c: Choice) -> bool {
    match c.kind() {
        Kind::None => Composed::<S, NoRegularisation>::profile().owns_time_mapping,
        Kind::AarsethZare => Composed::<S, AzStandIn>::profile().owns_time_mapping,
        Kind::Heggie => Composed::<S, HeggieStandIn>::profile().owns_time_mapping,
        Kind::LogH => Composed::<S, LogHStandIn>::profile().owns_time_mapping,
    }
}

fn composed_owns(s: Selection, c: Choice) -> bool {
    match s {
        EULER => composed_flag::<Euler>(c),
        KDK => composed_flag::<Kdk>(c),
        Y4 => composed_flag::<Yoshida4>(c),
        Y6 => composed_flag::<Yoshida6>(c),
        _ => composed_flag::<Rk4>(c),
    }
}

#[test]
fn qa_m307_owns_time_mapping() {
    check_owns(|s, c| Pair::select(s, c).owns_time_mapping());
    check_owns(composed_owns);
}

negative_control!(
    qa_m307_owns_time_mapping,
    "the regularisation's own profile read as the composed occupant's",
    expected = "owns_time_mapping must be (regularisation != none)",
    check_owns(|_, c| match c.kind() {
        Kind::Heggie => HeggieStandIn::profile::<Kdk>().owns_time_mapping,
        _ => !matches!(c, Choice::None),
    })
);

// ── `none` is the stepper's own ADVANCE ────────────────────────────────────────────────────────────────────────────

/// A precision the tests run at.
trait P: AdvanceReal + Into<f64> {
    fn of(x: f64) -> Self;
}
impl P for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
}
impl P for f64 {
    fn of(x: f64) -> f64 {
        x
    }
}

/// A cadence that is done after `stop` steps.
struct StopAfter {
    calls: u32,
    stop: u32,
}
impl<R> Cadence<R> for StopAfter {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.calls += 1;
    }
    fn done(&self) -> bool {
        self.calls >= self.stop
    }
}

/// An unequal-mass triangle with momenta summing to zero.
fn ic<R: P>() -> (State<R>, Geom<R>) {
    let m = [1.0, 2.0, 1.5].map(R::of);
    let r = [[0.0, 2.0], [1.5, -0.5], [-2.0, -1.0]].map(|v| v.map(R::of));
    let p = [[0.2, 0.1], [-0.3, 0.2], [0.1, -0.3]].map(|v| v.map(R::of));
    (State { m, r, p }, Geom::new(&m, &r))
}

/// `A`'s answer from `t_now = 3` to `t_target`, at `N_sub = 4`, with a cadence done after `stop` steps.
fn advance<A: Advance, R: P>(t_target: u32, stop: u32) -> (Vec<u64>, u32, u32) {
    let (s0, g0) = ic::<R>();
    let params = AdvanceParams {
        dt_macro: R::of(5e-3),
        n_sub: 4,
    };
    let mut cadence = StopAfter { calls: 0, stop };
    let out = A::advance(&s0, &g0, 3, t_target, &params, &mut cadence);
    let bits = out
        .state
        .r
        .iter()
        .chain(out.state.p.iter())
        .flatten()
        .map(|&x| Into::<f64>::into(x).to_bits())
        .collect();
    (bits, out.substeps, cadence.calls)
}

const SAME: &str = "a stepper under `none` must be the stepper's own ADVANCE";

/// Checks the composed occupant `A` against the stepper `S`: a march stopped early, a whole one, and an empty span.
fn check_same<A: Advance, S: Advance, R: P>(name: &str) {
    for (target, stop) in [(6, 5), (6, u32::MAX), (3, u32::MAX), (4, 1)] {
        let got = advance::<A, R>(target, stop);
        let want = advance::<S, R>(target, stop);
        assert_eq!(
            got, want,
            "{SAME}: {name}, t_target {target}, stop after {stop}"
        );
    }
    // The march moved the state, so the comparison above has something to compare.
    assert_ne!(
        advance::<A, R>(6, u32::MAX).0,
        advance::<A, R>(3, u32::MAX).0,
        "{name}: no motion"
    );
}

fn same_for<S: Stepper, R: P>(name: &str) {
    check_same::<Composed<S, NoRegularisation>, S, R>(name);
}

#[test]
fn qa_m307_none_is_the_stepper() {
    for_precision::<f64>();
    for_precision::<f32>();
}

fn for_precision<R: P>() {
    same_for::<Euler, R>("Euler");
    same_for::<Kdk, R>("KDK");
    same_for::<Yoshida4, R>("Yoshida-4");
    same_for::<Yoshida6, R>("Yoshida-6");
    same_for::<Rk4, R>("RK4");
}

negative_control!(
    qa_m307_none_is_the_stepper,
    "Yoshida-6 under none compared with Yoshida-4",
    expected = "a stepper under `none` must be the stepper's own ADVANCE",
    check_same::<Composed<Yoshida6, NoRegularisation>, Yoshida4, f64>("control")
);
