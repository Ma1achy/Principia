//! QA's tests for TASK-M1-21 (REQ-GEN-035; R-426; dd_generation_root §3.8 "The three range states"), written from the
//! requirement's statement and verify detail, not from the implementation:
//! - a `lin` or `diverging` scalar field whose range is undeclared and not marked provisional fails generation, naming
//!   it: an existing payload field retagged, and a fresh fixture field added to the payload ledger
//!   (`qa_m1_21_gap_*`);
//! - an undeclared range never spans zero, so it never puts a field on `ramp_coolwarm` (`qa_m1_21_gap_not_coolwarm*`);
//! - `q_mass` and `mean_y` are provisional, naming REQ-PAY-097 and REQ-PAY-098, and keep `ramp_coolwarm`
//!   (`qa_m1_21_provisional`);
//! - `S`, `ftle`, `diffusion`, `C_ty`, `E_0`, `Lz_0`, `energy_drift` and `Lz_drift` are declared (−∞, ∞) with the
//!   citation the requirement names (`qa_m1_21_declared_citations`);
//! - the gap list is exactly the payload's gaps, none a `lin` or `diverging` scalar, each with a requirement or none
//!   (`qa_m1_21_gap_list`);
//! - no view changes ramp: the cool-warm set is R-423's six fields less `theta` (cyclic), with `C_ty` and the
//!   `diverging` `E_0` and `Lz_0` (`qa_m1_21_coolwarm_set_unchanged`);
//! - an entry's range is in one of §3.8's three states, by its own mark (`qa_m1_21_state_*`): unmarked (−∞, ∞) is a
//!   gap; other written bounds are not; a cited (−∞, ∞) is declared with its citation and spans zero; a provisional
//!   (−∞, ∞) is provisional with its requirement and spans zero; a provisional mark on other bounds is no gap;
//! - a fresh field cited or marked provisional generates and is drawn on `ramp_coolwarm`
//!   (`qa_m1_21_marked_fresh_*`);
//! - a gap never lets a field carry the `diverging` scale, a vector included (`qa_m1_21_gap_diverging_vector_refused`);
//! - every `lin` or `diverging` scalar the numeric template draws cites a source, or is provisional
//!   (`qa_m1_21_drawn_fields_cite_a_source`).
//!
//! Each test has its negative control (R-176).

use std::collections::BTreeSet;

use ledger::gen::numeric::{NumericView, Ramp};
use ledger::payload::{self, GAPS};
use ledger::schema::{
    Bound, Declaration, Entry, EntryBuilder, FieldType, Ledger, Location, Range, RangeState, Scale,
};
use validation::negative_control;

/// The payload ledger's validated entries.
fn payload() -> Vec<Entry> {
    ledger::gen::validate(&payload::ledger()).unwrap_or_else(|e| panic!("{e}"))
}

fn entry<'a>(es: &'a [Entry], name: &str) -> &'a Entry {
    es.iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no payload field `{name}`"))
}

fn state(es: &[Entry], name: &str) -> RangeState {
    payload::range_state(entry(es, name)).unwrap_or_else(|| panic!("`{name}` has no range state"))
}

// ── A gap on a `lin` or `diverging` scalar fails generation, naming it ─────────────────────────────────────────────

/// The payload ledger with `name` given `scale` and the range written where none is declared, (−∞, ∞).
fn retagged(name: &str, scale: Scale) -> Ledger {
    let mut l = payload::ledger();
    let e = l
        .entries
        .iter_mut()
        .find(|e| e.name == Some(name))
        .unwrap_or_else(|| panic!("no ledger entry `{name}`"));
    e.scale = Some(scale);
    e.range = Some(Range {
        lo: Bound::Unbounded,
        hi: Bound::Unbounded,
    });
    l
}

