//! The declared lower bounds (REQ-RENDER-084; R-400; TASK-M1-17): a numeric view whose field's declared range spans
//! zero is drawn on the diverging ramp, so each field the corpus or its own definition makes non-negative declares
//! `lo ≥ 0`, citing the line that does (`ledger::payload::NON_NEGATIVE`): the mass fractions and `|ρ|` of
//! render_gui_spec §10.1, `virial_ratio = 2K₀/|V₀|` and `K₀` of dd_decoder §3.6, and the latches and log fields of
//! dd_generation_root §3.4 and §3.6 (`declared_ranges_*`). The unsigned bit fields are non-negative by their encoding.
//!
//! The range states (REQ-GEN-035; R-426; dd_generation_root §3.8): an undeclared range is a ledger gap, not one that
//! spans zero. Every `lin` or `diverging` scalar field the numeric template draws has a declared range with its
//! citation, or is `q_mass` or `mean_y`, provisional (`declared_ranges_drawn_fields_are_declared`); `S`, `ftle`,
//! `diffusion`, `E_0`, `Lz_0`, `C_ty`, `energy_drift` and `Lz_drift` are declared (−∞, ∞), each citing its source
//! (`declared_ranges_signed_fields_cite_their_sources`); `q_mass` and `mean_y` are provisional, naming REQ-PAY-097 and
//! REQ-PAY-098, and on `ramp_coolwarm` (`declared_ranges_provisional_*`); a `lin` or `diverging` scalar whose range is
//! a gap fails generation naming it and R-426, and is not placed on `ramp_coolwarm`, while a gap on a cyclic field or
//! a vector generates (`declared_ranges_gap_*`); and the gap list names each remaining gap with the requirement that
//! fills it, for the M1 gate report (`declared_ranges_gap_list`, printed with `--nocapture`).
//!
//! Each test registers its negative control (R-176).

use ledger::gen::numeric::{self, NumericView, Ramp};
use ledger::payload::{self, GAPS, NON_NEGATIVE, PROVISIONAL, SIGNED};
use ledger::schema::{
    Bound, Consumer, Entry, EntryBuilder, FieldType, Ledger, Location, Provenance, Range,
    RangeState, Scale,
};
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

// ── R-426: the range states ─────────────────────────────────────────────────────────────────────────────────────────

/// Each payload entry with its range's state.
fn states() -> Vec<(Entry, RangeState)> {
    payload()
        .into_iter()
        .map(|e| {
            let state = payload::range_state(&e)
                .unwrap_or_else(|| panic!("`{}` is a payload field with no range state", e.name));
            (e, state)
        })
        .collect()
}

/// Whether `e` is a `lin` or `diverging` scalar: a field whose ramp the template would choose from its range.
fn lin_or_diverging_scalar(e: &Entry) -> bool {
    matches!(e.scale, Scale::Lin | Scale::Diverging) && !matches!(e.ty, FieldType::Vector { .. })
}

/// Whether `citation` names a section or a ruling.
fn names_a_line(citation: &str) -> bool {
    citation.contains('§') || citation.contains("R-")
}

/// Checks that every `lin` or `diverging` scalar field the numeric template draws, and the floored drifts it skips,
/// has a declared range citing a line, or is `q_mass` or `mean_y` marked provisional; and that the drifts are among
/// them.
fn check_drawn(states: &[(Entry, RangeState)]) {
    let mut drifts = 0;
    for (e, state) in states {
        let floored = e.floor.is_some();
        if !lin_or_diverging_scalar(e) || (NumericView::of(e).is_none() && !floored) {
            continue;
        }
        drifts += usize::from(floored);
        match state {
            RangeState::Declared { citation, .. } => assert!(
                names_a_line(citation),
                "`{}`'s declared range cites no line: `{citation}`",
                e.name
            ),
            RangeState::Provisional { .. } => assert!(
                ["q_mass", "mean_y"].contains(&e.name),
                "`{}` is provisional, but R-426 keeps only `q_mass` and `mean_y` so",
                e.name
            ),
            RangeState::Gap => panic!("`{}` is drawn from a range it does not declare", e.name),
        }
    }
    assert_eq!(drifts, 2, "the floored drifts are not both checked");
}

#[test]
fn declared_ranges_drawn_fields_are_declared() {
    check_drawn(&states());
}

