//! QA tests for TASK-M5-32, written from REQ-PAY-109 (R-72, R-545, RQ-278 (d)) and dd_generation_root §3.7, not from
//! the implementation.
//!
//! REQ-PAY-109: "`running_mean_divergence`'s and `coherence`'s formulas and declared §3.8 ranges, each with its
//! citation and neither spanning zero, must be defined in dd_generation_root §3.7 and the ledger […] so both stay
//! `lin` on viridis." Its verify detail: both declared with ranges that do not span zero, each citing §3.7's formula,
//! the numeric template places neither on `ramp_coolwarm`; control: a fixture `lin` entry declared (−∞, ∞) is placed
//! on `ramp_coolwarm`.
//!
//! - `declared_ranges_qa_m5_32_doc_rows_match_ledger`: §3.8 entries table rows of dd_generation_root (the authority)
//!   give each member `lin` and a range that does not span zero, and the ledger declares exactly that range.
//! - `declared_ranges_qa_m5_32_citation_names_formula_section`: each citation names §3.7, the §3.7 subsection whose
//!   text holds the member's formula, the range and REQ-PAY-109; the subsection is checked against the doc.
//! - `declared_ranges_qa_m5_32_template_not_coolwarm`: the generated numeric view's WGSL draws each member on
//!   `ramp_viridis` at the fixed declared range, never on `ramp_coolwarm`.
//! - `declared_ranges_qa_m5_32_unbounded_fixture_coolwarm`: REQ-PAY-109's control, a fixture `lin` entry built here
//!   (not a retagged ledger member) declared (−∞, ∞) is drawn on `ramp_coolwarm`, and the boundary fixtures either
//!   side of zero land where `lo < 0 < hi` says.
//! - `declared_ranges_qa_m5_32_formula_values_in_declared_range`: every value §3.7's formulas give in f32 (the running
//!   mean from `m_0 = 0` with a non-finite spread read as 1; `1 − n_unresolved/N²` for `0 ≤ n_unresolved ≤ N²`) lies
//!   in the ledger's declared range, so the declaration is the formula's, not picked.
//!
//! Each test registers its negative control (R-176).

use ledger::gen::numeric::{NumericView, Placement, Ramp};
use ledger::schema::{
    Bound, Consumer, Entry, EntryBuilder, FieldType, Location, Provenance, Range, RangeState, Scale,
};
use proptest::prelude::*;
use validation::{negative_control, prop};

/// The two members REQ-PAY-109 names, with the formula text dd_generation_root §3.7 gives each.
const MEMBERS: [(&str, &str); 2] = [
    (
        "running_mean_divergence",
        "running_mean_divergence(t) = (1/n)·Σ",
    ),
    ("coherence", "coherence = 1 − n_unresolved / N²"),
];

fn doc() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/design/principia_dd_generation_root.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn reduction() -> Vec<Entry> {
    ledger::gen::validate(&ledger::reduction::ledger()).unwrap_or_else(|e| panic!("{e}"))
}

fn entry<'a>(entries: &'a [Entry], name: &str) -> &'a Entry {
    entries
        .iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no reduction member `{name}`"))
}

/// Parses a §3.8 range cell as the doc writes it: `[a, b]`, `(a, b)`, mixed, with `∞` / `−∞` unbounded.
fn parse_range(cell: &str) -> Range {
    let cell = cell.trim();
    let num = |s: &str| -> Option<f64> {
        let s = s.trim().replace('−', "-");
        if s.ends_with('∞') {
            None
        } else {
            Some(s.parse().unwrap_or_else(|_| panic!("bad bound `{s}`")))
        }
    };
    let (open_lo, open_hi) = (cell.starts_with('('), cell.ends_with(')'));
    let inner = &cell[cell.char_indices().nth(1).unwrap().0..cell.len() - 1];
    let (a, b) = inner.split_once(',').expect("a range cell has a comma");
    let bound = |v: Option<f64>, open: bool| match v {
        None => Bound::Unbounded,
        Some(v) if open => Bound::Open(v),
        Some(v) => Bound::Closed(v),
    };
    Range {
        lo: bound(num(a), open_lo),
        hi: bound(num(b), open_hi),
    }
}

