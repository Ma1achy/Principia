//! The literature-IC import (TASK-M2-20; REQ-VAL-019, REQ-VAL-120, REQ-VAL-174; R-592): Agekyan & Anosova's region-D
//! near-rest starts, Table 1 (`fixtures/anosova/`), and the Burrau rest start, each imported and multiplied back by the
//! recorded rescale (dd_validation_orbits §1.6 and §2).
//!
//! dd_validation_orbits §1.6's list, one test each: (i) the computed `M = η·10⁻⁴` on every row, and the printed column
//! 11 on every row but the exception list, the list itself asserted; (ii) `E` by eq. 4 against the start's own
//! energy; (iii) the exact-rest control; (iv) the invariants through the chart, multiplied back, and the scale-free
//! `L_z²·|E|/(G²·M⁵)`; (v) a point outside region D refused. Column 8 is recorded beside eq. 4 in
//! `fixtures/anosova/col8_comparison.csv`, never judged (RQ-570); the test checks only that the record is the
//! comparison. The Burrau constants (REQ-VAL-174) read back to their printed four decimals. The `literature-import`
//! gate runs its three inputs to their expected outcomes.
//!
//! **The tolerance** is the register's `literature_import_tol` (REQ-VAL-120, proposed by this task, provisional until
//! the M2 gate), `k` in `k·ε` relative at f64.

use std::path::Path;

use validation::import::{
    angular_eq3, check_exceptions, column8_csv, energy_eq4, fails, fields, import, in_region_d,
    invariants, printed_m, printed_report, region_d_report, relative, round_decimal, table1,
    unit_state, Back, Exception, LiteratureImportGate, Printed, RegionD, Rescale, Row, Source,
    Start, Tagged, TABLE1_HEADER,
};
use validation::negative_control;

const TOL: f64 = 256.0;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(p: &str) -> String {
    std::fs::read_to_string(root().join(p)).unwrap_or_else(|e| panic!("cannot read {p}: {e}"))
}

fn region() -> RegionD {
    RegionD::parse(&read("fixtures/anosova/region_d.json")).expect("region_d.json parses")
}

fn rows() -> Vec<Row> {
    table1(&read("fixtures/anosova/table1.csv")).expect("table1.csv parses")
}

#[test]
fn literature_import_tolerance_is_the_register_value() {
    assert_eq!(
        ledger::constants::LITERATURE_IMPORT_TOL.number(),
        TOL,
        "the register's literature_import_tol"
    );
}

// ── The fixture ───────────────────────────────────────────────────────────────────────────────────────────────────

/// Every row of Table 1 is kept, `N = 1 … 100` in order, each on its page (1010 for 1–52, 1011 for 53–96, 1012 for
/// 97–100); the `η` used is paper I's except `N = 88`, paper II's; paper II's `η` is recorded on exactly the six rows
/// where the papers differ; 62 rows print `E` and `M`, 38 print "—" for both.
fn check_table(rows: &[Row]) {
    assert_eq!(rows.len(), 100, "Table 1 has {} rows, not 100", rows.len());
    for (i, r) in rows.iter().enumerate() {
        let n = u32::try_from(i + 1).expect("small");
        assert_eq!(r.n, n, "row {i} is N = {}, not {n}", r.n);
        let page = match n {
            1..=52 => 1010,
            53..=96 => 1011,
            _ => 1012,
        };
        assert_eq!(r.page, page, "N = {n} is on page {}, not {page}", r.page);
        let (want_eta, want_source) = if n == 88 {
            (
                r.eta_ii.clone().expect("N = 88 carries paper II's η"),
                Source::II,
            )
        } else {
            (r.eta_i.clone(), Source::I)
        };
        assert_eq!(
            (&r.eta, r.source),
            (&want_eta, want_source),
            "N = {n}'s η used"
        );
        assert_eq!(
            r.e_col8.is_some(),
            r.m_col11.is_some(),
            "N = {n} prints one of E and M without the other"
        );
    }
    let differ: Vec<u32> = rows
        .iter()
        .filter(|r| r.eta_ii.is_some())
        .map(|r| r.n)
        .collect();
    assert_eq!(
        differ,
        [26, 61, 69, 77, 88, 93],
        "the rows where the papers' η differ"
    );
    let printed = rows.iter().filter(|r| r.m_col11.is_some()).count();
    assert_eq!(printed, 62, "the rows printing M");
}

