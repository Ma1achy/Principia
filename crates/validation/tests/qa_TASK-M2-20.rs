//! QA tests for TASK-M2-20 (REQ-VAL-019, REQ-VAL-120, REQ-VAL-174), written from R-592 (the human's RQ-566, RQ-571
//! and RQ-573 rulings), dd_validation_orbits §1.6 and §2, and the requirements' verify details, not from the import's
//! code. The physics is this file's own: `L_z`, `E`, eq. 4, eq. 8, the rounding of column 11 and the multiplying back
//! are computed here from the definition and compared with what the import and the `literature-import` gate return.
//!
//! - Table 1 (`fixtures/anosova/table1.csv`), read by this file's own CSV reader: every row `N = 1 … 100`, in order,
//!   each with its page of paper I (pp. 1010–1012, as the scan splits them) and its outcome text.
//! - Which `η` each row uses: paper II's only on `N = 88`; paper I's on `N = 26, 61, 69, 77, 93`, paper II's value
//!   recorded beside it (R-592's table); on every other row the papers agree.
//! - Which `ξ` each row uses: paper I's on every row, paper II's recorded in the note on exactly `N = 35, 49, 54, 65`
//!   where the papers differ; `N = 17`'s broken glyph settled as `0.115279340`, the other reading recorded (R-592).
//! - (i) `M = Σ m(xẏ − yẋ)` with eq. 3's velocities equals `η·10⁻⁴` on every row, and the printed column 11 matches
//!   it to four decimals on every row printing one except exactly `N = 16, 23, 26, 32, 34`; through the gate, a new
//!   mismatch fails, a listed row that starts matching fails, and paper II's `η` on row 93 fails.
//! - (ii) `E` by eq. 4 equals the start's own `½Σmv² − GΣmᵢmⱼ/rᵢⱼ`, and is what the gate compares with.
//! - (iii) The exact-rest control: zero velocities, so `L_z == 0` and `E == U` exactly, on the start and on `D(z)`.
//! - (iv) Through the chart: the canonical `D(z)` has its CoM at the origin, `Σmᵢ = 1`, `I = 1`, `Σpᵢ = 0`; its `E`
//!   and `L_z`, multiplied back by this file's own `(M, G, λ)` and mirror, match eq. 4 and `η·10⁻⁴` within
//!   `literature_import_tol`; `L_z²·|E|/(G²·M⁵)` matches with no factor.
//! - (v) Points outside region D (eq. 8, `ξ < 0`, `η < 0`) are refused by the gate.
//! - Column 8 is recorded beside eq. 4 on every row (`col8_comparison.csv`), with RQ-566's examples, and the gate's
//!   verdict does not depend on it (RQ-570 open).
//! - The Burrau rest start reads back `M = 12`, `R = 2.2361`, `E = −12.8167` (REQ-VAL-174).
//!
//! Each test has its negative control (R-176), failing for its stated reason.

#[path = "support/scratch.rs"]
mod scratch;

use std::path::{Path, PathBuf};

use kernel::decode::Ic;
use ledger::constants::LITERATURE_IMPORT_TOL;
use validation::gate::{config, Gate as _, Verdict};
use validation::import::{import, LiteratureImportGate, RegionD, Start};
use validation::negative_control;

use scratch::Scratch;

/// R-592's exception list: the rows whose printed column 11 is not `η·10⁻⁴` (RQ-571's ruling).
const EXCEPTIONS: [u32; 5] = [16, 23, 26, 32, 34];