/// The payload ledger plus a fresh scalar `qa_fixture`, derived from `S`, with `scale` over `range` and nothing citing
/// it: §3 gives it no range, so its range is undeclared.
fn with_fixture(scale: Scale, range: Range) -> Ledger {
    let mut l = payload::ledger();
    l.entries.push(
        EntryBuilder::new("qa_fixture")
            .location(Location::Derived { from: vec!["S"] })
            .ty(FieldType::F32)
            .scale(scale)
            .range(range)
            .provenance(ledger::schema::Provenance::Kernel)
            .consumers(&[ledger::schema::Consumer::Render]),
    );
    l
}

const UNBOUNDED: Range = Range {
    lo: Bound::Unbounded,
    hi: Bound::Unbounded,
};

/// Generation of `l` fails with a line naming `field`.
fn check_refused(l: &Ledger, field: &str) {
    let err = match ledger::gen::validate(l) {
        Ok(_) => panic!("generation was not refused for the undeclared `{field}`"),
        Err(e) => e.to_string(),
    };
    assert!(
        err.lines().any(|line| line.contains(&format!("`{field}`"))),
        "the refusal does not name `{field}`: {err}"
    );
}

#[test]
fn qa_m1_21_gap_existing_field_refused() {
    // `theta` (cyclic) and `rho_angle` (cyclic) have no declared range; as `lin` or `diverging` scalars their gap
    // would choose the ramp, so generation must refuse them.
    check_refused(&retagged("theta", Scale::Lin), "theta");
    check_refused(&retagged("rho_angle", Scale::Lin), "rho_angle");
    check_refused(&retagged("theta", Scale::Diverging), "theta");
}

negative_control!(
    qa_m1_21_gap_existing_field_refused,
    "a field with a declared range, `S`, retagged the same way is not refused",
    expected = "generation was not refused for the undeclared `S`",
    check_refused(&retagged("S", Scale::Lin), "S")
);

#[test]
fn qa_m1_21_gap_fresh_lin_field_refused() {
    // REQ-GEN-035's control: "a fixture lin entry with an undeclared range and no provisional mark fails generation
    // naming it". A field new to the ledger, cited nowhere, is undeclared whatever its name.
    check_refused(&with_fixture(Scale::Lin, UNBOUNDED), "qa_fixture");
}

negative_control!(
    qa_m1_21_gap_fresh_lin_field_refused,
    "the fixture with a one-signed `log` scale generates",
    expected = "generation was not refused for the undeclared `qa_fixture`",
    check_refused(
        &with_fixture(
            Scale::Log,
            Range {
                lo: Bound::Open(0.0),
                hi: Bound::Unbounded
            }
        ),
        "qa_fixture"
    )
);

#[test]
fn qa_m1_21_gap_fresh_diverging_field_refused() {
    check_refused(&with_fixture(Scale::Diverging, UNBOUNDED), "qa_fixture");
}

negative_control!(
    qa_m1_21_gap_fresh_diverging_field_refused,
    "the fixture as `cyclic`, whose view reads no range, generates",
    expected = "generation was not refused for the undeclared `qa_fixture`",
    check_refused(&with_fixture(Scale::Cyclic, UNBOUNDED), "qa_fixture")
);

// ── An undeclared range never spans zero, so never puts a field on cool-warm ───────────────────────────────────────

/// `name`'s entry in `l`, as validated, and the ramp its numeric view would draw on, generation's gate set aside.
fn ramp_of(l: &Ledger, name: &str) -> Option<Ramp> {
    let b = l
        .entries
        .iter()
        .find(|e| e.name == Some(name))
        .unwrap_or_else(|| panic!("no ledger entry `{name}`"))
        .clone();
    let e = b.build().unwrap_or_else(|e| panic!("{e}"));
    NumericView::of(&e).map(|v| v.ramp())
}

fn check_not_coolwarm(l: &Ledger, name: &str) {
    assert_ne!(
        ramp_of(l, name),
        Some(Ramp::Coolwarm),
        "the undeclared `{name}` is placed on ramp_coolwarm"
    );
}