#[test]
fn literature_import_table1_keeps_every_row() {
    check_table(&rows());
    assert_eq!(
        read("fixtures/anosova/table1.csv").lines().next(),
        Some(TABLE1_HEADER)
    );
}

negative_control!(
    literature_import_table1_keeps_every_row,
    "a table with its ΔE/E > 0.05 row 11 dropped does not keep every row",
    expected = "rows, not 100",
    check_table(&rows().into_iter().filter(|r| r.n != 11).collect::<Vec<_>>())
);

mod row88_paper_i {
    use super::*;
    negative_control!(
        literature_import_table1_keeps_every_row,
        "N = 88 with paper I's η, outside region D, is not the η R-592 uses",
        expected = "N = 88's η used",
        {
            let mut t = rows();
            t[87].eta = t[87].eta_i.clone();
            check_table(&t);
        }
    );
}

// ── (i) Angular momentum ──────────────────────────────────────────────────────────────────────────────────────────

/// On every row, `M = Σ m(xẏ − yẋ)` from eq. 3 is `η·10⁻⁴`.
fn check_m(d: &RegionD, rows: &[Row]) {
    for r in rows {
        let m = angular_eq3(d, r.xi(), r.eta());
        assert_eq!(
            m,
            r.eta() * 1e-4,
            "N = {}: M = {m:e} from eq. 3 is not η·10⁻⁴",
            r.n
        );
    }
}

#[test]
fn literature_import_angular_momentum_every_row() {
    check_m(&region(), &rows());
}

negative_control!(
    literature_import_angular_momentum_every_row,
    "with body 2 given a vertical velocity, M is no longer η·10⁻⁴",
    expected = "is not η·10⁻⁴",
    {
        let mut d = region();
        d.v[1][1] = 1e-5;
        check_m(&d, &rows());
    }
);

/// The printed column 11 matches `η·10⁻⁴` to its four decimals on every row printing it but the listed ones, which
/// are exactly the mismatches, with their printed and computed values.
fn check_printed(rows: &[Row], listed: &[Exception]) {
    if let Err(e) = check_exceptions(&printed_m(rows), listed) {
        panic!("Table 1's printed M against η·10⁻⁴: {e}");
    }
}

#[test]
fn literature_import_printed_m_but_the_exception_list() {
    let d = region();
    let listed: Vec<u32> = d.m_exceptions.iter().map(|e| e.n).collect();
    assert_eq!(
        listed,
        [16, 23, 26, 32, 34],
        "the exception list R-592 gives"
    );
    check_printed(&rows(), &d.m_exceptions);
}

negative_control!(
    literature_import_printed_m_but_the_exception_list,
    "N = 16 taken off the list is a new mismatch",
    expected = "mismatches and is not listed",
    check_printed(&rows(), &region().m_exceptions[1..])
);

mod listed_row_matches {
    use super::*;
    negative_control!(
        literature_import_printed_m_but_the_exception_list,
        "N = 1, which matches, put on the list is a listed row that matches",
        expected = "is listed but its printed M matches",
        {
            let mut listed = region().m_exceptions;
            listed.push(Exception {
                n: 1,
                printed: "0.5170".to_owned(),
                computed: "0.5170".to_owned(),
            });
            check_printed(&rows(), &listed);
        }
    );
}

mod paper_ii_on_93 {
    use super::*;
    negative_control!(
        literature_import_printed_m_but_the_exception_list,
        "paper II's misprinted η on N = 93 (RQ-573) makes a sixth mismatch",
        expected = "N = 93 mismatches and is not listed",
        {
            let mut t = rows();
            t[92].eta = t[92].eta_ii.clone().expect("N = 93 carries paper II's η");
            check_printed(&t, &region().m_exceptions);
        }
    );
}

