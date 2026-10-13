//! The `diverging` scale agrees with the range (REQ-GEN-033; R-403; dd_generation_root §3.8): an entry carries
//! `scale: diverging` only when its declared range spans zero, `lo < 0 < hi`, either end possibly unbounded, and
//! generation refuses one that does not, naming the field and its range, beside the metadata gate (REQ-GEN-002):
//! - every `diverging` entry of the payload ledger spans zero, and `K_0` and `V_0` are `lin` over `[0, ∞)` and
//!   `(−∞, 0]` (`diverging_range_payload_*`);
//! - the fixture's `diverging` entry over `[0, ∞)`, and over `(−∞, 0]`, is refused, naming it and its range; over
//!   `(−∞, ∞)` it generates (`diverging_range_refuses_*`, `diverging_range_spanning_generates`);
//! - `Range::spans_zero` at its boundary: an end at 0, open or closed, does not span it (`diverging_range_spans_*`).
//!
//! Each test registers its negative control (R-176).

mod support;

use ledger::schema::{Bound, Entry, Ledger, Range, Scale};
use support::{check_generates, check_refused_naming, entry, fixture};
use validation::negative_control;

/// The fixture's `diverging` entry, `fx_f32`, over `range`.
fn diverging_over(range: Range) -> Ledger {
    let mut ledger = fixture();
    let e = entry(&mut ledger, "fx_f32");
    assert_eq!(e.scale, Some(Scale::Diverging), "the fixture's fx_f32");
    *e = e.clone().range(range);
    ledger
}

const NON_NEGATIVE: Range = Range {
    lo: Bound::Closed(0.0),
    hi: Bound::Unbounded,
};

const NON_POSITIVE: Range = Range {
    lo: Bound::Unbounded,
    hi: Bound::Closed(0.0),
};

const UNBOUNDED: Range = Range {
    lo: Bound::Unbounded,
    hi: Bound::Unbounded,
};

#[test]
fn diverging_range_refuses_non_negative() {
    check_refused_naming(
        &diverging_over(NON_NEGATIVE),
        &["`fx_f32`", "`diverging` over [0, ∞)", "R-403"],
    );
}

negative_control!(
    diverging_range_refuses_non_negative,
    "a diverging entry over (−∞, ∞) spans zero and generates",
    expected = "generation was not refused",
    check_refused_naming(&diverging_over(UNBOUNDED), &["`fx_f32`"])
);

#[test]
fn diverging_range_refuses_non_positive() {
    check_refused_naming(
        &diverging_over(NON_POSITIVE),
        &["`fx_f32`", "`diverging` over (−∞, 0]", "R-403"],
    );
}

negative_control!(
    diverging_range_refuses_non_positive,
    "a lin entry over (−∞, 0] is no diverging entry and generates",
    expected = "generation was not refused",
    {
        let mut ledger = diverging_over(NON_POSITIVE);
        let e = entry(&mut ledger, "fx_f32");
        *e = e.clone().scale(Scale::Lin);
        check_refused_naming(&ledger, &["`fx_f32`"]);
    }
);

#[test]
fn diverging_range_spanning_generates() {
    check_generates(&diverging_over(UNBOUNDED));
    check_generates(&diverging_over(Range {
        lo: Bound::Open(-1.0),
        hi: Bound::Closed(2.0),
    }));
}

negative_control!(
    diverging_range_spanning_generates,
    "a diverging entry over [0, ∞) is refused",
    expected = "generation refused",
    check_generates(&diverging_over(NON_NEGATIVE))
);

/// The payload ledger's entries.
fn payload() -> Vec<Entry> {
    ledger::gen::validate(&ledger::payload::ledger()).unwrap_or_else(|e| panic!("{e}"))
}