negative_control!(
    declared_ranges_drawn_fields_are_declared,
    "`S` with its citation dropped is a gap the template would draw",
    expected = "`S` is drawn from a range it does not declare",
    {
        let mut states = states();
        states.iter_mut().find(|(e, _)| e.name == "S").expect("S").1 = RangeState::Gap;
        check_drawn(&states);
    }
);

/// Checks that each field R-426 declares signed is declared (−∞, ∞), citing what `want` gives: each phrase in it.
fn check_signed(states: &[(Entry, RangeState)], want: &[(&str, &[&str])]) {
    for (field, phrases) in want {
        let (_, state) = states
            .iter()
            .find(|(e, _)| e.name == *field)
            .unwrap_or_else(|| panic!("no payload field `{field}`"));
        let RangeState::Declared { range, citation } = state else {
            panic!("`{field}`'s range is not declared: {state:?}");
        };
        assert_eq!(
            *range,
            Range::UNBOUNDED,
            "`{field}` is not declared (−∞, ∞)"
        );
        for phrase in *phrases {
            assert!(
                citation.contains(phrase),
                "`{field}`'s citation does not cite {phrase}: `{citation}`"
            );
        }
    }
}

/// R-426's declarations, and the fix round's: each field and what its citation names.
const SIGNED_SOURCES: [(&str, &[&str]); 8] = [
    ("S", &["R-423", "signed at finite time"]),
    ("ftle", &["R-423", "signed at finite time"]),
    ("diffusion", &["R-423", "signed at finite time"]),
    ("E_0", &["§3.4", "diverging"]),
    ("Lz_0", &["§3.4", "diverging"]),
    ("C_ty", &["R-423", "C_ty = diffusion · C_tt(n)", "n ≥ 2"]),
    ("energy_drift", &["§3.8's worked entry", "§3.4", "signed"]),
    ("Lz_drift", &["§3.8's worked entry", "§3.4", "signed"]),
];

#[test]
fn declared_ranges_signed_fields_cite_their_sources() {
    let states = states();
    check_signed(&states, &SIGNED_SOURCES);
    assert_eq!(SIGNED.len(), SIGNED_SOURCES.len(), "SIGNED declares others");
    // The drifts, which the numeric template skips as floored fields, generate: the gate passes on them.
    for drift in ["energy_drift", "Lz_drift"] {
        let (e, _) = states
            .iter()
            .find(|(e, _)| e.name == drift)
            .expect("a drift");
        assert!(
            e.floor.is_some() && NumericView::of(e).is_none(),
            "`{drift}` is drawn"
        );
        assert_eq!(numeric::undeclared(e), None, "`{drift}` is refused");
    }
}

negative_control!(
    declared_ranges_signed_fields_cite_their_sources,
    "`C_ty` citing only R-423, without its derivation",
    expected = "`C_ty`'s citation does not cite C_ty = diffusion · C_tt(n)",
    {
        let mut states = states();
        let c = states
            .iter_mut()
            .find(|(e, _)| e.name == "C_ty")
            .expect("C_ty");
        c.1 = RangeState::Declared {
            range: Range::UNBOUNDED,
            citation: "R-423",
        };
        check_signed(&states, &SIGNED_SOURCES);
    }
);

/// Checks that `q_mass` and `mean_y` are provisional, naming REQ-PAY-097 and REQ-PAY-098, write (−∞, ∞) and are
/// drawn on `ramp_coolwarm` (R-426); and that no other field is provisional.
fn check_provisional(states: &[(Entry, RangeState)]) {
    for (field, requirement) in [("q_mass", "REQ-PAY-097"), ("mean_y", "REQ-PAY-098")] {
        let (e, state) = states.iter().find(|(e, _)| e.name == field).expect(field);
        assert_eq!(
            *state,
            RangeState::Provisional { requirement },
            "`{field}` is not provisional until {requirement}"
        );
        assert_eq!(
            e.range,
            Range::UNBOUNDED,
            "`{field}` does not write (−∞, ∞)"
        );
        let view = NumericView::of(e).expect("a numeric view");
        assert_eq!(view.ramp(), Ramp::Coolwarm, "`{field}` left cool-warm");
    }
    let provisional = states
        .iter()
        .filter(|(_, s)| matches!(s, RangeState::Provisional { .. }))
        .count();
    assert_eq!(
        provisional, 2,
        "fields other than q_mass and mean_y are provisional"
    );
}