mod listed_value_wrong {
    use super::*;
    negative_control!(
        literature_import_printed_m_but_the_exception_list,
        "an exception listed with a wrong computed value is not the row's",
        expected = "is listed as",
        {
            let mut listed = region().m_exceptions;
            listed[0].computed = "0.3499".to_owned();
            check_printed(&rows(), &listed);
        }
    );
}

/// `round_decimal` rounds half up at the given place and refuses what is not `0.` and digits.
fn check_rounding(cases: &[(&str, usize, Option<&str>)]) {
    for (s, places, want) in cases {
        assert_eq!(
            round_decimal(s, *places).as_deref(),
            *want,
            "round_decimal({s}, {places})"
        );
    }
}

#[test]
fn literature_import_printed_m_rounds_half_up() {
    check_rounding(&[
        ("0.516951804", 4, Some("0.5170")),
        ("0.349994402", 4, Some("0.3500")),
        ("0.033925931", 4, Some("0.0339")),
        ("0.12345", 4, Some("0.1235")),
        ("0.12344999", 4, Some("0.1234")),
        ("0.99996", 4, Some("1.0000")),
        ("0.1234", 4, None),
        ("1.5", 4, None),
        ("0.12a45", 4, None),
        ("0.7311230217", 4, Some("0.7311")),
    ]);
}

negative_control!(
    literature_import_printed_m_rounds_half_up,
    "0.12345 rounded half up is 0.1235, not 0.1234",
    expected = "round_decimal(0.12345, 4)",
    check_rounding(&[("0.12345", 4, Some("0.1234"))])
);

// ── (ii) Energy by eq. 4 ──────────────────────────────────────────────────────────────────────────────────────────

/// Eq. 4 on every row is the start's own energy, `K + U` from its masses, positions and velocities, within the
/// tolerance.
fn check_eq4(d: &RegionD, rows: &[Row]) {
    for r in rows {
        let s = d.start("eq. 4", r.xi(), r.eta()).expect("in region D");
        let own = invariants(s.g, s.m, s.r, s.v).e;
        let eq4 = energy_eq4(d, r.xi(), r.eta());
        let eps = ((eq4 - own) / own).abs() / f64::EPSILON;
        assert!(
            eps <= TOL,
            "N = {}: eq. 4 gives {eq4:e}, the start's energy is {own:e} ({eps:.1} ε)",
            r.n
        );
    }
}

#[test]
fn literature_import_energy_by_eq4() {
    check_eq4(&region(), &rows());
}

negative_control!(
    literature_import_energy_by_eq4,
    "eq. 4 without K₀ is not the near-rest start's energy",
    expected = "the start's energy is",
    {
        let d = region();
        let mut no_k0 = d.clone();
        no_k0.k0 = 0.0;
        for r in rows() {
            let s = d.start("eq. 4", r.xi(), r.eta()).expect("in region D");
            let own = invariants(s.g, s.m, s.r, s.v).e;
            let eq4 = energy_eq4(&no_k0, r.xi(), r.eta());
            let eps = ((eq4 - own) / own).abs() / f64::EPSILON;
            assert!(
                eps <= TOL,
                "N = {}: eq. 4 gives {eq4:e}, the start's energy is {own:e}",
                r.n
            );
        }
    }
);

// ── (iii) The exact-rest control ──────────────────────────────────────────────────────────────────────────────────

/// The exact-rest fixture: `G` and the velocities.
#[derive(serde::Deserialize)]
struct Rest {
    citation: String,
    g: f64,
    v: [[f64; 2]; 3],
    configurations: String,
}

/// At exact rest, on every row's configuration, `L_z` is `+0` bitwise and `E == U`, on the start and on its canonical
/// decode `D(z)`.
fn check_rest(v: [[f64; 2]; 3], g: f64) {
    let d = region();
    for r in rows() {
        let mut s = d.start("rest", r.xi(), r.eta()).expect("in region D");
        (s.v, s.g) = (v, g);
        let own = invariants(s.g, s.m, s.r, s.v);
        let u = validation::import::potential(s.g, s.m, s.r);
        let i = import(&s).expect("imports");
        for (what, inv, pot) in [
            ("the start", own, u),
            (
                "D(z)",
                i.canonical,
                validation::import::potential(1.0, i.ingested.decoded.m, i.ingested.decoded.r),
            ),
        ] {
            assert_eq!(
                inv.lz.to_bits(),
                0.0f64.to_bits(),
                "N = {}: {what}'s L_z = {:e} is not +0",
                r.n,
                inv.lz
            );
            assert_eq!(inv.e, pot, "N = {}: {what}'s E is not U", r.n);
        }
    }
}