#[test]
fn qa_m1_21_gap_not_coolwarm_existing() {
    check_not_coolwarm(&retagged("theta", Scale::Lin), "theta");
    check_not_coolwarm(&retagged("rho_angle", Scale::Lin), "rho_angle");
}

negative_control!(
    qa_m1_21_gap_not_coolwarm_existing,
    "`S`, declared (−∞, ∞), is on ramp_coolwarm",
    expected = "the undeclared `S` is placed on ramp_coolwarm",
    check_not_coolwarm(&retagged("S", Scale::Lin), "S")
);

#[test]
fn qa_m1_21_gap_not_coolwarm_fresh() {
    check_not_coolwarm(&with_fixture(Scale::Lin, UNBOUNDED), "qa_fixture");
}

negative_control!(
    qa_m1_21_gap_not_coolwarm_fresh,
    "a provisional field, `q_mass`, is on ramp_coolwarm",
    expected = "the undeclared `q_mass` is placed on ramp_coolwarm",
    check_not_coolwarm(&payload::ledger(), "q_mass")
);

#[test]
fn qa_m1_21_gap_state_never_spans_zero() {
    assert!(!RangeState::Gap.spans_zero(), "a gap spans zero");
    let es = payload();
    for (name, ..) in GAPS {
        assert!(
            !payload::spans_zero(entry(&es, name)),
            "the gap `{name}` spans zero"
        );
    }
}

negative_control!(
    qa_m1_21_gap_state_never_spans_zero,
    "a provisional range spans zero",
    expected = "q_mass spans zero",
    assert!(
        !payload::spans_zero(entry(&payload(), "q_mass")),
        "q_mass spans zero"
    )
);

// ── q_mass and mean_y: provisional, on cool-warm ───────────────────────────────────────────────────────────────────

fn check_provisional(es: &[Entry], want: &[(&str, &str)]) {
    for (name, requirement) in want {
        match state(es, name) {
            RangeState::Provisional { requirement: r } => assert_eq!(
                r, *requirement,
                "`{name}` is provisional until {r}, not {requirement}"
            ),
            other => panic!("`{name}` is not provisional: {other:?}"),
        }
        let v = NumericView::of(entry(es, name)).expect("a numeric view");
        assert_eq!(v.ramp(), Ramp::Coolwarm, "`{name}` left ramp_coolwarm");
    }
}

#[test]
fn qa_m1_21_provisional() {
    check_provisional(
        &payload(),
        &[("q_mass", "REQ-PAY-097"), ("mean_y", "REQ-PAY-098")],
    );
}

negative_control!(
    qa_m1_21_provisional,
    "`S`, declared, is not provisional",
    expected = "`S` is not provisional",
    check_provisional(&payload(), &[("S", "REQ-PAY-097")])
);

// ── The declared fields and their citations ───────────────────────────────────────────────────────────────────────

/// REQ-GEN-035's declarations: each field, declared (−∞, ∞), and what its citation must name.
const SIGNED: [(&str, &[&str]); 8] = [
    ("S", &["R-423"]),
    ("ftle", &["R-423"]),
    ("diffusion", &["R-423"]),
    ("C_ty", &["R-423", "C_tt(n)", "diffusion"]),
    ("E_0", &["§3.4", "diverging"]),
    ("Lz_0", &["§3.4", "diverging"]),
    ("energy_drift", &["§3.8", "§3.4"]),
    ("Lz_drift", &["§3.8", "§3.4"]),
];

fn check_declared(es: &[Entry], want: &[(&str, &[&str])]) {
    for (name, phrases) in want {
        let RangeState::Declared { range, citation } = state(es, name) else {
            panic!("`{name}` is not declared");
        };
        assert_eq!(range, UNBOUNDED, "`{name}` is not declared (−∞, ∞)");
        assert_eq!(
            entry(es, name).range,
            UNBOUNDED,
            "`{name}` writes another range"
        );
        for p in *phrases {
            assert!(
                citation.contains(p),
                "`{name}`'s citation does not name {p}: {citation}"
            );
        }
    }
}