#[test]
fn declared_ranges_provisional_q_mass_and_mean_y() {
    check_provisional(&states());
    assert_eq!(PROVISIONAL.len(), 2);
}

negative_control!(
    declared_ranges_provisional_q_mass_and_mean_y,
    "`mean_y` read as a gap leaves cool-warm",
    expected = "`mean_y` is not provisional until REQ-PAY-098",
    {
        let mut states = states();
        states
            .iter_mut()
            .find(|(e, _)| e.name == "mean_y")
            .expect("mean_y")
            .1 = RangeState::Gap;
        check_provisional(&states);
    }
);

/// A range written in place of a provisional (−∞, ∞) is its requirement's declaration, and its bounds decide the
/// ramp: `q_mass` written `[0, 1]` is declared, citing REQ-PAY-097, and on viridis, as REQ-PAY-097's range would put it.
#[test]
fn declared_ranges_provisional_written_range_is_declared() {
    check_written_provisional(Range {
        lo: Bound::Closed(0.0),
        hi: Bound::Closed(1.0),
    });
}

fn check_written_provisional(range: Range) {
    let mut e = payload()
        .into_iter()
        .find(|e| e.name == "q_mass")
        .expect("q_mass");
    e.range = range;
    assert_eq!(
        payload::range_state(&e),
        Some(RangeState::Declared {
            range,
            citation: "REQ-PAY-097"
        }),
        "`q_mass` written {range} is not declared by REQ-PAY-097"
    );
    let ramp = NumericView::of(&e).expect("a numeric view").ramp();
    assert_eq!(
        ramp,
        Ramp::Viridis,
        "`q_mass` written {range} is on {ramp:?}"
    );
}

negative_control!(
    declared_ranges_provisional_written_range_is_declared,
    "`q_mass` written (−∞, ∞) is still provisional",
    expected = "`q_mass` written (−∞, ∞) is not declared by REQ-PAY-097",
    check_written_provisional(Range::UNBOUNDED)
);

/// The payload ledger with entry `name` given `scale`.
fn rescaled(name: &str, scale: Scale) -> Ledger {
    let mut l = ledger::layout();
    l.entries
        .iter_mut()
        .find(|e| e.name == Some(name))
        .unwrap_or_else(|| panic!("no ledger entry `{name}`"))
        .scale = Some(scale);
    l
}

/// Checks that generation of `l` is refused by the gap gate alone, naming `field`, its scale and R-426: one line, not
/// also R-403's refusal of a range that does not span zero.
fn check_gap_refused(l: &Ledger, field: &str, scale: &str) {
    let err = match ledger::gen::validate(l) {
        Ok(_) => panic!("generation was not refused for the gap `{field}`"),
        Err(e) => e.to_string(),
    };
    let lines: Vec<&str> = err
        .lines()
        .filter(|l| l.contains(&format!("`{field}`")))
        .collect();
    assert!(
        lines.len() == 1
            && lines[0].contains(&format!("scale `{scale}`"))
            && lines[0].contains("R-426")
            && lines[0].contains("gap"),
        "the refusal of `{field}` is not the gap's alone: {err}"
    );
}

/// The payload ledger plus a fixture field `fx_gap`, an f32 derived from `S`, with `scale` over (−∞, ∞) and no mark:
/// a field new to the ledger, cited nowhere, so its range is a gap whatever its name.
fn with_fixture(scale: Scale) -> Ledger {
    let mut l = ledger::layout();
    l.entries.push(
        EntryBuilder::new("fx_gap")
            .location(Location::Derived { from: vec!["S"] })
            .ty(FieldType::F32)
            .scale(scale)
            .range(Range::UNBOUNDED)
            .provenance(Provenance::Kernel)
            .consumers(&[Consumer::Debug]),
    );
    l
}

