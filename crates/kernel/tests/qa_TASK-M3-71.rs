//! QA tests for TASK-M3-71, written from REQ-EVT-035 and the definition it asks for (integrator contract Part 7,
//! "The floor's record, defined"; R-954, R-973), not from the implementation:
//! - A pair's floor at a state is the f32 spacing at the pair's largest coordinate, each position cast to f32 first:
//!   `2^c`, `c = clamp(e − 23, −40, 104)`, `e` the largest exponent field of the pair's four coordinates, stored as
//!   the code `c + 41`. Pair `k` is the side opposite body `k` (R-22). The floor varies with the pair's own
//!   coordinates, not the third body's.
//! - The record is the largest code over every pair near firing at every state the test reads, until it fires
//!   (`d_quant < r_coll + floor`, strictly, in squared form), and over the `d_min` state, the state and pair the latch
//!   holds when the sample is written (so a later, closer `d_min` state replaces the earlier one's code).
//! - The bring-up mode's f32 writer writes the record's two words, `coll_floor_bits` and `_pad`, as 0 (code 0, "not
//!   recorded"; colour_composition Appendix A's row), so a reused buffer keeps no stale word.
//!
//! Every figure below is exact in f32: the separations are whole multiples of the spacing at their coordinates, and
//! `r_coll` is a multiple of a power of two whose square f32 holds exactly.
//!
//! Each test has its negative controls (R-176): a floor or a record built by a rule the definition rules out.

use kernel::detect::collision::{floor_codes, CollisionRadius, Detector};
use validation::negative_control;

type Pos = [[f64; 2]; 3];

/// `2^e`, exact.
fn p2(e: i32) -> f64 {
    2f64.powi(e)
}

// ---------------------------------------------------------------------------------------------------------------
// A pair's floor.
// ---------------------------------------------------------------------------------------------------------------

/// Each pair's floor code at the positions `r`, by R-22 pair id.
type Codes = fn(&Pos) -> [u32; 3];

/// The cases, each with its expected codes worked by hand from the definition.
fn code_cases() -> Vec<(&'static str, Pos, [u32; 3])> {
    vec![
        // Every coordinate in [1, 2): the spacing 2⁻²³, the code −23 + 41 = 18.
        (
            "all in [1, 2)",
            [[1.5, -1.25], [1.0, 1.75], [-1.9, 1.1]],
            [18, 18, 18],
        ),
        // Bodies 0 and 1 in [0.5, 1) (2⁻²⁴, 17), body 2 at 3.0 (2⁻²², 19): pair 2, (0, 1), keeps 17.
        (
            "the pair's own coordinates",
            [[0.75, -0.5], [-0.625, 0.875], [3.0, -0.5]],
            [19, 19, 17],
        ),
        // 1 − 2⁻⁴⁰ rounds to 1.0 in f32 (e = 0, code 18), though its f64 exponent is −1 (code 17).
        (
            "rounded to f32 first",
            [[1.0 - p2(-40), 0.5], [0.5, -0.75], [0.0, 0.5]],
            [17, 18, 18],
        ),
        // The negative coordinates by |x|: pair 0 (1, 2) at 0.4 (e = −2, 16); the others at 3.5 (e = 1, 19).
        (
            "by |x|",
            [[-3.5, -0.1], [0.25, -0.2], [-0.3, 0.4]],
            [16, 19, 19],
        ),
        // Raised to 2⁻⁴⁰: 0 and 2⁻³⁰ give code 1; 2⁻¹⁶ gives c = −39, code 2.
        (
            "raised to 2^-40",
            [[p2(-16), 0.0], [0.0, 0.0], [p2(-30), 0.0]],
            [1, 2, 2],
        ),
        ("all at the origin", [[0.0; 2]; 3], [1, 1, 1]),
        // Capped at 2¹⁰⁴: f32::MAX (e = 127) gives code 145; 2¹⁰⁰ gives c = 77, code 118.
        (
            "capped at 2^104",
            [[f64::from(f32::MAX), 0.0], [0.0, 0.0], [p2(100), 0.0]],
            [118, 145, 145],
        ),
    ]
}