#[test]
fn literature_import_exact_rest_control() {
    let rest: Rest =
        serde_json::from_str(&read("fixtures/anosova/exact_rest.json")).expect("exact_rest.json");
    assert_eq!(rest.v, [[0.0; 2]; 3], "the exact-rest fixture's velocities");
    assert!(
        rest.citation.contains("arXiv:1805.07980"),
        "the exact-rest fixture cites Li & Liao"
    );
    assert!(
        rest.configurations.contains("table1.csv"),
        "the exact-rest fixture takes Table 1's configurations"
    );
    check_rest(rest.v, rest.g);
}

negative_control!(
    literature_import_exact_rest_control,
    "eq. 3's near-rest velocities are not exact rest: L_z is η·10⁻⁴",
    expected = "is not +0",
    check_rest(region().v, region().g)
);

// ── (iv) Through the chart ────────────────────────────────────────────────────────────────────────────────────────

/// Every row, imported at its `η` used, multiplied back as `back`, gives eq. 4's `E`, `η·10⁻⁴` and the scale-free
/// invariant within the tolerance.
fn check_through(back: Back) {
    let d = region();
    for r in rows() {
        let t = region_d_report(&d, &format!("N = {}", r.n), r.xi(), r.eta(), back)
            .expect("in region D");
        for c in &t.compared {
            assert!(
                c.eps <= TOL,
                "N = {}: {} through the chart is {:e}, the paper's {:e} ({:.1} ε)",
                r.n,
                c.what,
                c.imported,
                c.paper,
                c.eps
            );
        }
        assert_eq!(
            t.compared.len(),
            3,
            "N = {}: E, L_z and the scale-free invariant",
            r.n
        );
    }
}

#[test]
fn literature_import_region_d_through_the_chart() {
    check_through(Back::Recorded);
}

negative_control!(
    literature_import_region_d_through_the_chart,
    "multiplying back by the length factor alone, without M and G, does not give the paper's E",
    expected = "E through the chart is",
    check_through(Back::LengthOnly)
);

// ── (v) Outside region D ──────────────────────────────────────────────────────────────────────────────────────────

/// Each point is refused as outside region D.
fn check_outside(points: &[(f64, f64)]) {
    let d = region();
    for &(xi, eta) in points {
        let e = d
            .start("outside", xi, eta)
            .expect_err("a point outside region D must be refused");
        assert!(
            e.contains("outside region D"),
            "the refusal names region D: {e}"
        );
        assert!(!in_region_d(xi, eta), "({xi}, {eta}) is in region D");
    }
}

#[test]
fn literature_import_outside_region_d_refused() {
    let r88 = &rows()[87];
    let paper_i: f64 = r88.eta_i.parse().expect("a number");
    check_outside(&[
        (r88.xi(), paper_i),
        (0.6, 0.6),
        (-1e-9, 0.5),
        (0.25, -1e-9),
        (0.5 + 1e-9, 0.0),
    ]);
    // The edges themselves are in region D (eq. 8's ≤, and ξ, η ≥ 0).
    for (xi, eta) in [(0.0, 0.0), (0.5, 0.0), (0.0, 0.75f64.sqrt()), (0.0, 0.5)] {
        assert!(in_region_d(xi, eta), "({xi}, {eta}) is on region D's edge");
    }
}

negative_control!(
    literature_import_outside_region_d_refused,
    "a point inside region D, N = 1's, is not refused",
    expected = "a point outside region D must be refused",
    check_outside(&[(0.152107965, 0.516951804)])
);

// ── Column 8, recorded ────────────────────────────────────────────────────────────────────────────────────────────

