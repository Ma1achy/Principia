//! QA tests for TASK-M0-35, written from REQ-GEN-003 (dd_generation_root §5 test 1) and REQ-GEN-028 (R-242 as
//! R-247 and R-248 amend it; §3.8 `overflow`), not from the implementation. Each ledger here is built from scratch,
//! and every case goes through the generator driver, which must refuse to emit on a finding. Each test has a
//! registered negative control (R-176).

use ledger::gen;
use ledger::schema::{
    Bound, Consumer, EntryBuilder, FieldType, Ledger, Location, Overflow, Provenance, Range, Scale,
    Span, Word,
};
use validation::negative_control;

const F16_MAX: f64 = 65504.0;

fn word(name: &'static str, bits: u32, reserved: &[(u32, u32)]) -> Word {
    Word {
        name,
        bits,
        reserved: reserved
            .iter()
            .map(|&(offset, width)| Span { offset, width })
            .collect(),
    }
}

fn at(word: &'static str, offset: u32, width: u32) -> Location {
    Location::Packed {
        word,
        offset,
        width,
    }
}

fn closed(lo: f64, hi: f64) -> Range {
    Range {
        lo: Bound::Closed(lo),
        hi: Bound::Closed(hi),
    }
}

fn field(name: &'static str, location: Location, ty: FieldType, range: Range) -> EntryBuilder {
    EntryBuilder::new(name)
        .location(location)
        .ty(ty)
        .scale(Scale::Lin)
        .range(range)
        .provenance(Provenance::Kernel)
        .consumers(&[Consumer::Render, Consumer::Export, Consumer::Debug])
}

fn ubits(name: &'static str, location: Location, hi: u64) -> EntryBuilder {
    field(name, location, FieldType::UBits, closed(0.0, hi as f64))
}

fn ledger(words: Vec<Word>, entries: Vec<EntryBuilder>) -> Ledger {
    Ledger { words, entries }
}

/// Generation succeeds.
fn passes(ledger: &Ledger) {
    if let Err(e) = gen::generate(ledger, gen::EMITTERS) {
        panic!("qa: layout refused: {e}");
    }
}

/// Generation is refused, and one line of the message carries every one of `needles` (the word or entry, the field,
/// the bits, ...).
fn refused(ledger: &Ledger, needles: &[&str]) {
    let message = match gen::generate(ledger, gen::EMITTERS) {
        Ok(_) => panic!("qa: layout not refused"),
        Err(e) => e.to_string(),
    };
    assert!(
        message
            .lines()
            .any(|line| needles.iter().all(|n| line.contains(n))),
        "qa: refused, but no line names all of {needles:?}: {message}"
    );
}

fn vector(component: FieldType, k: u32) -> FieldType {
    FieldType::Vector {
        component: Box::new(component),
        k,
    }
}

// --- REQ-GEN-003: overlap --------------------------------------------------------------------------------------

/// `w` (8 bits): `qa_a` at 0–3, `qa_b` at `b_offset`..+2, `qa_c` at 6–7; bits 4–5 are covered only if `b_offset` is 4.
fn three_fields(b_offset: u32) -> Ledger {
    ledger(
        vec![word("qa_w", 8, &[])],
        vec![
            ubits("qa_a", at("qa_w", 0, 4), 15),
            ubits("qa_b", at("qa_w", b_offset, 2), 3),
            ubits("qa_c", at("qa_w", 6, 2), 3),
        ],
    )
}

#[test]
fn qa_layout_static_overlap_is_refused_naming_word_fields_and_bits() {
    // `qa_b` at 3–4 overlaps `qa_a` at bit 3 (and leaves bit 5 uncovered).
    refused(&three_fields(3), &["qa_w", "qa_a", "qa_b", "3"]);
    // A field over a reserved span overlaps it too.
    let over_reserved = ledger(
        vec![word("qa_r", 8, &[(4, 4)])],
        vec![ubits("qa_d", at("qa_r", 0, 6), 63)],
    );
    refused(&over_reserved, &["qa_r", "qa_d", "4", "5"]);
    // Two fields on exactly the same bits.
    let twins = ledger(
        vec![word("qa_t", 4, &[])],
        vec![
            ubits("qa_t1", at("qa_t", 0, 4), 15),
            ubits("qa_t2", at("qa_t", 0, 4), 15),
        ],
    );
    refused(&twins, &["qa_t", "qa_t1", "qa_t2", "0", "3"]);
}