/// A `lin` or `diverging` scalar whose range is a gap fails generation naming it (R-426): `theta` and `rho_angle`,
/// gaps, retagged `lin` and `diverging`, and a fixture field with no mark.
#[test]
fn declared_ranges_gap_lin_or_diverging_is_refused() {
    for field in ["theta", "rho_angle"] {
        check_gap_refused(&rescaled(field, Scale::Lin), field, "lin");
        check_gap_refused(&rescaled(field, Scale::Diverging), field, "diverging");
    }
    check_gap_refused(&with_fixture(Scale::Lin), "fx_gap", "lin");
    check_gap_refused(&with_fixture(Scale::Diverging), "fx_gap", "diverging");
}

negative_control!(
    declared_ranges_gap_lin_or_diverging_is_refused,
    "`theta` as the ledger writes it, cyclic, generates",
    expected = "generation was not refused for the gap `theta`",
    check_gap_refused(&ledger::layout(), "theta", "cyclic")
);

/// A gap on an entry no ramp reads the range of generates (§3.8): a cyclic field, a vector (even `lin`), and a gap
/// retagged `log`, whose view is on the fixed `[0, 1]`; so does a provisional range and a declared one.
#[test]
fn declared_ranges_gap_elsewhere_generates() {
    let fine = [
        ("theta", Scale::Cyclic),
        ("theta", Scale::Log),
        ("r", Scale::Lin),
        ("p_sh", Scale::Diverging),
        ("q_mass", Scale::Lin),
        ("S", Scale::Lin),
    ];
    check_generates(&fine);
}

fn check_generates(cases: &[(&str, Scale)]) {
    for &(field, scale) in cases {
        if let Err(e) = ledger::gen::validate(&rescaled(field, scale)) {
            panic!("`{field}` as {scale:?} was refused: {e}");
        }
    }
}

negative_control!(
    declared_ranges_gap_elsewhere_generates,
    "`rho_angle` retagged lin is a gap the gate refuses",
    expected = "`rho_angle` as Lin was refused",
    check_generates(&[("rho_angle", Scale::Lin)])
);

/// Checks that a `lin` view of each field of `fields`, gaps, is not on `ramp_coolwarm`: a gap never spans zero.
fn check_gap_not_coolwarm(fields: &[&str]) {
    for field in fields {
        let mut e = payload()
            .into_iter()
            .find(|e| e.name == *field)
            .expect("a field");
        e.scale = Scale::Lin;
        let ramp = NumericView::of(&e).expect("a numeric view").ramp();
        assert_ne!(
            ramp,
            Ramp::Coolwarm,
            "the gap `{field}` is drawn on cool-warm"
        );
        assert!(!payload::spans_zero(&e), "the gap `{field}` spans zero");
    }
}

#[test]
fn declared_ranges_gap_is_not_coolwarm() {
    check_gap_not_coolwarm(&["theta", "rho_angle"]);
    let fixture = with_fixture(Scale::Lin);
    let e = fixture
        .entries
        .last()
        .expect("the fixture")
        .build()
        .expect("complete");
    let ramp = NumericView::of(&e).expect("a numeric view").ramp();
    assert_ne!(
        ramp,
        Ramp::Coolwarm,
        "the fixture gap `fx_gap` is drawn on cool-warm"
    );
}

negative_control!(
    declared_ranges_gap_is_not_coolwarm,
    "`S`, declared (−∞, ∞), is no gap: it is on cool-warm",
    expected = "the gap `S` is drawn on cool-warm",
    check_gap_not_coolwarm(&["S"])
);

/// The state's own reading of "spans zero" (R-400, R-426), at the boundary: a declared range by its bounds, an end at 0
/// not spanning it; a provisional range always; a gap never.
#[test]
fn declared_ranges_state_spans_zero() {
    check_state_spans(|s| s.spans_zero());
}

fn check_state_spans(spans: fn(&RangeState) -> bool) {
    let declared = |lo, hi| RangeState::Declared {
        range: Range { lo, hi },
        citation: "§3.8",
    };
    let cases = [
        (declared(Bound::Unbounded, Bound::Unbounded), true),
        (declared(Bound::Open(-1.0), Bound::Closed(1.0)), true),
        (declared(Bound::Closed(0.0), Bound::Unbounded), false),
        (declared(Bound::Unbounded, Bound::Open(0.0)), false),
        (RangeState::Gap, false),
        (
            RangeState::Provisional {
                requirement: "REQ-PAY-097",
            },
            true,
        ),
    ];
    for (state, want) in cases {
        assert_eq!(spans(&state), want, "{state:?} spans zero: {}", !want);
    }
}