/// `col8_comparison.csv` is the comparison of column 8 with eq. 4 on every row, as the code computes it: a record,
/// not a judgement of column 8 (RQ-570).
fn check_column8(record: &str) {
    let want = column8_csv(&region(), &rows());
    assert_eq!(
        record, want,
        "col8_comparison.csv is not the comparison; regenerated:\n{want}"
    );
}

#[test]
fn literature_import_column8_recorded() {
    let record = read("fixtures/anosova/col8_comparison.csv");
    assert_eq!(record.lines().count(), 101, "a header and a line per row");
    check_column8(&record);
}

negative_control!(
    literature_import_column8_recorded,
    "a record with one row's eq. 4 changed is not the comparison",
    expected = "is not the comparison",
    check_column8(&read("fixtures/anosova/col8_comparison.csv").replacen(
        "1,-2.4640",
        "1,-2.4641",
        1
    ))
);

// ── The Burrau constants (REQ-VAL-174) ────────────────────────────────────────────────────────────────────────────

fn burrau(mass: &str, hyper: &str, energy: &str) -> Printed {
    Printed {
        start: Start {
            name: "Burrau rest start".to_owned(),
            citation: "dd_validation_orbits §2".to_owned(),
            g: 1.0,
            m: [3.0, 4.0, 5.0],
            r: [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]],
            v: [[0.0; 2]; 3],
        },
        mass: mass.to_owned(),
        hyper: hyper.to_owned(),
        energy: energy.to_owned(),
    }
}

/// The imported Burrau rest start, multiplied back, reads `M = 12`, `R = 2.2361` and `E = −12.8167` to their printed
/// precision, and its `E`, `L_z` and `R` match the start's own within the tolerance.
fn check_burrau(p: &Printed) {
    let t = printed_report(p, Back::Recorded).expect("imports");
    assert!(
        t.misprinted.is_empty(),
        "the Burrau constants do not read back: {:?}",
        t.misprinted
    );
    for c in &t.compared {
        assert!(
            c.eps <= TOL,
            "{} reads {:e}, the start's {:e}",
            c.what,
            c.imported,
            c.paper
        );
    }
    assert_eq!(
        t.imported.back.e,
        t.imported.rescale.energy(t.imported.canonical.e)
    );
}

#[test]
fn literature_import_burrau_constants() {
    check_burrau(&burrau("12", "2.2361", "-12.8167"));
}

negative_control!(
    literature_import_burrau_constants,
    "E = −12.8168 is not what the Burrau start reads back",
    expected = "do not read back",
    check_burrau(&burrau("12", "2.2361", "-12.8168"))
);

// ── The gate ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// `literature-import`'s three inputs give their expected outcomes: `literature.json` passes, `length_only.json`
/// fails and `outside_d.json` is refused.
fn check_gate(dir: &str, expect: &[(&str, &str)]) {
    use validation::gate::{config, Gate as _};
    let cfg = config(root(), "literature-import").expect("gate.json");
    for (input, want) in expect {
        let text = read(&format!("{dir}/{input}"));
        let mut v: serde_json::Value = serde_json::from_str(&text).expect("json");
        v.as_object_mut().expect("an object").remove("expected");
        let got = match validation::import::LiteratureImportGate.run(&cfg, input, &v) {
            Err(_) => "refused",
            Ok(r) if r.verdict == validation::gate::Verdict::Pass => "pass",
            Ok(_) => "fail",
        };
        assert_eq!(got, *want, "the gate's outcome on {input}");
    }
}

#[test]
fn literature_import_gate_inputs() {
    check_gate(
        "fixtures/gates/literature-import",
        &[
            ("literature.json", "pass"),
            ("length_only.json", "fail"),
            ("outside_d.json", "refused"),
        ],
    );
}

negative_control!(
    literature_import_gate_inputs,
    "the length-only input does not pass",
    expected = "the gate's outcome on length_only.json",
    check_gate(
        "fixtures/gates/literature-import",
        &[("length_only.json", "pass")]
    )
);