/// Checks that `doc`'s §3.8 entries table row for each member gives `lin` and a range that does not span zero, and
/// that `entries` declares that exact range, `lin`, with a citation (not a gap, not provisional).
fn check_doc_rows(doc: &str, entries: &[Entry]) {
    for (name, _) in MEMBERS {
        let prefix = format!("| `{name}` |");
        let row = subsection(doc, "Layout and §3.8 entries")
            .lines()
            .find(|l| l.starts_with(&prefix))
            .unwrap_or_else(|| panic!("no §3.8 table row for `{name}`"));
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        // | name | location | type | scale | range | notes |
        assert_eq!(cells[4], "lin", "`{name}`'s row is not `lin`: {row}");
        let range = parse_range(cells[5]);
        assert!(
            !range.spans_zero(),
            "`{name}`'s §3.8 row declares a range spanning zero: {row}"
        );
        let e = entry(entries, name);
        assert_eq!(e.scale, Scale::Lin, "`{name}` is not `lin` in the ledger");
        match e.range_state() {
            RangeState::Declared {
                range: declared,
                citation,
            } => {
                assert_eq!(
                    declared, range,
                    "`{name}`: the ledger declares {declared}, the doc row {range}"
                );
                assert_ne!(
                    citation,
                    ledger::schema::WRITTEN_BOUNDS,
                    "`{name}` is declared by its bounds alone, with no citation"
                );
            }
            other => panic!("`{name}`'s range is not declared: {other:?}"),
        }
    }
}

/// dd_generation_root's §3.8 table rows and the ledger agree, each a cited range that does not span zero.
#[test]
fn declared_ranges_qa_m5_32_doc_rows_match_ledger() {
    check_doc_rows(&doc(), &reduction());
}

negative_control!(
    declared_ranges_qa_m5_32_doc_rows_match_ledger,
    "the doc row as PR #208 built it, (−∞, ∞), is refused",
    expected = "declares a range spanning zero",
    check_doc_rows(
        &doc().replace(
            "| `coherence` | scalar-index 43 | f32 | lin | [0, 1] |",
            "| `coherence` | scalar-index 43 | f32 | lin | (−∞, ∞) |",
        ),
        &reduction()
    )
);

/// The text of §3.7 subsection `heading` (a `####` heading) of `doc`.
fn subsection<'a>(doc: &'a str, heading: &str) -> &'a str {
    let start = doc.find("\n### 3.7 ").expect("§3.7");
    let end = start + doc[start..].find("\n### 3.7a").expect("§3.7a");
    let s37 = &doc[start..end];
    let marker = format!("\n#### {heading}");
    let at = s37
        .find(&marker)
        .unwrap_or_else(|| panic!("§3.7 has no subsection \"{heading}\""));
    let body = &s37[at + marker.len()..];
    &body[..body.find("\n#### ").unwrap_or(body.len())]
}

/// Checks each member's citation in `entries`: it names §3.7 and a §3.7 subsection (quoted after `§3.7`) whose text
/// holds the member's formula, the declared `[0, 1]`, and REQ-PAY-109.
fn check_citations(doc: &str, entries: &[Entry]) {
    for (name, formula) in MEMBERS {
        let RangeState::Declared { citation, .. } = entry(entries, name).range_state() else {
            panic!("`{name}`'s range is not declared");
        };
        let after = citation
            .split_once("§3.7 \"")
            .unwrap_or_else(|| panic!("`{name}`'s citation names no §3.7 subsection: `{citation}`"))
            .1;
        let heading = &after[..after.find('"').expect("a closing quote")];
        assert!(
            subsection(doc, heading).contains(formula),
            "`{name}`'s citation names §3.7 \"{heading}\", which does not give its formula `{formula}`"
        );
        for phrase in ["REQ-PAY-109", "[0, 1]"] {
            assert!(
                citation.contains(phrase),
                "`{name}`'s citation does not name {phrase}: `{citation}`"
            );
        }
    }
}

#[test]
fn declared_ranges_qa_m5_32_citation_names_formula_section() {
    check_citations(&doc(), &reduction());
}