#[test]
fn qa_m1_21_declared_citations() {
    let es = payload();
    check_declared(&es, &SIGNED);
    // The drifts generate though floored; the rest are drawn on cool-warm, signed.
    for (name, _) in SIGNED {
        let e = entry(&es, name);
        assert!(payload::spans_zero(e), "`{name}` does not span zero");
        if e.floor.is_none() {
            assert_eq!(
                NumericView::of(e).map(|v| v.ramp()),
                Some(Ramp::Coolwarm),
                "`{name}` is not on ramp_coolwarm"
            );
        }
    }
}

negative_control!(
    qa_m1_21_declared_citations,
    "`theta`, a gap, is not declared",
    expected = "`theta` is not declared",
    check_declared(&payload(), &[("theta", &["R-423"])])
);

// ── The gap list ──────────────────────────────────────────────────────────────────────────────────────────────────

/// Checks that `gaps` names exactly the payload's gaps, none a `lin` or `diverging` scalar, each with a REQ id or none,
/// and the examples the task names among them.
fn check_gap_list(es: &[Entry], gaps: &[(&str, Option<&str>, &str)]) {
    let listed: BTreeSet<&str> = gaps.iter().map(|g| g.0).collect();
    let actual: BTreeSet<&str> = es
        .iter()
        .filter(|e| payload::range_state(e) == Some(RangeState::Gap))
        .map(|e| e.name)
        .collect();
    assert_eq!(listed, actual, "the gap list is not the ledger's gaps");
    for (name, req, _) in gaps {
        let e = entry(es, name);
        let ramp_scale = matches!(e.scale, Scale::Lin | Scale::Diverging);
        let scalar = !matches!(e.ty, FieldType::Vector { .. });
        assert!(
            !(ramp_scale && scalar),
            "the gap `{name}` is a lin or diverging scalar"
        );
        if let Some(r) = req {
            assert!(
                r.starts_with("REQ-"),
                "`{name}`'s filler is no requirement: {r}"
            );
        }
    }
    for name in ["theta", "rho_angle", "r", "p", "r_sh", "p_sh"] {
        assert!(listed.contains(name), "`{name}` is not listed as a gap");
    }
}

#[test]
fn qa_m1_21_gap_list() {
    check_gap_list(&payload(), &GAPS);
    for (name, req, why) in GAPS {
        println!("gap `{name}`: filled by {} — {why}", req.unwrap_or("none"));
    }
}

negative_control!(
    qa_m1_21_gap_list,
    "the gap list with `theta` dropped",
    expected = "the gap list is not the ledger's gaps",
    {
        let gaps: Vec<_> = GAPS.iter().copied().filter(|g| g.0 != "theta").collect();
        check_gap_list(&payload(), &gaps);
    }
);

// ── No view changes ramp ─────────────────────────────────────────────────────────────────────────────────────────

/// The views on cool-warm before R-426: R-423's six less `theta` (now cyclic), `C_ty` (RQ-277), and the `diverging`
/// `E_0` and `Lz_0`.
const COOLWARM: [&str; 8] = [
    "q_mass",
    "ftle",
    "mean_y",
    "S",
    "diffusion",
    "C_ty",
    "E_0",
    "Lz_0",
];

fn check_coolwarm(es: &[Entry], want: &[&str]) {
    let got: BTreeSet<&str> = es
        .iter()
        .filter(|e| NumericView::of(e).map(|v| v.ramp()) == Some(Ramp::Coolwarm))
        .map(|e| e.name)
        .collect();
    let want: BTreeSet<&str> = want.iter().copied().collect();
    assert_eq!(got, want, "the cool-warm views changed");
}