/// Each input's negative results: the length-only input says it is multiplied back by the length factor alone and
/// must fail (PIT-3); the recorded one says nothing of it.
fn check_negative_note(dir: &str, input: &str, noted: bool) {
    use validation::gate::{config, Gate as _};
    let cfg = config(root(), "literature-import").expect("gate.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&read(&format!("{dir}/{input}"))).expect("json");
    v.as_object_mut().expect("an object").remove("expected");
    let r = LiteratureImportGate.run(&cfg, input, &v).expect("measured");
    assert_eq!(r.gate, "literature-import", "the report's gate");
    let has = r.negative_results.iter().any(|n| n.contains("PIT-3"));
    assert_eq!(
        has, noted,
        "{input}'s negative results {:?} on the length-only rescale",
        r.negative_results
    );
}

#[test]
fn literature_import_gate_notes_the_length_only_input() {
    use validation::gate::Gate as _;
    assert_eq!(LiteratureImportGate.name(), "literature-import");
    let dir = "fixtures/gates/literature-import";
    check_negative_note(dir, "length_only.json", true);
    check_negative_note(dir, "literature.json", false);
}

negative_control!(
    literature_import_gate_notes_the_length_only_input,
    "the recorded input does not carry the length-only note",
    expected = "on the length-only rescale",
    check_negative_note("fixtures/gates/literature-import", "literature.json", true)
);

// ── The pieces, on hand-checked values ───────────────────────────────────────────────────────────────────────────

/// `invariants` on a state worked by hand: masses `(1, 2, 3)`, `G = 2`, positions `(1, 0)`, `(0, 2)`, `(−2, −1)`,
/// velocities `(1, 2)`, `(−1, ½)`, `(½, −1)`, the centre of mass at `(−⅚, ⅙)`. `T = 5.625`, `L_z = 13.5`; `U` and `R`
/// to 30 digits (mpmath): `U = −7.01442215545223408843672546239`, `R = 1.81046341520003595398191556246`. Body 3's
/// `x = −2` keeps `m₃·x₃` apart from `m₃/x₃` in the centre of mass.
fn check_by_hand(g: f64) {
    let got = invariants(
        g,
        [1.0, 2.0, 3.0],
        [[1.0, 0.0], [0.0, 2.0], [-2.0, -1.0]],
        [[1.0, 2.0], [-1.0, 0.5], [0.5, -1.0]],
    );
    for (what, got, want) in [
        ("E", got.e, 5.625 - 7.014_422_155_452_234),
        ("L_z", got.lz, 13.5),
        ("R", got.hyper, 1.810_463_415_200_036),
    ] {
        assert!(
            relative(got, want) <= 4.0 * f64::EPSILON,
            "{what} of the hand-worked state is {got:e}, by hand {want:e}"
        );
    }
}

#[test]
fn literature_import_invariants_by_hand() {
    check_by_hand(2.0);
}

negative_control!(
    literature_import_invariants_by_hand,
    "G = 1 is not the hand-worked state's energy",
    expected = "E of the hand-worked state",
    check_by_hand(1.0)
);

/// `unit_state` refuses a start with a mass or `G` that is zero, negative or not finite, or a coordinate that is not
/// finite; and scales a good one to `Σmᵢ = 1`, `G = 1`, `pᵢ = (mᵢ/M)·vᵢ/√(G·M)`.
fn check_refusals(bad: &[(&str, Start)]) {
    for (why, s) in bad {
        let e = unit_state(s).expect_err(why);
        assert!(e.contains("must be positive"), "{why}: {e}");
    }
}

fn good() -> Start {
    Start {
        name: "good".to_owned(),
        citation: "by hand".to_owned(),
        g: 4.0,
        m: [1.0, 1.0, 2.0],
        r: [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]],
        v: [[4.0, 0.0], [0.0, -8.0], [1.0, 1.0]],
    }
}