fn check_codes(codes: Codes) {
    for (name, r, want) in code_cases() {
        let got = codes(&r);
        assert_eq!(
            got, want,
            "{name}: the pair floors are {got:?}, not {want:?}"
        );
    }
}

/// A floor code from an exponent `e`, as the definition forms it.
#[cfg(feature = "controls")]
fn code_of(e: i32) -> u32 {
    ((e - 23).clamp(-40, 104) + 41) as u32
}

/// The exponent field of an f64, unrounded (a wrong reading: the test casts to f32 first).
#[cfg(feature = "controls")]
fn f64_exponent(x: f64) -> i32 {
    ((x.to_bits() >> 52) & 0x7ff) as i32 - 1023
}

/// The exponent field of `x` cast to f32.
#[cfg(feature = "controls")]
fn f32_exponent(x: f64) -> i32 {
    (((x as f32).to_bits() >> 23) & 0xff) as i32 - 127
}

/// Pair floors from per-body exponents by `e_of`, then `code`, pair `k` opposite body `k`.
#[cfg(feature = "controls")]
fn pair_codes(r: &Pos, e_of: fn(f64) -> i32, code: fn(i32) -> u32) -> [u32; 3] {
    let e = |b: usize| e_of(r[b][0]).max(e_of(r[b][1]));
    [
        code(e(1).max(e(2))),
        code(e(2).max(e(0))),
        code(e(0).max(e(1))),
    ]
}

#[test]
fn qa_floor_is_each_pairs_f32_spacing() {
    check_codes(floor_codes::<f64>);
}

negative_control!(
    qa_floor_is_each_pairs_f32_spacing,
    "a floor read from the f64 coordinates, not cast to f32",
    expected = "rounded to f32 first: the pair floors",
    check_codes(|r| pair_codes(r, f64_exponent, code_of))
);

negative_control!(
    qa_floor_is_each_pairs_f32_spacing_all_bodies,
    "a floor taken over all three bodies, not the pair's own coordinates",
    expected = "the pair's own coordinates: the pair floors",
    check_codes(|r| {
        let e = r.iter().flatten().map(|&x| f32_exponent(x)).max().unwrap();
        [code_of(e); 3]
    })
);

negative_control!(
    qa_floor_is_each_pairs_f32_spacing_unclamped,
    "a floor not raised to 2^-40",
    expected = "raised to 2^-40: the pair floors",
    check_codes(|r| pair_codes(r, f32_exponent, |e| (e - 23 + 41).clamp(0, 145) as u32))
);

// ---------------------------------------------------------------------------------------------------------------
// The record over a run.
// ---------------------------------------------------------------------------------------------------------------

/// A record of the run under `r_coll` over the states `s` (the IC first): the code the sample would store.
type Recorder = fn(f64, &[Pos]) -> u32;

/// The detector's record, as the wrapper writes it.
fn detector(r_coll: f64, s: &[Pos]) -> Detector {
    let mut d = Detector::new(CollisionRadius::from_r_coll(r_coll), &s[0]);
    for r in &s[1..] {
        d.observe(r);
    }
    d
}

fn recorded(r_coll: f64, s: &[Pos]) -> u32 {
    detector(r_coll, s).floor_code()
}

/// A scenario: its name, `r_coll`, its states, the record expected after each prefix, and whether it has collided.
struct Scenario {
    name: &'static str,
    r_coll: f64,
    states: Vec<Pos>,
    want: Vec<u32>,
    fired: bool,
}