negative_control!(
    declared_ranges_qa_m5_32_citation_names_formula_section,
    "`coherence` citing the subsection that gives `running_mean_divergence`'s formula is refused",
    expected = "which does not give its formula",
    {
        let mut l = ledger::reduction::ledger();
        let e = l
            .entries
            .iter_mut()
            .find(|e| e.name == Some("coherence"))
            .expect("coherence");
        e.declaration = Some(ledger::schema::Declaration::Cited(
            "dd_generation_root §3.7 \"Temporal accumulators\": [0, 1] (REQ-PAY-109)",
        ));
        check_citations(
            &doc(),
            &ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}")),
        );
    }
);

/// Checks that `e`'s numeric view is `lin` on `ramp_viridis`, at the fixed declared range with `RANGE_AUTO` off, and
/// that its generated `colour()` calls `ramp_viridis` and never `ramp_coolwarm`.
fn check_viridis(e: &Entry) {
    let name = e.name;
    let view = NumericView::of(e).unwrap_or_else(|| panic!("`{name}` has no numeric view"));
    let wgsl = view.colour("ctx.value");
    assert!(
        !wgsl.contains("ramp_coolwarm"),
        "`{name}` is drawn on ramp_coolwarm:\n{wgsl}"
    );
    assert_eq!(view.ramp(), Ramp::Viridis, "`{name}` is not on viridis");
    assert_eq!(
        view.placement,
        Placement::Lin,
        "`{name}` is not placed `lin`"
    );
    assert!(wgsl.contains("ramp_viridis("), "`{name}`'s WGSL:\n{wgsl}");
    assert_eq!(
        view.u_range_default(),
        [0.0, 1.0],
        "`{name}`'s fixed range is not [0, 1]"
    );
    assert!(
        !view.header().contains("RANGE_AUTO: u32 = 1"),
        "`{name}` defaults to the measured range though its range is declared"
    );
}

#[test]
fn declared_ranges_qa_m5_32_template_not_coolwarm() {
    let entries = reduction();
    for (name, _) in MEMBERS {
        check_viridis(entry(&entries, name));
    }
}

/// A standalone `lin` f32 fixture field at a scalar slot, declared `range`, cited.
fn fixture(range: Range) -> Entry {
    EntryBuilder::new("fx_one_signed")
        .location(Location::Scalar(0))
        .ty(FieldType::F32)
        .scale(Scale::Lin)
        .range(range)
        .provenance(Provenance::Kernel)
        .consumers(&[Consumer::Debug])
        .cited("a qa fixture (TASK-M5-32)")
        .build()
        .unwrap_or_else(|e| panic!("{e:?}"))
}

negative_control!(
    declared_ranges_qa_m5_32_template_not_coolwarm,
    "a `lin` fixture declared (−∞, ∞) is drawn on ramp_coolwarm",
    expected = "is drawn on ramp_coolwarm",
    check_viridis(&fixture(Range::UNBOUNDED))
);

/// Checks that `e`'s numeric view is drawn on `ramp_coolwarm`.
fn check_coolwarm(e: &Entry) {
    let view = NumericView::of(e).expect("a numeric view");
    let wgsl = view.colour("ctx.value");
    assert!(
        view.ramp() == Ramp::Coolwarm && wgsl.contains("ramp_coolwarm("),
        "`{}` declared {} is not drawn on ramp_coolwarm:\n{wgsl}",
        e.name,
        e.range
    );
}

/// REQ-PAY-109's control: a fixture `lin` entry declared (−∞, ∞) is placed on `ramp_coolwarm`; so is one declared
/// `[−1, 1]`, while the one-signed boundaries `[0, 1]`, `(0, 1]` and `[−0, 1]` stay on viridis.
#[test]
fn declared_ranges_qa_m5_32_unbounded_fixture_coolwarm() {
    check_coolwarm(&fixture(Range::UNBOUNDED));
    check_coolwarm(&fixture(Range {
        lo: Bound::Closed(-1.0),
        hi: Bound::Closed(1.0),
    }));
    for lo in [Bound::Closed(0.0), Bound::Open(0.0), Bound::Closed(-0.0)] {
        let e = fixture(Range {
            lo,
            hi: Bound::Closed(1.0),
        });
        assert_eq!(
            NumericView::of(&e).expect("a numeric view").ramp(),
            Ramp::Viridis,
            "`{}` is not on viridis",
            e.range
        );
    }
}