#[test]
fn qa_m1_21_coolwarm_set_unchanged() {
    check_coolwarm(&payload(), &COOLWARM);
}

negative_control!(
    qa_m1_21_coolwarm_set_unchanged,
    "`mean_y` taken off the expected set",
    expected = "the cool-warm views changed",
    check_coolwarm(&payload(), &COOLWARM[..2])
);

// ── The three range states, by the entry's own mark (§3.8; R-426) ────────────────────────────────────────────────

/// A complete scalar `qa_state` entry, `lin` over `range`, with the range mark `mark`.
fn marked(range: Range, mark: Option<Declaration>) -> Entry {
    let mut b = EntryBuilder::new("qa_state")
        .location(Location::Scalar(0))
        .ty(FieldType::F32)
        .scale(Scale::Lin)
        .range(range)
        .provenance(ledger::schema::Provenance::Kernel)
        .consumers(&[ledger::schema::Consumer::Render]);
    b.declaration = mark;
    b.build().unwrap_or_else(|e| panic!("{e}"))
}

const QA_CITATION: &str = "qa fixture: dd_generation_root §3.8 `declared(lo, hi, citation)`";
const QA_REQUIREMENT: &str = "REQ-QA-FIXTURE";

fn from_zero() -> Range {
    Range {
        lo: Bound::Closed(0.0),
        hi: Bound::Unbounded,
    }
}

fn signed_finite() -> Range {
    Range {
        lo: Bound::Open(-1.0),
        hi: Bound::Closed(1.0),
    }
}

fn check_gap(e: &Entry) {
    assert_eq!(e.range_state(), RangeState::Gap, "the range is not a gap");
    assert!(!e.range_state().spans_zero(), "the gap spans zero");
}

#[test]
fn qa_m1_21_state_unmarked_unbounded_is_gap() {
    // §3.8 `gap`: "no range is declared; the (−∞, ∞) the ledger once wrote by default. A gap never spans zero."
    check_gap(&marked(UNBOUNDED, None));
}

negative_control!(
    qa_m1_21_state_unmarked_unbounded_is_gap,
    "the same (−∞, ∞) with a citation is declared, not a gap",
    expected = "the range is not a gap",
    check_gap(&marked(UNBOUNDED, Some(Declaration::Cited(QA_CITATION))))
);

/// An unmarked entry over `range` is no gap, and spans zero exactly when `range` does (`lo < 0 < hi`).
fn check_written_not_gap(range: Range, spans: bool) {
    let state = marked(range, None).range_state();
    match state {
        RangeState::Declared { range: r, .. } => {
            assert_eq!(r, range, "the declared range is not the written one")
        }
        other => panic!("the written range {range} is not declared: {other:?}"),
    }
    assert_eq!(
        state.spans_zero(),
        spans,
        "{range} spans zero: {}",
        state.spans_zero()
    );
}

#[test]
fn qa_m1_21_state_unmarked_written_bounds_are_no_gap() {
    // Only the (−∞, ∞) written by default declares nothing (§3.8 `gap`); a range written with a bound is not that.
    check_written_not_gap(from_zero(), false);
    check_written_not_gap(signed_finite(), true);
    check_written_not_gap(
        Range {
            lo: Bound::Unbounded,
            hi: Bound::Closed(0.0),
        },
        false,
    );
}

negative_control!(
    qa_m1_21_state_unmarked_written_bounds_are_no_gap,
    "the unmarked (−∞, ∞) is a gap, not declared",
    expected = "is not declared",
    check_written_not_gap(UNBOUNDED, true)
);

fn check_cited(e: &Entry) {
    match e.range_state() {
        RangeState::Declared { range, citation } => {
            assert_eq!(range, UNBOUNDED, "the cited range is not (−∞, ∞)");
            assert_eq!(citation, QA_CITATION, "the citation is not the one given");
        }
        other => panic!("the cited (−∞, ∞) is not declared: {other:?}"),
    }
    assert!(
        e.range_state().spans_zero(),
        "a declared (−∞, ∞) does not span zero"
    );
}