negative_control!(
    qa_layout_static_overlap_is_refused_naming_word_fields_and_bits,
    "fields at 0–3, 4–5 and 6–7 tile the word exactly, so the refusal check must fail on it",
    expected = "qa: layout not refused",
    refused(&three_fields(4), &["qa_w"])
);

// --- REQ-GEN-003: coverage or reserved -------------------------------------------------------------------------

/// `qa_g` (8 bits): fields at 0–1 and 5–7, and `reserved` over the gap between.
fn gapped(reserved: &[(u32, u32)]) -> Ledger {
    ledger(
        vec![word("qa_g", 8, reserved)],
        vec![
            ubits("qa_lo", at("qa_g", 0, 2), 3),
            ubits("qa_hi", at("qa_g", 5, 3), 7),
        ],
    )
}

#[test]
fn qa_layout_static_uncovered_bits_are_refused_naming_word_and_bits() {
    refused(&gapped(&[]), &["qa_g", "2", "4"]);
    // Reserving only part of the gap leaves the rest uncovered.
    refused(&gapped(&[(2, 2)]), &["qa_g", "4"]);
    // An empty word is all uncovered.
    refused(
        &ledger(vec![word("qa_e", 4, &[])], vec![]),
        &["qa_e", "0", "3"],
    );
}

negative_control!(
    qa_layout_static_uncovered_bits_are_refused_naming_word_and_bits,
    "the gap 2–4 explicitly reserved leaves no bit uncovered, so the refusal check must fail on it",
    expected = "qa: layout not refused",
    refused(&gapped(&[(2, 3)]), &["qa_g"])
);

// --- REQ-GEN-028: u-bits widths ----------------------------------------------------------------------------------

/// A 64-bit word with a u-bits field over `range` at bits 0..width, the rest reserved.
fn ubits_in_64(width: u32, range: Range, overflow: Option<Overflow>) -> Ledger {
    let mut e = field("qa_u", at("qa_64", 0, width), FieldType::UBits, range);
    e.overflow = overflow;
    ledger(vec![word("qa_64", 64, &[(width, 64 - width)])], vec![e])
}

#[test]
fn qa_layout_static_ubits_width_boundary_is_2_pow_w_minus_1() {
    for w in 1..=32u32 {
        let max = (1u64 << w) - 1;
        passes(&ubits_in_64(w, closed(0.0, max as f64), None));
        refused(
            &ubits_in_64(w, closed(0.0, (max + 1) as f64), None),
            &["qa_64", "qa_u", "0", &(w - 1).to_string()],
        );
    }
}

negative_control!(
    qa_layout_static_ubits_width_boundary_is_2_pow_w_minus_1,
    "[0, 2^8 - 1] fits 8 bits, so the refusal check must fail on it",
    expected = "qa: layout not refused",
    refused(&ubits_in_64(8, closed(0.0, 255.0), None), &["qa_u"])
);

#[test]
fn qa_layout_static_ubits_negative_or_unbounded_end_fits_no_width() {
    // Least value below 0.
    refused(
        &ubits_in_64(8, closed(-1.0, 0.0), None),
        &["qa_64", "qa_u", "0", "7"],
    );
    // An unbounded end fits no width, not even 32 bits...
    let up = Range {
        lo: Bound::Closed(0.0),
        hi: Bound::Unbounded,
    };
    let down = Range {
        lo: Bound::Unbounded,
        hi: Bound::Closed(0.0),
    };
    refused(&ubits_in_64(32, up, None), &["qa_64", "qa_u", "0", "31"]);
    refused(&ubits_in_64(32, down, None), &["qa_64", "qa_u", "0", "31"]);
    // ... and `overflow` (an f16-pair key) does not license it on a u-bits field.
    for o in [Overflow::Saturate, Overflow::Inf] {
        refused(&ubits_in_64(32, up, Some(o)), &["qa_64", "qa_u"]);
    }
}

