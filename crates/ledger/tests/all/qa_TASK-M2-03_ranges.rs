//! QA tests for the ledger ranges TASK-M2-03's definitions declare (REQ-PAY-088, dd_decoder §3.6; dd_generation_root
//! §3.8 "a field whose range is still a gap is listed as one, with the requirement that fills it"), written from §3.6,
//! not from the implementation:
//! - `rho_mag` and `lambda_mag` are the mass-weighted magnitudes with `rho_mag² + lambda_mag² = I = 1`, "each lies in
//!   `[0, 1]`": each is declared exactly [0, 1], citing §3.6 and REQ-PAY-088, a range that does not span zero, so its
//!   view is on `ramp_viridis` over the fixed [0, 1] (R-400) (`qa_m2_03_magnitudes_declared`);
//! - `rho_angle` is "in radians on `(−π, π]`": declared exactly that, citing §3.6 and REQ-PAY-088, no longer a gap,
//!   and still cyclic, drawn on `ramp_twilight` with the period 2π (render_gui_spec §10.1; REQ-TOOL-161)
//!   (`qa_m2_03_rho_angle_declared_exactly`).
//!
//! Each test has its negative control (R-176).

use ledger::gen::numeric::{cyclic_period, End, NumericView, Placement, Ramp};
use ledger::payload;
use ledger::schema::{Bound, Declaration, Entry, Ledger, Range, RangeState, Scale};
use validation::negative_control;

fn validated(l: &Ledger) -> Vec<Entry> {
    ledger::gen::validate(l).unwrap_or_else(|e| panic!("{e}"))
}

fn entry<'a>(es: &'a [Entry], name: &str) -> &'a Entry {
    es.iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no payload field `{name}`"))
}

/// `name`'s declared range and citation, or a panic naming its state.
fn declared(es: &[Entry], name: &str) -> (Range, &'static str) {
    match entry(es, name).range_state() {
        RangeState::Declared { range, citation } => (range, citation),
        other => panic!("`{name}`'s range is not declared: {other:?}"),
    }
}

fn check_cites(name: &str, citation: &str) {
    for p in ["§3.6", "REQ-PAY-088"] {
        assert!(
            citation.contains(p),
            "`{name}`'s declaration does not cite {p}: {citation}"
        );
    }
}

/// The payload ledger with `name`'s range rewritten to `range`, its citation kept.
#[cfg(feature = "controls")]
fn with_range(name: &str, range: Range) -> Ledger {
    let mut l = payload::ledger();
    let e = l
        .entries
        .iter_mut()
        .find(|e| e.name == Some(name))
        .unwrap_or_else(|| panic!("no ledger entry `{name}`"));
    e.range = Some(range);
    l
}

// ── rho_mag and lambda_mag: [0, 1] ───────────────────────────────────────────────────────────────────────────────

const UNIT: Range = Range {
    lo: Bound::Closed(0.0),
    hi: Bound::Closed(1.0),
};

fn check_magnitudes(es: &[Entry]) {
    for name in ["rho_mag", "lambda_mag"] {
        let (range, citation) = declared(es, name);
        assert_eq!(
            range, UNIT,
            "`{name}`'s declared range is not dd_decoder §3.6's [0, 1]"
        );
        check_cites(name, citation);
        let e = entry(es, name);
        assert!(
            matches!(e.declaration, Some(Declaration::Cited(_))) || e.declaration.is_none(),
            "`{name}` carries a provisional mark: {:?}",
            e.declaration
        );
        assert!(!payload::spans_zero(e), "`{name}`'s range spans zero");
        let v = NumericView::of(e).unwrap_or_else(|| panic!("`{name}` has no numeric view"));
        assert_eq!(v.ramp(), Ramp::Viridis, "`{name}` is not on ramp_viridis");
        assert_eq!(
            (v.lo, v.hi),
            (End::Fixed(0.0), End::Fixed(1.0)),
            "`{name}`'s view is not on the fixed [0, 1]"
        );
    }
}

#[test]
fn qa_m2_03_magnitudes_declared() {
    check_magnitudes(&validated(&payload::ledger()));
}

negative_control!(
    qa_m2_03_magnitudes_declared,
    "lambda_mag back on [0, ∞), its lower bound only",
    expected = "`lambda_mag`'s declared range is not dd_decoder §3.6's [0, 1]",
    check_magnitudes(&validated(&with_range(
        "lambda_mag",
        Range {
            lo: Bound::Closed(0.0),
            hi: Bound::Unbounded
        }
    )))
);

// ── rho_angle: (−π, π], cyclic ───────────────────────────────────────────────────────────────────────────────────

fn check_rho_angle_exact(es: &[Entry]) {
    let pi = std::f64::consts::PI;
    let (range, citation) = declared(es, "rho_angle");
    assert_eq!(
        range,
        Range {
            lo: Bound::Open(-pi),
            hi: Bound::Closed(pi),
        },
        "rho_angle's declared range is not dd_decoder §3.6's (−π, π]"
    );
    check_cites("rho_angle", citation);
    assert!(
        !payload::GAPS.iter().any(|g| g.0 == "rho_angle"),
        "rho_angle is declared but still listed a gap"
    );
    let e = entry(es, "rho_angle");
    assert_eq!(e.scale, Scale::Cyclic, "rho_angle is no longer cyclic");
    let v = NumericView::of(e).expect("rho_angle has a numeric view");
    assert_eq!(
        v.ramp(),
        Ramp::Twilight,
        "rho_angle is not on ramp_twilight"
    );
    assert_eq!(
        v.placement,
        Placement::Cyclic {
            period: cyclic_period()
        },
        "rho_angle's view is not cyclic"
    );
    assert_eq!(
        cyclic_period(),
        std::f64::consts::TAU,
        "the period is not 2π"
    );
}

#[test]
fn qa_m2_03_rho_angle_declared_exactly() {
    check_rho_angle_exact(&validated(&payload::ledger()));
}

negative_control!(
    qa_m2_03_rho_angle_declared_exactly,
    "rho_angle declared [−π, π], which includes −π",
    expected = "rho_angle's declared range is not dd_decoder §3.6's (−π, π]",
    check_rho_angle_exact(&validated(&with_range(
        "rho_angle",
        Range {
            lo: Bound::Closed(-std::f64::consts::PI),
            hi: Bound::Closed(std::f64::consts::PI)
        }
    )))
);