#[test]
fn qa_m1_21_state_cited_unbounded_is_declared() {
    // §3.8 `declared(lo, hi, citation)`: "Only a declared range spans zero or not"; (−∞, ∞) does (R-400).
    check_cited(&marked(UNBOUNDED, Some(Declaration::Cited(QA_CITATION))));
}

negative_control!(
    qa_m1_21_state_cited_unbounded_is_declared,
    "the same (−∞, ∞) uncited is a gap",
    expected = "the cited (−∞, ∞) is not declared",
    check_cited(&marked(UNBOUNDED, None))
);

fn check_provisional_state(e: &Entry) {
    match e.range_state() {
        RangeState::Provisional { requirement } => {
            assert_eq!(
                requirement, QA_REQUIREMENT,
                "the provisional mark names another requirement"
            )
        }
        other => panic!("the provisional (−∞, ∞) is not provisional: {other:?}"),
    }
    // §3.8 `provisional(requirement)`: "drawn meanwhile as (−∞, ∞), so spanning zero".
    assert!(
        e.range_state().spans_zero(),
        "a provisional range does not span zero"
    );
}

#[test]
fn qa_m1_21_state_provisional_unbounded() {
    check_provisional_state(&marked(
        UNBOUNDED,
        Some(Declaration::Provisional(QA_REQUIREMENT)),
    ));
}

negative_control!(
    qa_m1_21_state_provisional_unbounded,
    "a cited (−∞, ∞) is declared, not provisional",
    expected = "the provisional (−∞, ∞) is not provisional",
    check_provisional_state(&marked(UNBOUNDED, Some(Declaration::Cited(QA_CITATION))))
);

fn check_marked_not_gap(e: &Entry) {
    assert_ne!(e.range_state(), RangeState::Gap, "a marked range is a gap");
}

#[test]
fn qa_m1_21_state_provisional_written_bounds_are_no_gap() {
    // A range the human has marked is not one "no range is declared" for (§3.8 `gap`), whatever bounds it writes.
    for range in [from_zero(), signed_finite(), UNBOUNDED] {
        check_marked_not_gap(&marked(
            range,
            Some(Declaration::Provisional(QA_REQUIREMENT)),
        ));
    }
}

negative_control!(
    qa_m1_21_state_provisional_written_bounds_are_no_gap,
    "the unmarked (−∞, ∞) is a gap",
    expected = "a marked range is a gap",
    check_marked_not_gap(&marked(UNBOUNDED, None))
);

// ── A marked fresh field generates on cool-warm; a gap never carries `diverging` ─────────────────────────────────

/// The payload ledger plus a fresh `qa_fixture` of type `ty`, derived from `S`, `scale` over (−∞, ∞) with `mark`.
fn with_marked_fixture(ty: FieldType, scale: Scale, mark: Option<Declaration>) -> Ledger {
    let mut l = payload::ledger();
    let mut b = EntryBuilder::new("qa_fixture")
        .location(Location::Derived { from: vec!["S"] })
        .ty(ty)
        .scale(scale)
        .range(UNBOUNDED)
        .provenance(ledger::schema::Provenance::Kernel)
        .consumers(&[ledger::schema::Consumer::Render]);
    b.declaration = mark;
    l.entries.push(b);
    l
}

/// Generation of `l` passes and draws `qa_fixture` on `ramp_coolwarm`.
fn check_generates_coolwarm(l: &Ledger) {
    let es = ledger::gen::validate(l).unwrap_or_else(|e| panic!("generation refused: {e}"));
    assert_eq!(
        NumericView::of(entry(&es, "qa_fixture")).map(|v| v.ramp()),
        Some(Ramp::Coolwarm),
        "the fixture is not on ramp_coolwarm"
    );
}