fn bad_starts() -> Vec<(&'static str, Start)> {
    let with = |f: &dyn Fn(&mut Start)| {
        let mut s = good();
        f(&mut s);
        s
    };
    vec![
        ("a zero mass is refused", with(&|s| s.m[1] = 0.0)),
        ("a negative mass is refused", with(&|s| s.m[2] = -1.0)),
        (
            "an infinite mass is refused",
            with(&|s| s.m[0] = f64::INFINITY),
        ),
        ("G = 0 is refused", with(&|s| s.g = 0.0)),
        ("an infinite G is refused", with(&|s| s.g = f64::INFINITY)),
        ("a NaN position is refused", with(&|s| s.r[2][1] = f64::NAN)),
        (
            "an infinite velocity is refused",
            with(&|s| s.v[0][0] = f64::INFINITY),
        ),
    ]
}

#[test]
fn literature_import_unit_state_refuses_and_scales() {
    check_refusals(&bad_starts());
    let x = unit_state(&good()).expect("a good start");
    assert_eq!(x.m, [0.25, 0.25, 0.5], "the masses over M = 4");
    assert_eq!(x.r, good().r, "the positions unchanged");
    // √(G·M) = 4: p₁ = ¼·(4, 0)/4, p₂ = ¼·(0, −8)/4, p₃ = ½·(1, 1)/4.
    assert_eq!(
        x.p,
        [[0.25, 0.0], [0.0, -0.5], [0.125, 0.125]],
        "the momenta"
    );
}

negative_control!(
    literature_import_unit_state_refuses_and_scales,
    "a good start is not refused",
    expected = "a good start is refused",
    check_refusals(&[("a good start is refused", good())])
);

/// The recorded rescale's factors on hand values, `M = 4`, `G = 9`, `λ = ¼`: `E = G·M²·λ·E_c = 36·E_c`, `L_z =
/// ±√G·M^{3/2}·λ^{−1/2}·L_{z,c} = ±48·L_{z,c}`, minus where mirrored, `R = 1/λ = 4`; the length-only rescale keeps
/// `λ` and the mirror and sets `M = G = 1`.
fn check_rescale(mirrored: bool, sign: f64) {
    let r = Rescale {
        mass: 4.0,
        g: 9.0,
        lambda: 0.25,
        mirrored,
    };
    assert_eq!(r.energy(1.0), 36.0, "E = G·M²·λ·E_c");
    assert_eq!(
        r.lz(1.0),
        sign * 48.0,
        "L_z = ±√G·M^{{3/2}}·λ^{{−1/2}}·L_z,c"
    );
    assert_eq!(r.hyper(), 4.0, "R = 1/λ");
    assert_eq!(
        r.length_only(),
        Rescale {
            mass: 1.0,
            g: 1.0,
            lambda: 0.25,
            mirrored
        },
        "the length-only rescale"
    );
}

#[test]
fn literature_import_rescale_factors() {
    check_rescale(false, 1.0);
    check_rescale(true, -1.0);
}

negative_control!(
    literature_import_rescale_factors,
    "a mirrored start's L_z keeps its sign",
    expected = "L_z = ±√G",
    check_rescale(true, 1.0)
);

/// `relative` is `|got − want|/|want|`, and `|got|` where `want` is 0.
fn check_relative(cases: &[(f64, f64, f64)]) {
    for &(got, want, r) in cases {
        assert_eq!(relative(got, want), r, "relative({got}, {want})");
    }
}

#[test]
fn literature_import_relative_difference() {
    check_relative(&[
        (3.0, 2.0, 0.5),
        (1.0, -4.0, 1.25),
        (-3.0, 0.0, 3.0),
        (7.0, 7.0, 0.0),
    ]);
}

negative_control!(
    literature_import_relative_difference,
    "the relative difference of 3 from 2 is not the absolute one",
    expected = "relative(3, 2)",
    check_relative(&[(3.0, 2.0, 1.0)])
);

/// A CSV line's fields: a quoted field keeps its commas, and `""` inside it is one `"`.
fn check_fields(line: &str, want: &[&str]) {
    assert_eq!(fields(line), want, "the fields of {line}");
}

