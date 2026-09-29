//! The static layout check (dd_generation_root §5 test 1, §3.8): in each packed word an overlapping pair, an
//! undeclared uncovered bit and a width too narrow for its range each fail generation, naming the word and the bits;
//! the fixture passes (REQ-GEN-003). The width rules (REQ-GEN-028; R-242 as R-247 and R-248 amend it): a u-bits field
//! holds its range, and an unbounded end fits no width; `f16-pair` and `fixed16` take exactly 16 bits and `f32`
//! exactly 32; a vector at a packed location fails; an `f16-pair` range, wherever it sits, lies within ±65504, and an
//! unbounded end needs `overflow`.

mod support;

use ledger::schema::{Bound, FieldType, Ledger, Overflow, Range};
use support::{check_generates, check_refused_naming, entry, fixture, packed};
use validation::negative_control;
use Bound::{Closed, Unbounded};

/// The fixture with `field` moved to bits `offset .. offset + width` of `word`.
fn moved(field: &str, word: &'static str, offset: u32, width: u32) -> Ledger {
    let mut ledger = fixture();
    entry(&mut ledger, field).location = Some(packed(word, offset, width));
    ledger
}

/// The fixture with `fx_word`'s reserved span cut to `width` bits from bit 6.
fn reserving(width: u32) -> Ledger {
    let mut ledger = fixture();
    ledger.words[0].reserved[0].width = width;
    ledger
}

/// The fixture with `field`'s type set to `ty`.
fn typed(field: &str, ty: FieldType) -> Ledger {
    let mut ledger = fixture();
    entry(&mut ledger, field).ty = Some(ty);
    ledger
}

/// The fixture with `field`'s range set to `lo .. hi`, and its `overflow` to `overflow`.
fn ranged(field: &str, lo: Bound, hi: Bound, overflow: Option<Overflow>) -> Ledger {
    let mut ledger = fixture();
    let e = entry(&mut ledger, field);
    e.range = Some(Range { lo, hi });
    e.overflow = overflow;
    ledger
}

/// `fx_vector` as `vector(f16-pair, 3)`, over `lo .. hi` per component, with `overflow`.
fn f16_vector(lo: Bound, hi: Bound, overflow: Option<Overflow>) -> Ledger {
    let mut ledger = ranged("fx_vector", lo, hi, overflow);
    entry(&mut ledger, "fx_vector").ty = Some(FieldType::Vector {
        component: Box::new(FieldType::F16Pair),
        k: 3,
    });
    ledger
}