#[test]
fn qa_m1_21_marked_fresh_cited_generates_on_coolwarm() {
    // R-400: a `lin` field whose declared range spans zero is on the diverging ramp.
    check_generates_coolwarm(&with_marked_fixture(
        FieldType::F32,
        Scale::Lin,
        Some(Declaration::Cited(QA_CITATION)),
    ));
}

negative_control!(
    qa_m1_21_marked_fresh_cited_generates_on_coolwarm,
    "the same fixture uncited is a gap, refused",
    expected = "generation refused",
    check_generates_coolwarm(&with_marked_fixture(FieldType::F32, Scale::Lin, None))
);

#[test]
fn qa_m1_21_marked_fresh_provisional_generates_on_coolwarm() {
    // R-426: a provisional range is kept on cool-warm.
    check_generates_coolwarm(&with_marked_fixture(
        FieldType::F32,
        Scale::Lin,
        Some(Declaration::Provisional(QA_REQUIREMENT)),
    ));
}

negative_control!(
    qa_m1_21_marked_fresh_provisional_generates_on_coolwarm,
    "the same fixture unmarked is a gap, refused",
    expected = "generation refused",
    check_generates_coolwarm(&with_marked_fixture(FieldType::F32, Scale::Lin, None))
);

fn f32_vector() -> FieldType {
    FieldType::Vector {
        component: Box::new(FieldType::F32),
        k: 3,
    }
}

#[test]
fn qa_m1_21_gap_diverging_vector_refused() {
    // REQ-GEN-035: the (−∞, ∞) written where §3 gives no range "never ... lets it carry the `diverging` scale";
    // §3.8: an entry carries `diverging` "only when its declared range spans zero". A vector is no exception.
    check_refused(
        &with_marked_fixture(f32_vector(), Scale::Diverging, None),
        "qa_fixture",
    );
}

negative_control!(
    qa_m1_21_gap_diverging_vector_refused,
    "the same vector with its (−∞, ∞) cited carries `diverging` by a declared range, and generates",
    expected = "generation was not refused for the undeclared `qa_fixture`",
    check_refused(
        &with_marked_fixture(
            f32_vector(),
            Scale::Diverging,
            Some(Declaration::Cited(QA_CITATION))
        ),
        "qa_fixture"
    )
);

// ── Every drawn `lin` or `diverging` scalar cites a source ──────────────────────────────────────────────────────

/// Each `lin` or `diverging` scalar in `es` the numeric template draws is declared with a citation naming a source
/// (a section or a ruling), or is one of R-426's two provisional fields.
fn check_drawn_cited(es: &[Entry]) {
    for e in es {
        let ramp_scale = matches!(e.scale, Scale::Lin | Scale::Diverging);
        if !ramp_scale || NumericView::of(e).is_none() {
            continue;
        }
        match e.range_state() {
            RangeState::Declared { citation, .. } => assert!(
                citation != ledger::schema::WRITTEN_BOUNDS
                    && (citation.contains('§') || citation.contains("R-")),
                "`{}`'s range cites no source: {citation}",
                e.name
            ),
            RangeState::Provisional { .. } => assert!(
                ["q_mass", "mean_y"].contains(&e.name),
                "`{}` is provisional, which only R-426's two fields are",
                e.name
            ),
            RangeState::Gap => panic!("`{}` is drawn from a gap", e.name),
        }
    }
}

#[test]
fn qa_m1_21_drawn_fields_cite_a_source() {
    check_drawn_cited(&payload());
}

negative_control!(
    qa_m1_21_drawn_fields_cite_a_source,
    "`S` with a bounded range and its citation dropped cites no source",
    expected = "`S`'s range cites no source",
    {
        let mut l = payload::ledger();
        let s = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("S"))
            .expect("`S`");
        s.range = Some(Range {
            lo: Bound::Closed(-1.0),
            hi: Bound::Closed(1.0),
        });
        s.declaration = None;
        let es = ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
        check_drawn_cited(&es);
    }
);