#[test]
fn literature_import_csv_fields() {
    check_fields(r#"a,"b, ""c""",d"#, &["a", r#"b, "c""#, "d"]);
    check_fields(r#""",x,"""#, &["", "x", ""]);
    check_fields("1,,2", &["1", "", "2"]);
}

negative_control!(
    literature_import_csv_fields,
    "a quoted comma does not split the field",
    expected = "the fields of",
    check_fields(r#""a,b",c"#, &["a", "b", "c"])
);

/// `check_exceptions` on small lists: the exact list passes; a mismatch not listed, a listed row whose values differ,
/// and a listed row that matches each fail with their own reason.
fn exception(n: u32, computed: &str) -> Exception {
    Exception {
        n,
        printed: "0.1000".to_owned(),
        computed: computed.to_owned(),
    }
}

fn check_exception_cases(cases: &[(Vec<Exception>, Vec<Exception>, Option<&str>)]) {
    for (got, listed, reason) in cases {
        match (check_exceptions(got, listed), reason) {
            (Ok(()), None) => {}
            (Err(e), Some(r)) if e.contains(r) => {}
            (out, _) => panic!("check_exceptions({got:?}, {listed:?}) is {out:?}, not {reason:?}"),
        }
    }
}

#[test]
fn literature_import_exception_check() {
    let (a, b) = (exception(3, "0.1001"), exception(7, "0.1002"));
    check_exception_cases(&[
        (vec![a.clone()], vec![a.clone()], None),
        (vec![a.clone(), b.clone()], vec![b.clone(), a.clone()], None),
        (
            vec![a.clone(), b.clone()],
            vec![a.clone()],
            Some("N = 7 mismatches and is not listed"),
        ),
        (
            vec![a.clone()],
            vec![exception(3, "0.1009")],
            Some("N = 3 is listed as"),
        ),
        (
            vec![a.clone()],
            vec![a.clone(), b],
            Some("N = 7 is listed but its printed M matches"),
        ),
        (vec![], vec![], None),
    ]);
}

negative_control!(
    literature_import_exception_check,
    "a mismatch left off the list passes",
    expected = "check_exceptions(",
    check_exception_cases(&[(vec![exception(3, "0.1001")], vec![], None)])
);

/// `worst` is the largest `eps` compared, NaN where one is NaN; `eps` is the relative difference in units of `2⁻⁵²`;
/// `fails` is `worst > k`, NaN, or a printed value not read back; and the report line names the start, its paper and
/// each quantity. Checked on N = 1 multiplied back by the length factor alone, whose `E` is off by about `G·M²`.
fn check_worst(k_of: impl Fn(f64) -> f64) {
    let d = region();
    let r = &rows()[0];
    let t: Tagged =
        region_d_report(&d, "N = 1", r.xi(), r.eta(), Back::LengthOnly).expect("in region D");
    let max = t.compared.iter().map(|c| c.eps).fold(0.0, f64::max);
    assert!(
        max > 1e12,
        "the length-only E is off by {max:e} ε, not about G·M²"
    );
    for c in &t.compared {
        assert_eq!(
            c.eps,
            relative(c.imported, c.paper) / f64::EPSILON,
            "{}'s ε",
            c.what
        );
    }
    assert_eq!(t.worst(), max, "worst is the largest ε");
    let k = k_of(max);
    assert!(!fails(&t, k), "a start at its threshold, {k:e} ε, fails");
    assert!(
        fails(&t, max.next_down()),
        "a start past its threshold passes"
    );
    let mut nan = t.clone();
    nan.compared[1].eps = f64::NAN;
    assert!(nan.worst().is_nan(), "a NaN ε is not the worst");
    assert!(fails(&nan, f64::INFINITY), "a NaN ε passes");
    let mut misprinted = t.clone();
    misprinted
        .misprinted
        .push("E reads 1, printed 2".to_owned());
    assert!(
        fails(&misprinted, f64::INFINITY),
        "a misprinted value passes"
    );
    let line = t.line();
    for part in ["N = 1", &t.citation, "κ =", "E ", "L_z ", "L_z²|E|/(G²M⁵)"] {
        assert!(
            line.contains(part),
            "the report line {line} does not name {part}"
        );
    }
}

#[test]
fn literature_import_worst_and_fails() {
    check_worst(|max| max);
}

negative_control!(
    literature_import_worst_and_fails,
    "a start just past its threshold does not fail",
    expected = "a start at its threshold",
    check_worst(f64::next_down)
);
