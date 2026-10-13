//! The declared lower bounds (REQ-RENDER-084; R-400; TASK-M1-17): a numeric view whose field's declared range spans
//! zero is drawn on the diverging ramp, so each field the corpus or its own definition makes non-negative declares
//! `lo ≥ 0`, citing the line that does (`ledger::payload::NON_NEGATIVE`): the mass fractions and `|ρ|` of
//! render_gui_spec §10.1, `virial_ratio = 2K₀/|V₀|` and `K₀` of dd_decoder §3.6, and the latches and log fields of
//! dd_generation_root §3.4 and §3.6 (`declared_ranges_*`). The unsigned bit fields are non-negative by their encoding
//! and need no citation.
//!
//! Each test registers its negative control (R-176).

use ledger::payload::NON_NEGATIVE;
use ledger::schema::{Bound, Entry, FieldType};
use validation::negative_control;

/// The payload ledger's entries.
fn payload() -> Vec<Entry> {
    ledger::gen::validate(&ledger::payload::ledger()).unwrap_or_else(|e| panic!("{e}"))
}

/// Whether `e` declares a lower bound of at least 0.
fn non_negative(e: &Entry) -> bool {
    matches!(e.range.lo, Bound::Closed(v) | Bound::Open(v) if v >= 0.0)
}

/// Checks that each of `cited` names a ledger field of `entries` declaring `lo ≥ 0`, with a citation naming a
/// section or a ruling; that every other field of `entries` declaring `lo ≥ 0` is unsigned bits; and that the fields
/// the task names are among them.
fn check_declared(entries: &[Entry], cited: &[(&str, &str)]) {
    for (field, citation) in cited {
        let e = entries
            .iter()
            .find(|e| e.name == *field)
            .unwrap_or_else(|| panic!("`{field}` is cited but no ledger field"));
        assert!(
            non_negative(e),
            "`{field}` is non-negative ({citation}) but declares {}",
            e.range
        );
        assert!(
            citation.contains('§') || citation.contains("R-"),
            "`{field}`'s citation names no line: `{citation}`"
        );
    }
    for e in entries.iter().filter(|e| non_negative(e)) {
        assert!(
            e.ty == FieldType::UBits || cited.iter().any(|(f, _)| *f == e.name),
            "`{}` declares {} with no citation",
            e.name,
            e.range
        );
    }
    for field in [
        "m0",
        "m1",
        "m2",
        "rho_mag",
        "lambda_mag",
        "virial_ratio",
        "K_0",
    ] {
        assert!(
            cited.iter().any(|(f, _)| *f == field),
            "`{field}` is not declared non-negative"
        );
    }
}

#[test]
fn declared_ranges_non_negative_fields_cite_their_lines() {
    let entries = payload();
    check_declared(&entries, &NON_NEGATIVE);
    for (field, citation) in NON_NEGATIVE {
        let e = entries.iter().find(|e| e.name == field).expect("cited");
        println!("{field}: {} — {citation}", e.range);
    }
}

negative_control!(
    declared_ranges_non_negative_fields_cite_their_lines,
    "`virial_ratio` left on the unbounded default reads as signed",
    expected = "`virial_ratio` is non-negative",
    {
        let mut entries = payload();
        let v = entries
            .iter_mut()
            .find(|e| e.name == "virial_ratio")
            .expect("virial_ratio");
        v.range.lo = Bound::Unbounded;
        check_declared(&entries, &NON_NEGATIVE);
    }
);

#[test]
fn declared_ranges_every_bound_is_cited() {
    let cited: Vec<(&str, &str)> = NON_NEGATIVE.to_vec();
    check_declared(&payload(), &cited);
}

negative_control!(
    declared_ranges_every_bound_is_cited,
    "`K_0`'s bound with its citation dropped",
    expected = "`K_0` declares [0, ∞) with no citation",
    {
        let cited: Vec<(&str, &str)> = NON_NEGATIVE
            .iter()
            .copied()
            .filter(|(f, _)| *f != "K_0")
            .collect();
        check_declared(&payload(), &cited);
    }
);

/// The masses declare `[0, 1]`, the corpus's mass fraction; the declared bound makes their range fully bounded, so
/// their views default to the fixed range, `RANGE_AUTO = 0` (render_gui_spec §10.1).
#[test]
fn declared_ranges_masses_are_fractions() {
    check_fractions(&payload());
}

/// Checks that `m0`, `m1` and `m2` declare `[0, 1]` and default to the fixed range.
fn check_fractions(entries: &[Entry]) {
    for field in ["m0", "m1", "m2"] {
        let e = entries.iter().find(|e| e.name == field).expect("a mass");
        assert_eq!(
            e.range.to_string(),
            "[0, 1]",
            "`{field}` is not a mass fraction"
        );
        let n = ledger::gen::numeric::NumericView::of(e).expect("a numeric view");
        assert!(
            !n.range_auto,
            "`{field}` does not default to its fixed range"
        );
    }
}

negative_control!(
    declared_ranges_masses_are_fractions,
    "a mass declared [0, ∞) defaults to auto",
    expected = "`m1` is not a mass fraction",
    {
        let mut entries = payload();
        let m = entries.iter_mut().find(|e| e.name == "m1").expect("m1");
        m.range.hi = Bound::Unbounded;
        check_fractions(&entries);
    }
);