negative_control!(
    qa_layout_static_ubits_negative_or_unbounded_end_fits_no_width,
    "[0, 2^32 - 1] is bounded and fits 32 bits, so the refusal check must fail on it",
    expected = "qa: layout not refused",
    refused(&ubits_in_64(32, closed(0.0, 4294967295.0), None), &["qa_u"])
);

// --- REQ-GEN-028: float types take their exact width, with no range-against-width test ---------------------------

/// A 64-bit word with a field of type `ty` over `range` at bits 0..width, the rest reserved.
fn typed_in_64(ty: FieldType, width: u32, range: Range) -> Ledger {
    let mut e = field("qa_f", at("qa_64", 0, width), ty, range);
    e.overflow = Some(Overflow::Saturate);
    ledger(vec![word("qa_64", 64, &[(width, 64 - width)])], vec![e])
}

fn unbounded() -> Range {
    Range {
        lo: Bound::Unbounded,
        hi: Bound::Unbounded,
    }
}

#[test]
fn qa_layout_static_float_types_take_exactly_their_width() {
    // Ranges no 16- or 32-bit unsigned field could hold: the float types have no range-against-width test.
    let cases = [
        (
            FieldType::F16Pair,
            16,
            closed(-F16_MAX, F16_MAX),
            "f16-pair",
        ),
        (FieldType::Fixed16, 16, closed(-1.0e9, 1.0e9), "fixed16"),
        (FieldType::F32, 32, unbounded(), "f32"),
    ];
    for (ty, exact, range, name) in cases {
        passes(&typed_in_64(ty.clone(), exact, range));
        for width in [1, exact - 1, exact + 1, 2 * exact] {
            refused(
                &typed_in_64(ty.clone(), width, range),
                &["qa_64", "qa_f", name, "0", &(width - 1).to_string()],
            );
        }
    }
    // f16-pair and fixed16 are not interchangeable with f32's 32.
    refused(
        &typed_in_64(FieldType::F32, 16, unbounded()),
        &["qa_f", "f32"],
    );
}

negative_control!(
    qa_layout_static_float_types_take_exactly_their_width,
    "f32 at exactly 32 bits passes whatever its range, so the refusal check must fail on it",
    expected = "qa: layout not refused",
    refused(&typed_in_64(FieldType::F32, 32, unbounded()), &["qa_f"])
);

#[test]
fn qa_layout_static_vector_at_a_packed_location_is_refused_naming_its_type() {
    let components = [
        (FieldType::UBits, 8, closed(0.0, 1.0)),
        (FieldType::F32, 32, closed(-1.0, 1.0)),
        (FieldType::F16Pair, 16, closed(-1.0, 1.0)),
        (FieldType::Fixed16, 16, closed(0.0, 1.0)),
    ];
    for (component, bits, range) in components {
        // k components side by side: the width a packed vector would need if it were allowed.
        let ty = vector(component, 2);
        refused(
            &typed_in_64(ty, 2 * bits, range),
            &["qa_64", "qa_f", "vector"],
        );
    }
}

negative_control!(
    qa_layout_static_vector_at_a_packed_location_is_refused_naming_its_type,
    "a vector at a scalar index is not at a packed location, so the refusal check must fail on it",
    expected = "qa: layout not refused",
    refused(
        &ledger(
            vec![],
            vec![field(
                "qa_vec",
                Location::Scalar(0),
                vector(FieldType::F32, 3),
                closed(-1.0, 1.0)
            )]
        ),
        &["qa_vec"]
    )
);

// --- REQ-GEN-028: an f16-pair range lies within ±65504, wherever it sits -----------------------------------------

/// A ledger holding one field `qa_h` of type `ty` over `range` at `place` (0 packed, 1 scalar index, 2 derived), with
/// `overflow`.
fn f16_at(place: u8, ty: FieldType, range: Range, overflow: Option<Overflow>) -> Ledger {
    let (words, location, mut entries) = match place {
        0 => (vec![word("qa_p", 16, &[])], at("qa_p", 0, 16), vec![]),
        1 => (vec![], Location::Scalar(0), vec![]),
        _ => (
            vec![],
            Location::Derived {
                from: vec!["qa_src"],
            },
            vec![field(
                "qa_src",
                Location::Scalar(0),
                FieldType::F32,
                closed(0.0, 1.0),
            )],
        ),
    };
    let mut e = field("qa_h", location, ty, range);
    e.overflow = overflow;
    entries.push(e);
    ledger(words, entries)
}