/// dd_validation_orbits §1.6's constants: `G` in the papers' units, eq. 3's velocities and `K₀`.
const G: f64 = 6.67e-8;
const V3: [[f64; 2]; 3] = [[5e-5, 0.0], [5e-5, 0.0], [-1e-4, 0.0]];
const K0: f64 = 7.5e-9;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(p: &str) -> String {
    std::fs::read_to_string(root().join(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// The register's `literature_import_tol`, `k` in `k·ε` relative (REQ-VAL-120, provisional until M2).
fn tol() -> f64 {
    LITERATURE_IMPORT_TOL.number() * f64::EPSILON
}

// ── This file's own reading of Table 1 ───────────────────────────────────────────────────────────────────────────

/// One CSV line's fields: commas separate, double quotes enclose a field that may hold commas, `""` is a quote.
fn csv_fields(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut quoted = false;
    let mut prev_quote = false;
    for c in line.chars() {
        if c == '"' {
            if quoted && prev_quote {
                out.last_mut().unwrap().push('"');
                prev_quote = false;
                continue;
            }
            quoted = !quoted;
            prev_quote = !quoted;
            continue;
        }
        prev_quote = false;
        if c == ',' && !quoted {
            out.push(String::new());
        } else {
            out.last_mut().unwrap().push(c);
        }
    }
    out
}

#[derive(Clone, Debug)]
struct QaRow {
    page: u32,
    n: u32,
    xi: String,
    eta_i: String,
    eta_ii: String,
    eta_used: String,
    source: String,
    col8: String,
    col11: String,
    outcome: String,
    note: String,
}

impl QaRow {
    fn xi(&self) -> f64 {
        self.xi.parse().expect("ξ")
    }
    fn eta(&self) -> f64 {
        self.eta_used.parse().expect("η")
    }
}

fn qa_rows(text: &str) -> Vec<QaRow> {
    let mut lines = text.lines();
    let header = csv_fields(lines.next().expect("a header"));
    let col = |name: &str| {
        header
            .iter()
            .position(|h| h == name)
            .unwrap_or_else(|| panic!("table1.csv has no column {name}"))
    };
    let c = [
        "page",
        "N",
        "xi",
        "eta_I",
        "eta_II",
        "eta_used",
        "eta_source",
        "E_col8_e7",
        "M_col11_e4",
        "outcome",
        "note",
    ]
    .map(col);
    lines
        .map(|l| {
            let f = csv_fields(l);
            assert_eq!(f.len(), header.len(), "table1.csv line {l:?}");
            QaRow {
                page: f[c[0]].parse().expect("page"),
                n: f[c[1]].parse().expect("N"),
                xi: f[c[2]].clone(),
                eta_i: f[c[3]].clone(),
                eta_ii: f[c[4]].clone(),
                eta_used: f[c[5]].clone(),
                source: f[c[6]].clone(),
                col8: f[c[7]].clone(),
                col11: f[c[8]].clone(),
                outcome: f[c[9]].clone(),
                note: f[c[10]].clone(),
            }
        })
        .collect()
}

fn table_text() -> String {
    read("fixtures/anosova/table1.csv")
}

fn rows() -> Vec<QaRow> {
    qa_rows(&table_text())
}

/// Table 1 with row `n`'s field `column` set to `value`, as CSV text.
fn table_with(n: u32, column: &str, value: &str) -> String {
    let text = table_text();
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let header = csv_fields(&lines[0]);
    let k = header.iter().position(|h| h == column).expect("the column");
    let mut hit = false;
    for line in lines.iter_mut().skip(1) {
        let mut f = csv_fields(line);
        if f[1] == n.to_string() {
            f[k] = value.to_owned();
            hit = true;
            *line = f
                .iter()
                .map(|x| {
                    if x.contains(',') || x.contains('"') {
                        format!("\"{}\"", x.replace('"', "\"\""))
                    } else {
                        x.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
        }
    }
    assert!(hit, "row {n} is in the table");
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

// ── This file's own physics ──────────────────────────────────────────────────────────────────────────────────────

fn positions(xi: f64, eta: f64) -> [[f64; 2]; 3] {
    [[-0.5, 0.0], [0.5, 0.0], [xi, eta]]
}

fn lz(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> f64 {
    (0..3)
        .map(|i| m[i] * (r[i][0] * v[i][1] - r[i][1] * v[i][0]))
        .sum()
}

fn kinetic(m: [f64; 3], v: [[f64; 2]; 3]) -> f64 {
    (0..3)
        .map(|i| 0.5 * m[i] * (v[i][0] * v[i][0] + v[i][1] * v[i][1]))
        .sum()
}

fn potential(g: f64, m: [f64; 3], r: [[f64; 2]; 3]) -> f64 {
    let mut u = 0.0;
    for i in 0..3 {
        for j in i + 1..3 {
            let d = ((r[i][0] - r[j][0]).powi(2) + (r[i][1] - r[j][1]).powi(2)).sqrt();
            u -= g * m[i] * m[j] / d;
        }
    }
    u
}

/// Eq. 4, as the paper writes it: `E = −G(1 + 1/√((ξ + ½)² + η²) + 1/√((ξ − ½)² + η²)) + K₀`.
fn eq4(xi: f64, eta: f64) -> f64 {
    -G * (1.0
        + 1.0 / ((xi + 0.5).powi(2) + eta * eta).sqrt()
        + 1.0 / ((xi - 0.5).powi(2) + eta * eta).sqrt())
        + K0
}

/// Eq. 8 with `ξ ≥ 0`, `η ≥ 0`.
fn in_d(xi: f64, eta: f64) -> bool {
    xi >= 0.0 && eta >= 0.0 && (xi + 0.5).powi(2) + eta * eta <= 1.0
}

fn rel(a: f64, b: f64) -> f64 {
    ((a - b) / b).abs()
}

/// The computed column 11 for `η`: `M = Σ m(xẏ − yẋ)` from eq. 3, in units of 10⁻⁴, to four decimals.
fn computed_col11(xi: f64, eta: f64) -> String {
    format!("{:.4}", lz([1.0; 3], positions(xi, eta), V3) * 1e4)
}

fn start(name: &str, xi: f64, eta: f64, g: f64, v: [[f64; 2]; 3]) -> Start {
    Start {
        name: name.to_owned(),
        citation: "Agekyan & Anosova (1968), Table 1 (qa)".to_owned(),
        g,
        m: [1.0; 3],
        r: positions(xi, eta),
        v,
    }
}

// ── The gate, run on an edited input ─────────────────────────────────────────────────────────────────────────────

/// `literature.json` with its Table 1 replaced by `table` (written to a scratch file) and its extra points `points`;
/// the gate's outcome: "pass", "fail" or the refusal, and the report's scatter.
fn gate_on(table: Option<&str>, points: &[[f64; 2]]) -> (String, Vec<String>) {
    let cfg = config(&root(), "literature-import").expect("gate.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&read("fixtures/gates/literature-import/literature.json"))
            .expect("json");
    let o = v.as_object_mut().expect("an object");
    o.remove("expected");
    let dir = Scratch::new("qa-m2-20");
    if let Some(t) = table {
        std::fs::create_dir_all(&dir).expect("scratch");
        let path = dir.join("table1.csv");
        std::fs::write(&path, t).expect("write the table");
        o.insert("table1".into(), path.to_string_lossy().into_owned().into());
    }
    o.insert(
        "points".into(),
        serde_json::to_value(points).expect("points"),
    );
    match LiteratureImportGate.run(&cfg, "literature.json", &v) {
        Err(e) => (format!("refused: {e}"), Vec::new()),
        Ok(r) if r.verdict == Verdict::Pass => ("pass".to_owned(), r.scatter),
        Ok(r) => ("fail".to_owned(), r.scatter),
    }
}

// ── Table 1: every row, with its page ────────────────────────────────────────────────────────────────────────────

/// Every row `N = 1 … 100` in order, each with its page (1010 for 1–52, 1011 for 53–96, 1012 for 97–100), its `ξ`, its
/// `η` and its outcome text; rows printing "—" in columns 8 and 11 and the "ΔE/E > 0.05" rows kept.
fn check_every_row(rows: &[QaRow]) {
    let ns: Vec<u32> = rows.iter().map(|r| r.n).collect();
    assert_eq!(
        ns,
        (1..=100).collect::<Vec<u32>>(),
        "Table 1's rows N = 1 … 100"
    );
    for r in rows {
        let page = match r.n {
            1..=52 => 1010,
            53..=96 => 1011,
            _ => 1012,
        };
        assert_eq!(r.page, page, "N = {}: its page of paper I", r.n);
        assert!(!r.outcome.is_empty(), "N = {}: its outcome", r.n);
        assert!(
            !r.xi.is_empty() && !r.eta_i.is_empty(),
            "N = {}: ξ and η",
            r.n
        );
    }
    let dashes = rows.iter().filter(|r| r.col11 == "—").count();
    let flagged = rows
        .iter()
        .filter(|r| r.outcome.contains("ΔE/E > 0.05"))
        .count();
    assert!(
        dashes > 0 && flagged > 0,
        "the \"—\" and ΔE/E > 0.05 rows are kept"
    );
    assert!(
        rows.iter()
            .all(|r| r.col11 == "—" || r.col11.parse::<f64>().is_ok()),
        "column 11 is a number or \"—\""
    );
}

#[test]
fn qa_m2_20_table1_keeps_every_row_with_its_page() {
    check_every_row(&rows());
}

negative_control!(
    qa_m2_20_table1_keeps_every_row_with_its_page,
    "a table with a \"—\" row dropped does not keep every row",
    expected = "Table 1's rows N = 1 … 100",
    check_every_row(&rows().into_iter().filter(|r| r.n != 11).collect::<Vec<_>>())
);

// ── Which η each row uses ────────────────────────────────────────────────────────────────────────────────────────

/// R-592's six rows where the papers differ: `N`, paper I's `η`, paper II's, and the source used.
const DIFFER: [(u32, &str, &str, &str); 6] = [
    (26, "0.760908567", "0.760908565", "I"),
    (61, "0.150592949", "0.150593949", "I"),
    (69, "0.238608305", "0.238608354", "I"),
    (77, "0.032869382", "0.032869383", "I"),
    (88, "0.7311230217", "0.311230217", "II"),
    (93, "0.471972279", "0.471612279", "I"),
];

fn check_eta_used(rows: &[QaRow]) {
    for r in rows {
        match DIFFER.iter().find(|d| d.0 == r.n) {
            Some(&(n, i, ii, used)) => {
                assert_eq!(
                    (r.eta_i.as_str(), r.eta_ii.as_str()),
                    (i, ii),
                    "N = {n}: both papers' η"
                );
                assert_eq!(r.source, used, "N = {n}: the η used is paper {used}'s");
                let want = if used == "I" { i } else { ii };
                assert_eq!(r.eta_used, want, "N = {n}: the η used is paper {used}'s");
                assert!(!r.note.is_empty(), "N = {n}: the reason is recorded");
            }
            None => {
                assert!(r.eta_ii.is_empty(), "N = {}: the papers agree", r.n);
                assert_eq!(
                    (r.source.as_str(), &r.eta_used),
                    ("I", &r.eta_i),
                    "N = {}: paper I's η",
                    r.n
                );
            }
        }
        assert!(
            in_d(r.xi(), r.eta()),
            "N = {}: the η used puts A₃ in region D",
            r.n
        );
    }
    let r93 = rows.iter().find(|r| r.n == 93).expect("row 93");
    assert!(
        r93.note.contains("misprint"),
        "row 93 labels paper II's value its misprint"
    );
    let r88 = rows.iter().find(|r| r.n == 88).expect("row 88");
    assert!(
        !in_d(r88.xi(), r88.eta_i.parse().unwrap()),
        "paper I's η on row 88 lies outside region D"
    );
}

#[test]
fn qa_m2_20_eta_used_is_paper_twos_only_on_row_88() {
    check_eta_used(&rows());
}

negative_control!(
    qa_m2_20_eta_used_is_paper_twos_only_on_row_88,
    "row 93 taking paper II's η is not R-592's choice",
    expected = "N = 93: the η used is paper I's",
    check_eta_used(&qa_rows(&table_with(93, "eta_used", "0.471612279")))
);

// ── (i) Angular momentum ─────────────────────────────────────────────────────────────────────────────────────────

/// The fixture's definition is §1.6's; on every row `M = Σ m(xẏ − yẋ) = η·10⁻⁴`, by this file's sum and by the
/// import's start; and eq. 3's velocities carry no momentum and `K₀ = 7.5·10⁻⁹`.
fn check_m(v: [[f64; 2]; 3]) {
    let d = RegionD::parse(&read("fixtures/anosova/region_d.json")).expect("region_d.json");
    assert_eq!(
        (d.g, d.m, d.a, d.v, d.k0),
        (G, [1.0; 3], [[-0.5, 0.0], [0.5, 0.0]], V3, K0),
        "§1.6's definition"
    );
    for r in rows() {
        let want = r.eta() * 1e-4;
        let own = lz([1.0; 3], positions(r.xi(), r.eta()), v);
        assert!(
            rel(own, want) <= tol(),
            "N = {}: M = {own:e}, η·10⁻⁴ = {want:e}",
            r.n
        );
        let s = d.start("qa", r.xi(), r.eta()).expect("in region D");
        assert!(
            rel(lz(s.m, s.r, s.v), want) <= tol(),
            "N = {}: the import's start's M is not η·10⁻⁴",
            r.n
        );
    }
    let p: [f64; 2] = [0, 1].map(|k| v.iter().map(|vi| vi[k]).sum());
    assert_eq!(p, [0.0, 0.0], "eq. 3's total momentum");
    assert!(rel(kinetic([1.0; 3], v), K0) <= tol(), "eq. 3's K₀");
}

#[test]
fn qa_m2_20_angular_momentum_is_eta_e4_on_every_row() {
    check_m(V3);
}

negative_control!(
    qa_m2_20_angular_momentum_is_eta_e4_on_every_row,
    "with A₃'s ẋ sign-flipped, M is −η·10⁻⁴",
    expected = "N = 1: M = ",
    check_m([[5e-5, 0.0], [5e-5, 0.0], [1e-4, 0.0]])
);

/// The rows whose printed column 11 differs from the computed `M` to four decimals, `(N, printed, computed)`.
fn mismatches(rows: &[QaRow]) -> Vec<(u32, String, String)> {
    rows.iter()
        .filter(|r| r.col11 != "—")
        .filter_map(|r| {
            let c = computed_col11(r.xi(), r.eta());
            (c != r.col11).then(|| (r.n, r.col11.clone(), c))
        })
        .collect()
}

/// The printed column 11 matches on every row printing one except exactly R-592's list, and the fixture's list
/// carries each one's printed and computed values; no `η` sits on a rounding tie.
fn check_exception_list(rows: &[QaRow]) {
    for r in rows {
        let scaled = r.eta() * 1e4;
        let frac = (scaled - scaled.floor() - 0.5).abs();
        assert!(frac > 1e-6, "N = {}: η is at a rounding tie", r.n);
    }
    let got = mismatches(rows);
    let ns: Vec<u32> = got.iter().map(|g| g.0).collect();
    assert_eq!(ns, EXCEPTIONS, "the printed-M mismatches are R-592's list");
    let d = RegionD::parse(&read("fixtures/anosova/region_d.json")).expect("region_d.json");
    let listed: Vec<(u32, String, String)> = d
        .m_exceptions
        .iter()
        .map(|e| (e.n, e.printed.clone(), e.computed.clone()))
        .collect();
    assert_eq!(
        listed, got,
        "the fixture's exception list, with printed and computed values"
    );
}

#[test]
fn qa_m2_20_printed_m_matches_but_on_the_exception_list() {
    check_exception_list(&rows());
}

negative_control!(
    qa_m2_20_printed_m_matches_but_on_the_exception_list,
    "a sixth misprinted M is a mismatch not on the list",
    expected = "the printed-M mismatches are R-592's list",
    check_exception_list(&qa_rows(&table_with(1, "M_col11_e4", "0.5171")))
);

/// The gate on Table 1 as `table` gives `want`.
fn check_gate_table(table: &str, want: &str) {
    let (got, scatter) = gate_on(Some(table), &[]);
    assert_eq!(got, want, "the gate's outcome on the edited table");
    if want == "pass" {
        let line = scatter
            .iter()
            .find(|l| l.starts_with("Table 1's printed M"))
            .expect("the exception line");
        assert!(
            line.contains("[16, 23, 26, 32, 34]"),
            "the gate reports the list: {line}"
        );
    }
}

#[test]
fn qa_m2_20_gate_judges_the_exception_list() {
    check_gate_table(&table_text(), "pass");
    // A new mismatch fails.
    check_gate_table(&table_with(1, "M_col11_e4", "0.5171"), "fail");
    // A listed row that starts matching fails.
    check_gate_table(&table_with(32, "M_col11_e4", "0.5680"), "fail");
    // Paper II's η on row 93 is a sixth mismatch.
    check_gate_table(&table_with(93, "eta_used", "0.471612279"), "fail");
}

negative_control!(
    qa_m2_20_gate_judges_the_exception_list,
    "a sixth printed-M mismatch does not pass the gate",
    expected = "the gate's outcome on the edited table",
    check_gate_table(&table_with(1, "M_col11_e4", "0.5171"), "pass")
);

// ── (ii) Energy by eq. 4 ─────────────────────────────────────────────────────────────────────────────────────────

/// Eq. 4 is the start's own energy `½Σmv² − GΣmᵢmⱼ/rᵢⱼ` on every row, and the paper value the gate compares `E` with.
fn check_eq4(k0: f64) {
    let d = RegionD::parse(&read("fixtures/anosova/region_d.json")).expect("region_d.json");
    for r in rows() {
        let (xi, eta) = (r.xi(), r.eta());
        let own = kinetic([1.0; 3], V3) + potential(G, [1.0; 3], positions(xi, eta));
        let e4 = eq4(xi, eta) - K0 + k0;
        assert!(
            rel(e4, own) <= tol(),
            "N = {}: eq. 4 {e4:e} against the start's E {own:e}",
            r.n
        );
        let t = validation::import::region_d_report(
            &d,
            "qa",
            xi,
            eta,
            validation::import::Back::Recorded,
        )
        .expect("in region D");
        let paper = t
            .compared
            .iter()
            .find(|c| c.what == "E")
            .expect("E compared")
            .paper;
        assert!(
            rel(paper, e4) <= tol(),
            "N = {}: the gate's paper E {paper:e}",
            r.n
        );
    }
}

#[test]
fn qa_m2_20_energy_by_eq4() {
    check_eq4(K0);
}

negative_control!(
    qa_m2_20_energy_by_eq4,
    "eq. 4 without K₀ is not the start's energy",
    expected = "against the start's E",
    check_eq4(0.0)
);

// ── (iii) The exact-rest control ─────────────────────────────────────────────────────────────────────────────────

/// At velocities `v` and `G = g`, on every row: the start's `L_z == 0` and `E == U` exactly, and through the chart
/// `D(z)` has every momentum exactly 0, so its `L_z == 0` and `E == U`, and the multiplied-back `L_z == 0`.
fn check_rest(v: [[f64; 2]; 3], g: f64) {
    for r in rows() {
        let s = start("rest", r.xi(), r.eta(), g, v);
        let l = lz(s.m, s.r, s.v);
        assert!(
            l == 0.0 && l.abs().to_bits() == 0,
            "N = {}: the start's L_z = {l:e} is not 0",
            r.n
        );
        let u = potential(g, s.m, s.r);
        assert_eq!(
            kinetic(s.m, s.v) + u,
            u,
            "N = {}: the start's E is not U",
            r.n
        );
        let i = import(&s).expect("imports");
        let x = &i.ingested.decoded;
        assert!(
            x.p.iter().flatten().all(|&p| p == 0.0),
            "N = {}: D(z)'s momenta {:?} are not 0",
            r.n,
            x.p
        );
        let vc: [[f64; 2]; 3] = std::array::from_fn(|j| [x.p[j][0] / x.m[j], x.p[j][1] / x.m[j]]);
        let lc = lz(x.m, x.r, vc);
        assert_eq!(lc, 0.0, "N = {}: D(z)'s L_z", r.n);
        assert_eq!(i.back.lz, 0.0, "N = {}: the multiplied-back L_z", r.n);
        let uc = potential(1.0, x.m, x.r);
        assert_eq!(kinetic(x.m, vc) + uc, uc, "N = {}: D(z)'s E is not U", r.n);
        let uc_import = validation::import::potential(1.0, x.m, x.r);
        assert_eq!(
            i.canonical.e, uc_import,
            "N = {}: the import's E of D(z) is not its U",
            r.n
        );
    }
}

#[test]
fn qa_m2_20_exact_rest_control() {
    let rest: serde_json::Value =
        serde_json::from_str(&read("fixtures/anosova/exact_rest.json")).expect("exact_rest.json");
    let v: [[f64; 2]; 3] = serde_json::from_value(rest["v"].clone()).expect("v");
    let g: f64 = serde_json::from_value(rest["g"].clone()).expect("g");
    assert_eq!(
        (v, g),
        ([[0.0; 2]; 3], 1.0),
        "the exact-rest fixture: zero velocities, G = 1"
    );
    check_rest(v, g);
}

negative_control!(
    qa_m2_20_exact_rest_control,
    "eq. 3's near-rest velocities are not rest: L_z = η·10⁻⁴",
    expected = "the start's L_z",
    check_rest(V3, 1.0)
);

// ── (iv) Through the chart ───────────────────────────────────────────────────────────────────────────────────────

/// Every row imported, at this file's own rescale: the canonical `D(z)` is on the section (CoM 0, `Σm = 1`, `I = 1`,
/// `Σp = 0`); its `E` times `G·M²·λ` and its `L_z` times `±√G·M^{3/2}·λ^{−1/2}` match eq. 4 and `η·10⁻⁴`, and
/// `L_z²|E|` matches `L_z²|E|/(G²M⁵)` with no factor, each within `literature_import_tol`. `mass_power` is the power
/// of `M` taken in the energy's factor (2 by §2).
fn check_through(mass_power: i32) {
    let k = tol();
    for r in rows() {
        let (xi, eta) = (r.xi(), r.eta());
        let s = start(&format!("N = {}", r.n), xi, eta, G, V3);
        let i = import(&s).expect("imports");
        let x: &Ic<f64> = &i.ingested.decoded;
        let msum: f64 = x.m.iter().sum();
        let com = [0, 1].map(|c| (0..3).map(|j| x.m[j] * x.r[j][c]).sum::<f64>());
        let psum = [0, 1].map(|c| (0..3).map(|j| x.p[j][c]).sum::<f64>());
        let inertia: f64 = (0..3)
            .map(|j| x.m[j] * (x.r[j][0].powi(2) + x.r[j][1].powi(2)))
            .sum();
        assert!((msum - 1.0).abs() <= k, "N = {}: Σm = {msum}", r.n);
        let pscale = x.p.iter().flatten().fold(0.0f64, |a, c| a.max(c.abs()));
        assert!(com.iter().all(|c| c.abs() <= k), "N = {}: CoM {com:?}", r.n);
        assert!(
            psum.iter().all(|c| c.abs() <= k * pscale),
            "N = {}: Σp {psum:?}",
            r.n
        );
        assert!((inertia - 1.0).abs() <= k, "N = {}: I = {inertia}", r.n);
        // λ = I_in^{−1/2}, I_in of the unit-mass start about its CoM; positions unchanged (§2).
        let mu = [1.0 / 3.0; 3];
        let c_in = [0, 1].map(|c| (0..3).map(|j| mu[j] * s.r[j][c]).sum::<f64>());
        let i_in: f64 = (0..3)
            .map(|j| mu[j] * ((s.r[j][0] - c_in[0]).powi(2) + (s.r[j][1] - c_in[1]).powi(2)))
            .sum();
        let lambda = i_in.powf(-0.5);
        assert!(
            rel(i.ingested.encoded.canonical.lambda, lambda) <= k,
            "N = {}: λ",
            r.n
        );
        let vc: [[f64; 2]; 3] = std::array::from_fn(|j| [x.p[j][0] / x.m[j], x.p[j][1] / x.m[j]]);
        let ec = kinetic(x.m, vc) + potential(1.0, x.m, x.r);
        let lc = lz(x.m, x.r, vc);
        let m_tot = 3.0f64;
        let sign = if i.ingested.encoded.report.mirrored {
            -1.0
        } else {
            1.0
        };
        let e_back = G * m_tot.powi(mass_power) * lambda * ec;
        let l_back = sign * G.sqrt() * m_tot.powf(1.5) / lambda.sqrt() * lc;
        let (e_paper, l_paper) = (eq4(xi, eta), eta * 1e-4);
        assert!(
            rel(e_back, e_paper) <= k,
            "N = {}: E through the chart {e_back:e}, eq. 4 {e_paper:e}",
            r.n
        );
        assert!(
            rel(l_back, l_paper) <= k,
            "N = {}: L_z through the chart {l_back:e}, η·10⁻⁴ {l_paper:e}",
            r.n
        );
        let free_paper = l_paper * l_paper * e_paper.abs() / (G * G * m_tot.powi(5));
        assert!(
            rel(lc * lc * ec.abs(), free_paper) <= k,
            "N = {}: L_z²|E| through the chart against L_z²|E|/(G²M⁵)",
            r.n
        );
        // And the import's own multiplied-back values agree with these.
        assert!(
            rel(i.back.e, e_paper) <= k && rel(i.back.lz, l_paper) <= k,
            "N = {}: the import's back",
            r.n
        );
        assert!(
            i.ingested.residual.is_finite() && i.ingested.kappa.is_finite(),
            "N = {}: residual and κ",
            r.n
        );
    }
}

#[test]
fn qa_m2_20_through_the_chart_after_the_rescale() {
    assert_eq!(
        LITERATURE_IMPORT_TOL.number(),
        256.0,
        "the register's literature_import_tol"
    );
    let cfg = config(&root(), "literature-import").expect("gate.json");
    assert_eq!(
        cfg.threshold.value,
        LITERATURE_IMPORT_TOL.number(),
        "gate.json's threshold is the register's"
    );
    assert_eq!(
        cfg.threshold.requirement, "REQ-VAL-120",
        "the threshold cites REQ-VAL-120"
    );
    check_through(2);
}

negative_control!(
    qa_m2_20_through_the_chart_after_the_rescale,
    "the energy multiplied back by M¹ in place of M² misses eq. 4 by a factor 3",
    expected = "E through the chart",
    check_through(1)
);

// ── (v) Outside region D ─────────────────────────────────────────────────────────────────────────────────────────

/// Each point is refused by the gate as outside region D.
fn check_outside(points: &[[f64; 2]]) {
    for p in points {
        let (got, _) = gate_on(None, &[*p]);
        assert!(
            got.starts_with("refused") && got.contains("outside region D"),
            "the gate on ({}, {}): {got}",
            p[0],
            p[1]
        );
    }
}

#[test]
fn qa_m2_20_outside_region_d_refused() {
    let outside = [
        [0.432697866, 0.7311230217], // N = 88 as paper I prints it
        [0.3, 0.8],                  // past the arc: 0.64 + 0.64 > 1
        [-0.01, 0.5],                // ξ < 0
        [0.2, -0.01],                // η < 0
    ];
    for p in outside {
        assert!(!in_d(p[0], p[1]), "{p:?} is outside D by eq. 8");
    }
    check_outside(&outside);
    // The edges are inside, and the gate passes with them.
    let (got, _) = gate_on(None, &[[0.0, 0.5], [0.2, 0.51f64.sqrt() * 0.999_999]]);
    assert_eq!(got, "pass", "points on region D's edges are inside it");
}

negative_control!(
    qa_m2_20_outside_region_d_refused,
    "a point inside region D is not refused",
    expected = "the gate on (0.2, 0.5)",
    check_outside(&[[0.2, 0.5]])
);

// ── Column 8: recorded, not asserted (RQ-570) ────────────────────────────────────────────────────────────────────

/// `col8_comparison.csv` has one line per row: eq. 4 (this file's) times 10⁷, column 8 as Table 1 prints it, their
/// ratio; RQ-566's examples read as the ruling gives them.
fn check_col8_record(table: &[QaRow]) {
    let text = read("fixtures/anosova/col8_comparison.csv");
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some("N,eq4_e7,col8_printed_e7,ratio_printed_over_eq4"),
        "the header"
    );
    let recorded: Vec<Vec<String>> = lines.map(csv_fields).collect();
    assert_eq!(recorded.len(), table.len(), "one comparison line per row");
    for (f, r) in recorded.iter().zip(table) {
        assert_eq!(f[0], r.n.to_string(), "the record's N");
        let e7 = eq4(r.xi(), r.eta()) * 1e7;
        assert_eq!(f[1], format!("{e7:.4}"), "N = {}: eq. 4 recorded", r.n);
        assert_eq!(f[2], r.col8, "N = {}: column 8 recorded as printed", r.n);
        if r.col8 != "—" {
            let ratio = r.col8.parse::<f64>().unwrap() / e7;
            assert_eq!(f[3], format!("{ratio:.4}"), "N = {}: the ratio", r.n);
        }
    }
    for (n, eq, printed) in [
        (1, "-2.464", "-2.254"),
        (3, "-2.589", "-2.041"),
        (10, "-3.858", "-0.887"),
    ] {
        let r = &table[n - 1];
        assert_eq!(
            format!("{:.3}", eq4(r.xi(), r.eta()) * 1e7),
            eq,
            "RQ-566's N = {n}: computed"
        );
        assert_eq!(r.col8, printed, "RQ-566's N = {n}: Table 1's column 8");
    }
}

#[test]
fn qa_m2_20_column8_is_recorded() {
    check_col8_record(&rows());
}

negative_control!(
    qa_m2_20_column8_is_recorded,
    "a record not taken from Table 1's column 8 is not the record",
    expected = "N = 2: column 8 recorded as printed",
    check_col8_record(&qa_rows(&table_with(2, "E_col8_e7", "-2.302")))
);

/// The gate on Table 1 with `n`'s `column` set to `value` passes, and its scatter records column 8 as edited.
fn check_not_judged(n: u32, column: &str, value: &str) {
    let (got, scatter) = gate_on(Some(&table_with(n, column, value)), &[]);
    assert_eq!(got, "pass", "the gate with row {n}'s {column} = {value}");
    let at = scatter
        .iter()
        .position(|l| l.contains("column 8") && l.contains("RQ-570"))
        .expect("the column 8 record");
    assert_eq!(
        scatter.len() - at - 1,
        100,
        "column 8 recorded on every row"
    );
    if column == "E_col8_e7" {
        assert!(
            scatter[at + n as usize].split(',').nth(2) == Some(value),
            "row {n}'s column 8 recorded as {value}: {}",
            scatter[at + n as usize]
        );
    }
}

#[test]
fn qa_m2_20_column8_is_not_judged() {
    check_not_judged(1, "E_col8_e7", "-9.999");
    check_not_judged(10, "E_col8_e7", "+123.456");
    check_not_judged(11, "E_col8_e7", "-2.000");
}

negative_control!(
    qa_m2_20_column8_is_not_judged,
    "the same edit in column 11, which is judged, fails the gate",
    expected = "the gate with row 1's M_col11_e4",
    check_not_judged(1, "M_col11_e4", "-9.999")
);

// ── The Burrau constants (REQ-VAL-174) ───────────────────────────────────────────────────────────────────────────

/// The Burrau rest start, imported, multiplied back by `(M, G, λ)` with the energy's mass power `mass_power`: `M`,
/// `R = 1/λ` and `E` read `12`, `2.2361` and `−12.8167`.
fn check_burrau(mass_power: i32) {
    let s = Start {
        name: "Burrau".to_owned(),
        citation: "dd_validation_orbits §2 (qa)".to_owned(),
        g: 1.0,
        m: [3.0, 4.0, 5.0],
        r: [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]],
        v: [[0.0; 2]; 3],
    };
    let i = import(&s).expect("imports");
    let x = &i.ingested.decoded;
    let m_tot: f64 = s.m.iter().sum();
    let lambda = i.ingested.encoded.canonical.lambda;
    let vc: [[f64; 2]; 3] = std::array::from_fn(|j| [x.p[j][0] / x.m[j], x.p[j][1] / x.m[j]]);
    let ec = kinetic(x.m, vc) + potential(1.0, x.m, x.r);
    let e = s.g * m_tot.powi(mass_power) * lambda * ec;
    let r_hyper = 1.0 / lambda;
    // R = √(I/M) by this file's own sum: I = Σ mᵢ‖rᵢ − R_com‖² = 60.
    let com = [0, 1].map(|c| (0..3).map(|j| s.m[j] * s.r[j][c]).sum::<f64>() / m_tot);
    let inertia: f64 = (0..3)
        .map(|j| s.m[j] * ((s.r[j][0] - com[0]).powi(2) + (s.r[j][1] - com[1]).powi(2)))
        .sum();
    assert_eq!(inertia, 60.0, "the Burrau start's I");
    assert!(
        rel(r_hyper, (inertia / m_tot).sqrt()) <= tol(),
        "R = 1/λ is √(I/M)"
    );
    assert_eq!(
        (
            format!("{m_tot}"),
            format!("{r_hyper:.4}"),
            format!("{e:.4}")
        ),
        ("12".to_owned(), "2.2361".to_owned(), "-12.8167".to_owned()),
        "the Burrau constants read back"
    );
    assert!(
        rel(e, -769.0 / 60.0) <= tol(),
        "E = −769/60 within literature_import_tol"
    );
}

#[test]
fn qa_m2_20_burrau_constants() {
    check_burrau(2);
}

negative_control!(
    qa_m2_20_burrau_constants,
    "without M² the energy does not read −12.8167",
    expected = "the Burrau constants read back",
    check_burrau(0)
);

// ── Table 1's ξ: paper I's on every row (R-592's applied text) ───────────────────────────────────────────────────

/// R-592 and dd_validation_orbits §1.6: paper II's `ξ` differs from paper I's on four rows; paper I's is the `ξ` used,
/// and paper II's is recorded in the row's note. `N`, paper I's `ξ`, paper II's.
const XI_DIFFER: [(u32, &str, &str); 4] = [
    (35, "0.480736434", "0.480236434"),
    (49, "0.057727505", "0.058727505"),
    (54, "0.007748064", "0.077480639"),
    (65, "0.306397814", "0.306897814"),
];

/// The transcriptions' one difference, `N = 17`'s `ξ`: A read `0.115219340`, B `0.115279340`, settled as B's.
const XI_17: (&str, &str) = ("0.115279340", "0.115219340");

fn check_xi(rows: &[QaRow]) {
    for &(n, i, ii) in &XI_DIFFER {
        let r = rows.iter().find(|r| r.n == n).expect("the row");
        assert_eq!(r.xi, i, "N = {n}: the ξ used is paper I's");
        assert!(
            r.note.contains(ii),
            "N = {n}: paper II's ξ {ii} is recorded in the note"
        );
    }
    for r in rows {
        if XI_DIFFER.iter().all(|d| d.0 != r.n) {
            assert!(
                !r.note.contains("paper II prints xi"),
                "N = {}: paper II's ξ differs only on N = 35, 49, 54, 65",
                r.n
            );
        }
    }
}

fn check_xi_17(rows: &[QaRow]) {
    let r17 = rows.iter().find(|r| r.n == 17).expect("row 17");
    assert_eq!(r17.xi, XI_17.0, "N = 17: ξ is settled as 0.115279340");
    assert!(
        r17.note.contains(XI_17.1),
        "N = 17: the other reading, 0.115219340, is recorded"
    );
}

#[test]
fn qa_m2_20_xi_is_paper_ones_on_every_row() {
    check_xi(&rows());
}

negative_control!(
    qa_m2_20_xi_is_paper_ones_on_every_row,
    "row 54 taking paper II's ξ is not R-592's choice",
    expected = "N = 54: the ξ used is paper I's",
    check_xi(&qa_rows(&table_with(54, "xi", "0.077480639")))
);

#[test]
fn qa_m2_20_xi_17_is_the_settled_reading() {
    check_xi_17(&rows());
}

negative_control!(
    qa_m2_20_xi_17_is_the_settled_reading,
    "row 17 taking transcription A's reading is not the settled ξ",
    expected = "N = 17: ξ is settled as 0.115279340",
    check_xi_17(&qa_rows(&table_with(17, "xi", "0.115219340")))
);