fn scenarios() -> Vec<Scenario> {
    // Body 2 stays apart from the pair (0, 1), never near firing and never the d_min pair.
    let b2 = [-0.875, -0.75];
    vec![
        // The d_min state counts with nothing near firing: the IC's pair at x ≈ 3 (19), then a farther state at
        // [0.5, 1) (the latch stays), then a closer one at [0.5, 1) (the latch moves, so the record is its 17).
        Scenario {
            name: "dmin_state",
            r_coll: 1e-3,
            states: vec![
                [[3.0, 0.0], [3.5, 0.0], [-1.5, 2.0]],
                [[0.75, 0.0], [-0.75, 0.0], [0.0, 0.875]],
                [[0.75, 0.5], [0.5, 0.5], b2],
            ],
            want: vec![19, 19, 17],
            fired: false,
        },
        // Near firing raises it above the d_min state: r_coll = 15·2⁻²⁴. At x ≈ 3 (floor 2⁻²²) the pair at
        // 16·2⁻²⁴ is not firing (16 > 15) but near (16 < 15 + 4); then at x ≈ 0.75 (floor 2⁻²⁴) the pair at
        // 15·2⁻²⁴ moves the latch (code 17) and does not fire (d² = r_coll², not below). The record keeps 19.
        Scenario {
            name: "near",
            r_coll: 15.0 * p2(-24),
            states: vec![
                [[3.0, 0.0], [3.0 + 16.0 * p2(-24), 0.0], b2],
                [[0.75, 0.0], [0.75 + 15.0 * p2(-24), 0.0], b2],
            ],
            want: vec![19, 19],
            fired: false,
        },
        // Read until the test fires: at x ≈ 0.75 the pair at 4·2⁻²⁴ fires (17); then at x ≈ 12 (floor 2⁻²⁰) a pair
        // at 2⁻²⁰ would be near (code 21), but the test no longer reads, and the latch stays at the closer state.
        Scenario {
            name: "after_collision",
            r_coll: 15.0 * p2(-24),
            states: vec![
                [[0.75, 0.0], [0.75 + 4.0 * p2(-24), 0.0], b2],
                [[12.0, 0.0], [12.0 + p2(-20), 0.0], b2],
            ],
            want: vec![17, 17],
            fired: true,
        },
        // Strictly below `r_coll + floor`: r_coll = 2⁻²¹ at x ≈ 3 (floor 2⁻²²), the pair at 3·2⁻²² = r_coll +
        // floor exactly, not near; then the latch moves to x ≈ 0.75 at 9·2⁻²⁴ (above r_coll = 8·2⁻²⁴, not near:
        // 9 = 8 + 1). The record is the d_min state's 17.
        Scenario {
            name: "boundary",
            r_coll: p2(-21),
            states: vec![
                [[3.0, 0.0], [3.0 + 3.0 * p2(-22), 0.0], b2],
                [[0.75, 0.0], [0.75 + 9.0 * p2(-24), 0.0], b2],
            ],
            want: vec![19, 17],
            fired: false,
        },
    ]
}

fn check_record(record: Recorder) {
    for sc in scenarios() {
        // The premise: the scenario collides, or not, as built.
        assert_eq!(
            detector(sc.r_coll, &sc.states).fired(),
            sc.fired,
            "the scenario {} is not built as described",
            sc.name
        );
        for (k, &want) in sc.want.iter().enumerate() {
            let got = record(sc.r_coll, &sc.states[..=k]);
            assert_eq!(
                got,
                want,
                "{}: the record after {} states is {got}, not {want}",
                sc.name,
                k + 1
            );
        }
    }
}

#[test]
fn qa_record_is_the_largest_near_firing_and_d_min_state_floor() {
    check_record(recorded);
}

negative_control!(
    qa_record_is_the_largest_near_firing_and_d_min_state_floor,
    "a record of the d_min state alone",
    expected = "near: the record",
    check_record(|r, s| detector(r, s).latched_code())
);

negative_control!(
    qa_record_is_the_largest_near_firing_and_d_min_state_floor_near_only,
    "a record of the near-firing states alone",
    expected = "dmin_state: the record",
    check_record(|r, s| detector(r, s).near_code())
);

negative_control!(
    qa_record_is_the_largest_near_firing_and_d_min_state_floor_every_latch,
    "a record that keeps every d_min state ever latched",
    expected = "dmin_state: the record",
    check_record(|r, s| (1..=s.len()).map(|k| recorded(r, &s[..k])).max().unwrap())
);