fn vector(component: FieldType, k: u32) -> FieldType {
    FieldType::Vector {
        component: Box::new(component),
        k,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-003: disjointness and coverage.
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn layout_static_fixture_passes() {
    check_generates(&fixture());
}

negative_control!(
    layout_static_fixture_passes,
    "an overlapping pair fails the layout, so the passes check must fail on it",
    expected = "generation refused",
    check_generates(&moved("fx_detail", "fx_word", 2, 2))
);

#[test]
fn layout_static_overlap_names_word_and_bits() {
    // `fx_detail` moved from 3–4 to 2–3 overlaps `fx_enum` (0–2) at bit 2, and leaves bit 4 uncovered.
    check_refused_naming(
        &moved("fx_detail", "fx_word", 2, 2),
        &["word `fx_word`: `fx_enum` and `fx_detail` overlap at bits 2–2"],
    );
    check_refused_naming(
        &moved("fx_flag", "fx_word", 6, 1),
        &["word `fx_word`: `reserved` and `fx_flag` overlap at bits 6–6"],
    );
}

negative_control!(
    layout_static_overlap_names_word_and_bits,
    "the fixture's own layout has no overlap, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(&moved("fx_detail", "fx_word", 3, 2), &["overlap"])
);

#[test]
fn layout_static_uncovered_bits_name_word_and_bits() {
    check_refused_naming(
        &reserving(7),
        &["word `fx_word`: bits 13–15 are neither covered by a field nor reserved"],
    );
}

negative_control!(
    layout_static_uncovered_bits_name_word_and_bits,
    "the full reserved span covers every bit, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(&reserving(10), &["neither covered"])
);

#[test]
fn layout_static_outside_or_undeclared_word_names_the_field() {
    check_refused_naming(
        &moved("fx_u16", "fx_times", 8, 16),
        &["word `fx_times`: `fx_u16` at bits 8–23 reaches past the word's 16 declared bits"],
    );
    check_refused_naming(
        &moved("fx_flag", "fx_nowhere", 0, 1),
        &["field `fx_flag` is placed in undeclared word `fx_nowhere`"],
    );
}

negative_control!(
    layout_static_outside_or_undeclared_word_names_the_field,
    "bits 0–15 lie inside `fx_times`, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(&moved("fx_u16", "fx_times", 0, 16), &["reaches past"])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-GEN-028: widths against ranges.
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn layout_static_too_narrow_names_word_and_bits() {
    let narrow = "word `fx_word`: `fx_enum` at bits 0–2 is 3 bit(s) wide, too narrow for its range";
    let cases = [
        (Closed(0.0), Closed(8.0), "[0, 8]"),
        (Closed(-1.0), Closed(5.0), "[-1, 5]"),
        (Closed(0.0), Unbounded, "[0, inf)"),
        (Unbounded, Closed(5.0), "(-inf, 5]"),
    ];
    for (lo, hi, range) in cases {
        let ledger = ranged("fx_enum", lo, hi, None);
        check_refused_naming(&ledger, &[&format!("{narrow} {range}")]);
    }
}

negative_control!(
    layout_static_too_narrow_names_word_and_bits,
    "[0, 7] fits 3 bits, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        &ranged("fx_enum", Closed(0.0), Closed(7.0), None),
        &["too narrow"]
    )
);

#[test]
fn layout_static_float_types_take_their_exact_width() {
    let cases = [
        (
            "fx_enum",
            FieldType::Fixed16,
            "`fx_enum` at bits 0–2 is fixed16, which takes exactly 16 bits, not 3",
        ),
        (
            "fx_flag",
            FieldType::F16Pair,
            "`fx_flag` at bits 5–5 is f16-pair, which takes exactly 16 bits, not 1",
        ),
        (
            "fx_u16",
            FieldType::F32,
            "`fx_u16` at bits 0–15 is f32, which takes exactly 32 bits, not 16",
        ),
        (
            "fx_f32",
            FieldType::F16Pair,
            "`fx_f32` at bits 0–31 is f16-pair, which takes exactly 16 bits, not 32",
        ),
    ];
    for (field, ty, message) in cases {
        check_refused_naming(&typed(field, ty), &[message]);
    }
}

negative_control!(
    layout_static_float_types_take_their_exact_width,
    "fixed16 at 16 bits is exact and has no range test, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(&typed("fx_u16", FieldType::Fixed16), &["takes exactly"])
);

#[test]
fn layout_static_vector_at_a_packed_location_fails_naming_its_type() {
    check_refused_naming(
        &typed("fx_u16", vector(FieldType::UBits, 2)),
        &["word `fx_times`: `fx_u16` at bits 0–15 has type vector(u-bits, 2), which the width check does not accept"],
    );
}

negative_control!(
    layout_static_vector_at_a_packed_location_fails_naming_its_type,
    "a vector at a scalar index is not at a packed location, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        &typed("fx_vector", vector(FieldType::UBits, 2)),
        &["does not accept"]
    )
);

#[test]
fn layout_static_f16_range_beyond_65504_fails() {
    let beyond = "f16-pair field `fx_half` has range (0, 70000], beyond f16's finite range ±65504";
    check_refused_naming(
        &ranged("fx_half", Bound::Open(0.0), Closed(70000.0), None),
        &[&format!("word `fx_pair`, bits 0–15: {beyond}")],
    );
    check_refused_naming(
        &f16_vector(Closed(-65505.0), Closed(1.0), None),
        &["entry `fx_vector`: f16-pair field `fx_vector` has range [-65505, 1], beyond"],
    );
}

negative_control!(
    layout_static_f16_range_beyond_65504_fails,
    "±65504 is f16's finite range itself, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        &f16_vector(Closed(-65504.0), Closed(65504.0), None),
        &["beyond"]
    )
);

#[test]
fn layout_static_f16_unbounded_end_without_overflow_fails() {
    check_refused_naming(
        &ranged("fx_half", Bound::Open(0.0), Unbounded, None),
        &["word `fx_pair`, bits 0–15: f16-pair field `fx_half` has an unbounded end in its range (0, inf) and states no `overflow`"],
    );
    check_refused_naming(
        &f16_vector(Unbounded, Closed(1.0), None),
        &["entry `fx_vector`: f16-pair field `fx_vector` has an unbounded end"],
    );
}

negative_control!(
    layout_static_f16_unbounded_end_without_overflow_fails,
    "an unbounded end with `overflow` is allowed, so the refusal check must fail on it",
    expected = "generation was not refused",
    check_refused_naming(
        &ranged(
            "fx_half",
            Bound::Open(0.0),
            Unbounded,
            Some(Overflow::Saturate)
        ),
        &["unbounded end"]
    )
);

#[test]
fn layout_static_f16_unbounded_end_with_overflow_passes() {
    for overflow in [Overflow::Saturate, Overflow::Inf] {
        check_generates(&ranged(
            "fx_half",
            Bound::Open(0.0),
            Unbounded,
            Some(overflow),
        ));
        check_generates(&f16_vector(Unbounded, Unbounded, Some(overflow)));
    }
}

negative_control!(
    layout_static_f16_unbounded_end_with_overflow_passes,
    "without `overflow` an unbounded end fails, so the passes check must fail on it",
    expected = "generation refused",
    check_generates(&ranged("fx_half", Bound::Open(0.0), Unbounded, None))
);
