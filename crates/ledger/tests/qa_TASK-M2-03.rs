//! QA tests for TASK-M2-03's ledger side, written from the requirements, not from the implementation:
//! - REQ-PAY-097 (R-426, R-400): `q_mass`'s range is declared from its formula, `min(m₀, m₁)/max(m₀, m₁)` (REQ-PAY-088,
//!   dd_decoder §3.6), which lies in [0, 1]; the declaration cites the formula and REQ-PAY-088; its provisional mark is
//!   gone; the range does not span zero, so its numeric view is on `ramp_viridis`; and its changed golden's
//!   `BASELINES.md` row cites R-426.
//! - REQ-PAY-088 (dd_generation_root §3.8: "a field whose range is still a gap is listed as one, with the requirement
//!   that fills it"): `rho_angle`'s gap is listed as filled by REQ-PAY-088, and REQ-PAY-088, which this task closes,
//!   defines `rho_angle ∈ (−π, π]` (dd_decoder §3.6); so once it is defined the range is no longer a gap, and the
//!   ledger writes it within [−π, π].
//!
//! Each test has its negative control (R-176).

use ledger::gen::numeric::{NumericView, Ramp};
use ledger::payload;
use ledger::schema::{Bound, Declaration, Entry, Ledger, Range, RangeState};
use validation::negative_control;

fn validated(l: &Ledger) -> Vec<Entry> {
    ledger::gen::validate(l).unwrap_or_else(|e| panic!("{e}"))
}

fn entry<'a>(es: &'a [Entry], name: &str) -> &'a Entry {
    es.iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no payload field `{name}`"))
}

// ── REQ-PAY-097: q_mass declared [0, 1] from its formula ──────────────────────────────────────────────────────────

fn check_q_mass(es: &[Entry]) {
    let e = entry(es, "q_mass");
    match e.range_state() {
        RangeState::Declared { range, citation } => {
            assert_eq!(
                range,
                Range {
                    lo: Bound::Closed(0.0),
                    hi: Bound::Closed(1.0),
                },
                "q_mass's declared range is not [0, 1]"
            );
            assert!(
                citation.contains("min(m₀, m₁)/max(m₀, m₁)") && citation.contains("REQ-PAY-088"),
                "q_mass's declaration does not cite its formula and REQ-PAY-088: {citation}"
            );
        }
        other => panic!("q_mass's range is not declared: {other:?}"),
    }
    assert!(
        matches!(e.declaration, Some(Declaration::Cited(_))),
        "q_mass's mark is not a citation: {:?}",
        e.declaration
    );
    assert!(
        !payload::PROVISIONAL.iter().any(|(n, _)| *n == "q_mass"),
        "q_mass is still listed provisional"
    );
    assert!(!payload::spans_zero(e), "q_mass's range spans zero");
    let v = NumericView::of(e).expect("q_mass has a numeric view");
    assert_eq!(
        v.ramp(),
        Ramp::Viridis,
        "q_mass's view is not on ramp_viridis"
    );
}

#[test]
fn qa_m2_03_q_mass_declared() {
    check_q_mass(&validated(&payload::ledger()));
}

negative_control!(
    qa_m2_03_q_mass_declared,
    "q_mass back in its M1 state, provisional over (−∞, ∞)",
    expected = "q_mass's range is not declared",
    {
        let mut l = payload::ledger();
        let e = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("q_mass"))
            .unwrap();
        e.range = Some(Range {
            lo: Bound::Unbounded,
            hi: Bound::Unbounded,
        });
        e.declaration = Some(Declaration::Provisional("REQ-PAY-097"));
        check_q_mass(&validated(&l));
    }
);

/// The `BASELINES.md` row of `case` cites `ruling`.
fn check_row(baselines: &str, case: &str, ruling: &str) {
    let row = baselines
        .lines()
        .find(|l| l.starts_with(&format!("| `{case}` |")))
        .unwrap_or_else(|| panic!("BASELINES.md has no row for `{case}`"));
    assert!(
        row.contains(ruling),
        "`{case}`'s row does not cite {ruling}"
    );
}

fn baselines() -> String {
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/BASELINES.md");
    std::fs::read_to_string(p).unwrap()
}

#[test]
fn qa_m2_03_q_mass_baseline_row() {
    check_row(&baselines(), "debug-views/generated-q_mass", "R-426");
}

negative_control!(
    qa_m2_03_q_mass_baseline_row,
    "a row that does not cite R-426",
    expected = "does not cite R-426",
    check_row(&baselines(), "debug-views/generated-r", "R-426")
);

// ── REQ-PAY-088: rho_angle's range, once its formula is defined ───────────────────────────────────────────────────

fn check_rho_angle(es: &[Entry]) {
    let e = entry(es, "rho_angle");
    let gap = payload::GAPS.iter().find(|g| g.0 == "rho_angle");
    match e.range_state() {
        RangeState::Declared { range, .. } => {
            let pi = std::f64::consts::PI;
            let lo_ok = matches!(range.lo, Bound::Closed(v) | Bound::Open(v) if v >= -pi);
            let hi_ok = matches!(range.hi, Bound::Closed(v) | Bound::Open(v) if v <= pi);
            assert!(
                lo_ok && hi_ok,
                "rho_angle's declared range {range} is not within dd_decoder §3.6's (−π, π]"
            );
            assert!(gap.is_none(), "rho_angle is declared but still listed a gap");
        }
        other => panic!(
            "rho_angle's range is {other:?}, listed as filled by {:?}, though REQ-PAY-088 now defines it on (−π, π] \
             (dd_decoder §3.6)",
            gap.and_then(|g| g.1)
        ),
    }
}

#[test]
fn qa_m2_03_rho_angle_declared() {
    check_rho_angle(&validated(&payload::ledger()));
}

negative_control!(
    qa_m2_03_rho_angle_declared,
    "rho_angle declared over (−∞, ∞)",
    expected = "is not within dd_decoder §3.6's (−π, π]",
    {
        let mut l = payload::ledger();
        let e = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("rho_angle"))
            .unwrap();
        e.range = Some(Range {
            lo: Bound::Closed(-10.0),
            hi: Bound::Closed(10.0),
        });
        e.declaration = Some(Declaration::Cited("qa"));
        check_rho_angle(&validated(&l));
    }
);