fn f16_vec() -> FieldType {
    vector(FieldType::F16Pair, 2)
}

#[test]
fn qa_layout_static_f16_range_beyond_65504_is_refused_wherever_it_sits() {
    for place in 0..3 {
        // f16's finite range itself passes, closed or open.
        passes(&f16_at(
            place,
            FieldType::F16Pair,
            closed(-F16_MAX, F16_MAX),
            None,
        ));
        let open = Range {
            lo: Bound::Open(-F16_MAX),
            hi: Bound::Open(F16_MAX),
        };
        passes(&f16_at(place, FieldType::F16Pair, open, None));
        // Just past either end fails, even with `overflow` stated.
        for range in [
            closed(0.0, 65504.5),
            closed(-65505.0, 0.0),
            closed(1.0e6, 2.0e6),
        ] {
            refused(&f16_at(place, FieldType::F16Pair, range, None), &["qa_h"]);
            refused(
                &f16_at(place, FieldType::F16Pair, range, Some(Overflow::Inf)),
                &["qa_h"],
            );
        }
    }
    // Per component for a vector of f16-pair (packed vectors fail anyway, so scalar and derived).
    for place in 1..3 {
        passes(&f16_at(place, f16_vec(), closed(-F16_MAX, F16_MAX), None));
        refused(
            &f16_at(place, f16_vec(), closed(0.0, 70000.0), None),
            &["qa_h"],
        );
    }
    // The packed finding names the word and the bits.
    refused(
        &f16_at(0, FieldType::F16Pair, closed(0.0, 70000.0), None),
        &["qa_p", "qa_h", "0", "15"],
    );
    // fixed16 is not binary16: no ±65504 rule (R-248).
    passes(&f16_at(0, FieldType::Fixed16, closed(0.0, 1.0e6), None));
}

negative_control!(
    qa_layout_static_f16_range_beyond_65504_is_refused_wherever_it_sits,
    "[-65504, 65504] is f16's finite range, so the refusal check must fail on it",
    expected = "qa: layout not refused",
    refused(
        &f16_at(1, FieldType::F16Pair, closed(-F16_MAX, F16_MAX), None),
        &["qa_h"]
    )
);

#[test]
fn qa_layout_static_f16_unbounded_end_needs_overflow() {
    let ranges = [
        Range {
            lo: Bound::Closed(0.0),
            hi: Bound::Unbounded,
        },
        Range {
            lo: Bound::Unbounded,
            hi: Bound::Closed(0.0),
        },
        unbounded(),
    ];
    for place in 0..3 {
        for range in ranges {
            refused(&f16_at(place, FieldType::F16Pair, range, None), &["qa_h"]);
            for o in [Overflow::Saturate, Overflow::Inf] {
                passes(&f16_at(place, FieldType::F16Pair, range, Some(o)));
                if place > 0 {
                    passes(&f16_at(place, f16_vec(), range, Some(o)));
                }
            }
            if place > 0 {
                refused(&f16_at(place, f16_vec(), range, None), &["qa_h"]);
            }
        }
        // `overflow` licenses the unbounded end, not a finite end beyond ±65504.
        let mixed = Range {
            lo: Bound::Closed(-70000.0),
            hi: Bound::Unbounded,
        };
        refused(
            &f16_at(place, FieldType::F16Pair, mixed, Some(Overflow::Saturate)),
            &["qa_h"],
        );
    }
    refused(
        &f16_at(0, FieldType::F16Pair, unbounded(), None),
        &["qa_p", "qa_h", "0", "15", "overflow"],
    );
}

negative_control!(
    qa_layout_static_f16_unbounded_end_needs_overflow,
    "an unbounded end with `overflow: inf` stated is allowed, so the refusal check must fail on it",
    expected = "qa: layout not refused",
    refused(
        &f16_at(0, FieldType::F16Pair, unbounded(), Some(Overflow::Inf)),
        &["qa_h"]
    )
);