negative_control!(
    declared_ranges_state_spans_zero,
    "a gap read as its written (−∞, ∞) spans zero, R-400's amended reading",
    expected = "Gap spans zero: true",
    check_state_spans(|s| match s {
        RangeState::Declared { range, .. } => range.spans_zero(),
        _ => true,
    })
);

/// A field of another ledger has no payload range state; its written bounds decide whether it spans zero.
#[test]
fn declared_ranges_other_ledgers_keep_their_bounds() {
    check_other_ledger(&ledger::reduction::ledger());
}

fn check_other_ledger(l: &Ledger) {
    let entries = ledger::gen::validate(l).unwrap_or_else(|e| panic!("{e}"));
    let mut seen = [false; 2];
    for e in &entries {
        assert_eq!(
            payload::range_state(e),
            None,
            "`{}` has a payload range state",
            e.name
        );
        let spans = payload::spans_zero(e);
        assert_eq!(
            spans,
            e.range.spans_zero(),
            "`{}` over {} spans zero: {spans}",
            e.name,
            e.range
        );
        seen[usize::from(spans)] = true;
    }
    assert_eq!(
        seen, [true; 2],
        "the reduction ledger has ranges either side"
    );
}

negative_control!(
    declared_ranges_other_ledgers_keep_their_bounds,
    "the payload ledger's own fields have range states",
    expected = "has a payload range state",
    check_other_ledger(&ledger::layout())
);

/// The payload stays strict (R-426): every payload entry's range is cited or provisional, or the entry is one of
/// [`GAPS`]; none is declared by its bounds alone.
#[test]
fn declared_ranges_payload_entries_are_marked() {
    check_marked(&payload());
}

fn check_marked(entries: &[Entry]) {
    for e in entries {
        let gap = GAPS.iter().any(|(f, ..)| *f == e.name);
        assert!(
            e.declaration.is_some() || gap,
            "`{}` is neither cited, provisional nor a listed gap",
            e.name
        );
        assert_ne!(
            e.range_state(),
            RangeState::Declared {
                range: e.range,
                citation: ledger::schema::WRITTEN_BOUNDS
            },
            "`{}` is declared by its bounds alone",
            e.name
        );
    }
}

negative_control!(
    declared_ranges_payload_entries_are_marked,
    "`K_0` with its citation dropped is declared by its bounds alone",
    expected = "`K_0` is neither cited, provisional nor a listed gap",
    {
        let mut entries = payload();
        entries
            .iter_mut()
            .find(|e| e.name == "K_0")
            .expect("K_0")
            .declaration = None;
        check_marked(&entries);
    }
);

/// Checks that the payload fields whose range is a gap are exactly `gaps`, each with the requirement that fills it,
/// a requirement id or none, and its reason; and prints the list, for the M1 gate report (R-426).
fn check_gap_list(states: &[(Entry, RangeState)], gaps: &[(&str, Option<&str>, &str)]) {
    let found: Vec<&str> = states
        .iter()
        .filter(|(_, s)| *s == RangeState::Gap)
        .map(|(e, _)| e.name)
        .collect();
    let listed: Vec<&str> = gaps.iter().map(|(f, ..)| *f).collect();
    assert_eq!(found, listed, "the gap list is not the ledger's gaps");
    println!("The payload ledger's range gaps (R-426; REQ-GEN-035), for the M1 gate report:");
    for (field, fills, why) in gaps {
        assert!(
            fills.is_none_or(|r| r.starts_with("REQ-")),
            "`{field}`'s gap is filled by no requirement id: {fills:?}"
        );
        assert!(!why.is_empty(), "`{field}`'s gap gives no reason");
        let e = &states
            .iter()
            .find(|(e, _)| e.name == *field)
            .expect("a gap")
            .0;
        assert!(
            !lin_or_diverging_scalar(e),
            "the gap `{field}` would be refused"
        );
        println!("- `{field}`: filled by {}; {why}", fills.unwrap_or("none"));
    }
}

#[test]
fn declared_ranges_gap_list() {
    check_gap_list(&states(), &GAPS);
}

negative_control!(
    declared_ranges_gap_list,
    "a gap list missing `r`",
    expected = "the gap list is not the ledger's gaps",
    check_gap_list(&states(), &GAPS[1..])
);