/// Checks that every `diverging` entry of `entries` spans zero, that they are the four R-403 names, and that `K_0`
/// and `V_0` are `lin` over `[0, ∞)` and `(−∞, 0]`.
fn check_payload(entries: &[Entry]) {
    let diverging: Vec<&str> = entries
        .iter()
        .filter(|e| e.scale == Scale::Diverging)
        .map(|e| {
            assert!(
                e.range.spans_zero(),
                "`{}` is diverging over {}, which does not span zero",
                e.name,
                e.range
            );
            e.name
        })
        .collect();
    assert_eq!(
        diverging,
        ["E_0", "Lz_0", "energy_drift", "Lz_drift"],
        "the diverging entries"
    );
    for (field, range) in [("K_0", NON_NEGATIVE), ("V_0", NON_POSITIVE)] {
        let e = entries
            .iter()
            .find(|e| e.name == field)
            .unwrap_or_else(|| panic!("no `{field}`"));
        assert!(
            e.scale == Scale::Lin && e.range == range,
            "`{field}` is {:?} over {}, not lin over {range}",
            e.scale,
            e.range
        );
    }
}

#[test]
fn diverging_range_payload_spans_zero_and_energies_are_lin() {
    check_payload(&payload());
}

negative_control!(
    diverging_range_payload_spans_zero_and_energies_are_lin,
    "the ledger before R-403, `V_0` diverging over (−∞, ∞)",
    expected = "the diverging entries",
    {
        let mut entries = payload();
        let v = entries.iter_mut().find(|e| e.name == "V_0").expect("V_0");
        v.scale = Scale::Diverging;
        v.range = UNBOUNDED;
        check_payload(&entries);
    }
);

/// Checks `spans` against `Range::spans_zero` at the boundary cases.
fn check_spans(spans: fn(&Range) -> bool) {
    use Bound::{Closed, Open, Unbounded};
    let cases = [
        (Unbounded, Unbounded, true),
        (Closed(-1.0), Closed(1.0), true),
        (Open(-1e-30), Open(1e-30), true),
        (Unbounded, Open(1e-30), true),
        (Open(-1e-30), Unbounded, true),
        (Closed(0.0), Unbounded, false),
        (Open(0.0), Unbounded, false),
        (Unbounded, Closed(0.0), false),
        (Unbounded, Open(0.0), false),
        (Closed(0.0), Closed(0.0), false),
        (Closed(1.0), Closed(2.0), false),
        (Closed(-2.0), Closed(-1.0), false),
    ];
    for (lo, hi, want) in cases {
        let range = Range { lo, hi };
        let got = spans(&range);
        assert_eq!(got, want, "{range}: spans_zero gives {got}, not {want}");
    }
}

#[test]
fn diverging_range_spans_zero_at_its_boundary() {
    check_spans(Range::spans_zero);
}

negative_control!(
    diverging_range_spans_zero_at_its_boundary,
    "a rule taking `lo ≤ 0 ≤ hi` counts [0, ∞) as spanning zero",
    expected = "[0, ∞): spans_zero gives true",
    check_spans(|r| {
        let lo = match r.lo {
            Bound::Closed(v) | Bound::Open(v) => v <= 0.0,
            Bound::Unbounded => true,
        };
        let hi = match r.hi {
            Bound::Closed(v) | Bound::Open(v) => v >= 0.0,
            Bound::Unbounded => true,
        };
        lo && hi
    })
);

/// Checks that each range writes as `want`, the interval notation a refusal names.
fn check_written(cases: &[(Range, &str)]) {
    for (range, want) in cases {
        assert_eq!(range.to_string(), *want, "{range:?} is not written {want}");
    }
}

#[test]
fn diverging_range_spans_zero_names_its_range() {
    check_written(&[
        (NON_NEGATIVE, "[0, ∞)"),
        (NON_POSITIVE, "(−∞, 0]"),
        (UNBOUNDED, "(−∞, ∞)"),
        (
            Range {
                lo: Bound::Open(-1.5),
                hi: Bound::Open(2.0),
            },
            "(-1.5, 2)",
        ),
    ]);
}

negative_control!(
    diverging_range_spans_zero_names_its_range,
    "an open end written as closed",
    expected = "is not written",
    check_written(&[(
        Range {
            lo: Bound::Open(0.0),
            hi: Bound::Unbounded
        },
        "[0, ∞)"
    )])
);