negative_control!(
    qa_record_is_the_largest_near_firing_and_d_min_state_floor_after_firing,
    "a record that reads near firing after the test fired",
    expected = "after_collision: the record",
    check_record(|r, s| {
        let near = s
            .iter()
            .map(|st| detector(r, std::slice::from_ref(st)).near_code())
            .max()
            .unwrap();
        near.max(detector(r, s).latched_code())
    })
);

negative_control!(
    qa_record_is_the_largest_near_firing_and_d_min_state_floor_inclusive,
    "a near-firing test that counts d = r_coll + floor",
    expected = "boundary: the record",
    check_record(|r, s| recorded(r * (1.0 + p2(-10)), s))
);

// ---------------------------------------------------------------------------------------------------------------
// The bring-up mode's f32 writer: the record's two words (colour_composition Appendix A's `coll_floor_bits`, `_pad`
// row; payload §2, code 0 "not recorded", reserved bits 8–31 zero).
// ---------------------------------------------------------------------------------------------------------------

/// The word index, within a sample, of `SimStateFTLE`'s `coll_floor_bits` and of its `_pad`.
fn record_words() -> [(&'static str, usize); 2] {
    use kernel::payload::SimStateFTLE;
    use std::mem::offset_of;
    [
        (
            "coll_floor_bits",
            offset_of!(SimStateFTLE, coll_floor_bits) / 4,
        ),
        ("_pad", offset_of!(SimStateFTLE, _pad) / 4),
    ]
}

/// Sample `i`, written by `write` into a buffer of `i + 2` samples filled with `0xA5A5A5A5` (a reused buffer's stale
/// words): Appendix A's row says both words are 0, so neither may keep the stale word.
fn check_record_words(write: fn(u32, &mut [u32])) {
    let n = kernel::bringup::words_per_sample();
    for i in [0u32, 1, 7, 65537] {
        let mut buf = vec![0xA5A5_A5A5u32; (i as usize + 2) * n];
        write(i, &mut buf);
        for (name, w) in record_words() {
            let got = buf[i as usize * n + w];
            assert!(
                got == 0,
                "sample {i}: `{name}` is {got:#x}, not Appendix A's 0"
            );
        }
    }
}

#[test]
fn qa_bringup_writes_the_records_two_words() {
    check_record_words(kernel::bringup::write_words);
}

/// Sample `i`'s word `w` of `out`, its word index in the buffer.
#[cfg(feature = "controls")]
fn word(i: u32, w: usize) -> usize {
    i as usize * kernel::bringup::words_per_sample() + w
}

/// The writer with the record's word `skip` left as the buffer held it, as before the fix.
#[cfg(feature = "controls")]
fn skipping(skip: usize, i: u32, out: &mut [u32]) {
    let at = word(i, record_words()[skip].1);
    let kept = out[at];
    kernel::bringup::write_words(i, out);
    out[at] = kept;
}

#[cfg(feature = "controls")]
mod skips_floor {
    use super::*;
    negative_control!(
        qa_bringup_writes_the_records_two_words,
        "a writer that leaves coll_floor_bits stale",
        expected = "`coll_floor_bits` is 0xa5a5a5a5",
        check_record_words(|i, out| skipping(0, i, out))
    );
}

#[cfg(feature = "controls")]
mod skips_pad {
    use super::*;
    negative_control!(
        qa_bringup_writes_the_records_two_words,
        "a writer that leaves _pad stale",
        expected = "`_pad` is 0xa5a5a5a5",
        check_record_words(|i, out| skipping(1, i, out))
    );
}

#[cfg(feature = "controls")]
mod nonzero_code {
    use super::*;
    negative_control!(
        qa_bringup_writes_the_records_two_words,
        "a writer that records a code (18) for the pattern",
        expected = "`coll_floor_bits` is 0x12",
        check_record_words(|i, out| {
            kernel::bringup::write_words(i, out);
            out[word(i, record_words()[0].1)] = 18;
        })
    );
}