// --- REQ-GEN-028: open ends on a u-bits field --------------------------------------------------------------------

/// A u-bits field holds integers, so an open end is the next integer inside it: `[0, 2^w)` and `(-1, 2^w - 1]` hold
/// exactly `0 ..= 2^w - 1` and fit `w` bits; `[0, 2^w + 1)` and `(-2, 2^w - 1]` do not.
fn open_ubits(w: u32, lo: Bound, hi: Bound) -> Ledger {
    ubits_in_64(w, Range { lo, hi }, None)
}

#[test]
fn qa_layout_static_ubits_open_ends_hold_the_integers_inside() {
    for w in [1u32, 4, 16] {
        let top = (1u64 << w) as f64;
        passes(&open_ubits(w, Bound::Closed(0.0), Bound::Open(top)));
        passes(&open_ubits(w, Bound::Open(-1.0), Bound::Closed(top - 1.0)));
        passes(&open_ubits(w, Bound::Open(-1.0), Bound::Open(top)));
        refused(
            &open_ubits(w, Bound::Closed(0.0), Bound::Open(top + 1.0)),
            &["qa_64", "qa_u"],
        );
        refused(
            &open_ubits(w, Bound::Open(-2.0), Bound::Closed(top - 1.0)),
            &["qa_64", "qa_u"],
        );
    }
}

negative_control!(
    qa_layout_static_ubits_open_ends_hold_the_integers_inside,
    "(-1, 16) holds 0..=15, which fits 4 bits, so the refusal check must fail on it",
    expected = "qa: layout not refused",
    refused(
        &open_ubits(4, Bound::Open(-1.0), Bound::Open(16.0)),
        &["qa_u"]
    )
);

// --- REQ-GEN-003: the overlap names the bits that overlap ---------------------------------------------------------

/// `qa_n` (8 bits): `qa_f` over all of it, and `reserved` spans that it overlaps.
fn over_spans(reserved: &[(u32, u32)]) -> Ledger {
    ledger(
        vec![word("qa_n", 8, reserved)],
        vec![ubits("qa_f", at("qa_n", 0, 8), 255)],
    )
}

/// The bits an overlap line names after its first `bits`: each number `a`, or run `a–b` (inclusive), expanded.
fn named_bits(line: &str) -> Vec<u32> {
    let Some((_, rest)) = line.split_once("bits") else {
        return Vec::new();
    };
    let mut bits = Vec::new();
    let tokens = rest.split(|c: char| !(c.is_ascii_digit() || c == '–' || c == '-'));
    for token in tokens.filter(|t| t.chars().any(|c| c.is_ascii_digit())) {
        let mut ends = token
            .split(['–', '-'])
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<u32>().unwrap());
        if let Some(lo) = ends.next() {
            let hi = ends.next().unwrap_or(lo);
            bits.extend(lo..=hi);
        }
    }
    bits
}

/// Generation is refused for an overlap, and the overlap lines name bits 2 and 6 and no bit between them.
fn overlap_names_only(ledger: &Ledger) {
    let message = match gen::generate(ledger, gen::EMITTERS) {
        Ok(_) => panic!("qa: layout not refused"),
        Err(e) => e.to_string(),
    };
    let mut named: Vec<u32> = message
        .lines()
        .filter(|l| l.contains("overlap"))
        .flat_map(named_bits)
        .collect();
    named.sort_unstable();
    named.dedup();
    assert!(
        named == [2, 6],
        "qa: overlap names bits other than 2 and 6 ({named:?}): {message}"
    );
}

#[test]
fn qa_layout_static_overlap_names_only_the_overlapping_bits() {
    // `qa_f` overlaps the reserved bits 2 and 6 only; bits 3–5 are the field's alone.
    overlap_names_only(&over_spans(&[(2, 1), (6, 1)]));
}

negative_control!(
    qa_layout_static_overlap_names_only_the_overlapping_bits,
    "a reserved span over 2–6 overlaps bits 3–5 too, so the naming check must fail on it",
    expected = "qa: overlap names bits other than 2 and 6",
    overlap_names_only(&over_spans(&[(2, 5)]))
);