negative_control!(
    declared_ranges_qa_m5_32_unbounded_fixture_coolwarm,
    "the fixture declared [0, 1] is not drawn on ramp_coolwarm",
    expected = "is not drawn on ramp_coolwarm",
    check_coolwarm(&fixture(Range::int(0, 1)))
);

/// Whether `v` lies in `range`, a non-finite `v` never.
fn within(range: Range, v: f32) -> bool {
    let v = f64::from(v);
    let lo = match range.lo {
        Bound::Closed(b) => v >= b,
        Bound::Open(b) => v > b,
        Bound::Unbounded => true,
    };
    let hi = match range.hi {
        Bound::Closed(b) => v <= b,
        Bound::Open(b) => v < b,
        Bound::Unbounded => true,
    };
    v.is_finite() && lo && hi
}

/// A footprint's divergence (§3.7 "Temporal accumulators"): its `spread_shape`, a non-finite one read as 1 by its
/// exponent bits when `select` is on.
fn divergence(spread: f32, select: bool) -> f32 {
    if select && spread.to_bits() & 0x7f80_0000 == 0x7f80_0000 {
        1.0
    } else {
        spread
    }
}

/// Checks §3.7's formulas against the ledger's declared ranges: over `grid` (one `Vec` of the quad's N² footprint
/// spreads per sim-time step), each `m_j = m_{j−1} + (D_j − m_{j−1})/j` from `m_0 = 0`, `D_j` the f32 mean of the
/// footprints' divergence, lies in `running_mean_divergence`'s range, with `m_1 = D_1`; and `1 − n/N²` lies in
/// `coherence`'s for every `0 ≤ n ≤ N²`.
fn check_formulas(entries: &[Entry], grid: &[Vec<f32>], select: bool) -> Result<(), TestCaseError> {
    let range = entry(entries, "running_mean_divergence").range;
    let mut m = 0.0f32;
    for (j, spreads) in grid.iter().enumerate() {
        let sum = spreads
            .iter()
            .fold(0.0f32, |s, &x| s + divergence(x, select));
        let d = sum / spreads.len() as f32;
        m += (d - m) / (j + 1) as f32;
        if j == 0 {
            prop_assert_eq!(m.to_bits(), d.to_bits(), "m_1 is not D_1");
        }
        prop_assert!(
            within(range, m),
            "running_mean_divergence {} at step {} is outside the declared {}",
            m,
            j + 1,
            range
        );
    }
    let range = entry(entries, "coherence").range;
    let footprints = grid[0].len() as u32;
    for n in 0..=footprints {
        let c = 1.0f32 - n as f32 / footprints as f32;
        prop_assert!(
            within(range, c),
            "coherence {} (n_unresolved {} of {}) is outside the declared {}",
            c,
            n,
            footprints,
            range
        );
    }
    Ok(())
}

/// The grids: a quad of N² footprints (N in 1..=8), 1 to 64 sim-time steps, each spread in [0, 1], its ends, or
/// non-finite.
fn grids() -> impl Strategy<Value = Vec<Vec<f32>>> {
    let spread = prop_oneof![
        6 => 0.0f32..=1.0,
        1 => Just(0.0f32),
        1 => Just(1.0f32),
        1 => Just(f32::NAN),
        1 => Just(f32::INFINITY),
        1 => Just(f32::NEG_INFINITY),
    ];
    (1usize..=8).prop_flat_map(move |n| {
        proptest::collection::vec(proptest::collection::vec(spread.clone(), n * n), 1..=64)
    })
}

#[test]
fn declared_ranges_qa_m5_32_formula_values_in_declared_range() {
    let entries = reduction();
    prop::run(&grids(), |grid| check_formulas(&entries, &grid, true));
}

negative_control!(
    declared_ranges_qa_m5_32_formula_values_in_declared_range,
    "without the non-finite select, a NaN or infinite spread leaves the declared range",
    expected = "is outside the declared",
    {
        let entries = reduction();
        prop::run(&grids(), |grid| check_formulas(&entries, &grid, false));
    }
);
