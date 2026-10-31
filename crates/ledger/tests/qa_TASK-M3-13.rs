//! QA's tests for TASK-M3-13's ledger side, REQ-PAY-098 (R-426, R-400; dd_generation_root §3.8; dd_simstate_payload
//! §4), written from the requirement, not from the implementation:
//! - `qa_m3_13_mean_y_declared`: `mean_y`'s range is declared, `[0, 4]` closed at both ends (the range of payload §4's
//!   `y`, a squared chord between two unit vectors, and so of its mean), its citation naming the definition
//!   (REQ-PAY-073, payload §4); it is not provisional, and no payload field is provisional any longer (R-426's two,
//!   `q_mass` and `mean_y`, are both declared).
//! - `qa_m3_13_mean_y_view_ramp`: a declared range that does not span zero puts the numeric view on `ramp_viridis`
//!   (R-400), and the rule is the range's, not the field's name: the same entry with a zero-spanning declared range is on
//!   `ramp_coolwarm`.
//!
//! Each test has its negative control (R-176).

use ledger::gen::numeric::{NumericView, Ramp};
use ledger::payload;
use ledger::schema::{Bound, Declaration, Entry, Ledger, Range, RangeState};
use validation::negative_control;

fn validated(l: &Ledger) -> Vec<Entry> {
    ledger::gen::validate(l).unwrap_or_else(|e| panic!("{e}"))
}

fn mean_y(es: &[Entry]) -> &Entry {
    es.iter()
        .find(|e| e.name == "mean_y")
        .expect("no payload field `mean_y`")
}

/// The payload ledger with `mean_y`'s range and mark replaced.
fn with_mean_y(range: Range, mark: Option<Declaration>) -> Ledger {
    let mut l = payload::ledger();
    let e = l
        .entries
        .iter_mut()
        .find(|e| e.name == Some("mean_y"))
        .expect("no ledger entry `mean_y`");
    e.range = Some(range);
    e.declaration = mark;
    l
}

/// Checks `mean_y`'s declaration and that no field is provisional.
fn check_declared(es: &[Entry]) {
    match mean_y(es).range_state() {
        RangeState::Declared { range, citation } => {
            assert_eq!(
                (range.lo, range.hi),
                (Bound::Closed(0.0), Bound::Closed(4.0)),
                "`mean_y`'s declared range is not [0, 4]"
            );
            assert!(
                citation.contains("REQ-PAY-073") && citation.contains("§4"),
                "`mean_y`'s citation does not name y's definition (REQ-PAY-073, payload §4): {citation}"
            );
        }
        other => panic!("`mean_y`'s range is not declared: {other:?}"),
    }
    for e in es {
        assert!(
            !matches!(e.range_state(), RangeState::Provisional { .. }),
            "`{}` is still provisional",
            e.name
        );
    }
    assert!(
        payload::PROVISIONAL.is_empty(),
        "the provisional table is not empty"
    );
}

#[test]
fn qa_m3_13_mean_y_declared() {
    check_declared(&validated(&payload::ledger()));
}

negative_control!(
    qa_m3_13_mean_y_declared,
    "`mean_y` as it was before REQ-PAY-098, provisional over (−∞, ∞), is not declared",
    expected = "`mean_y`'s range is not declared",
    check_declared(&validated(&with_mean_y(
        Range {
            lo: Bound::Unbounded,
            hi: Bound::Unbounded,
        },
        Some(Declaration::Provisional("REQ-PAY-098"))
    )))
);

fn check_ramp(es: &[Entry], want: Ramp) {
    let v = NumericView::of(mean_y(es)).expect("`mean_y` has no numeric view");
    assert_eq!(v.ramp(), want, "`mean_y`'s view ramp");
}

#[test]
fn qa_m3_13_mean_y_view_ramp() {
    check_ramp(&validated(&payload::ledger()), Ramp::Viridis);
    // The rule is the declared range's (R-400): the same field over a cited range that spans zero is on cool-warm.
    check_ramp(
        &validated(&with_mean_y(
            Range {
                lo: Bound::Closed(-4.0),
                hi: Bound::Closed(4.0),
            },
            Some(Declaration::Cited(
                "qa: a zero-spanning range, dd_simstate_payload §4",
            )),
        )),
        Ramp::Coolwarm,
    );
}

negative_control!(
    qa_m3_13_mean_y_view_ramp,
    "the payload's `mean_y` is not on ramp_coolwarm",
    expected = "`mean_y`'s view ramp",
    check_ramp(&validated(&payload::ledger()), Ramp::Coolwarm)
);
