//! The literature-IC import (inverse-encode contract Part 7; dd_encode §1 and §4, the validation import row;
//! dd_validation_orbits §1.6 and §2; R-592; REQ-VAL-019, REQ-VAL-120, REQ-VAL-174): a published start, in its paper's
//! units, enters encode through the kernel's ingestion (`kernel::encode::ingest`) and comes back, multiplied by the
//! recorded rescale, as the invariants the paper states. The `literature-import` gate ([`LiteratureImportGate`])
//! judges every start against REQ-VAL-120's tolerance and reports each one tagged with its name and citation.
//!
//! **The recorded rescale** (dd_validation_orbits §2): a start with masses `mᵢ`, positions `rᵢ`, velocities `vᵢ` and
//! constant `G` enters in `G = 1` with `Σmᵢ = 1` ([`unit_state`]): masses `mᵢ/M`, positions unchanged, momenta
//! `(mᵢ/M)·vᵢ/√(GM)`. `C` then rescales to `I = 1` by `λ`, and may mirror. Multiplying back from the canonical
//! decode `D(z)` ([`Rescale`]): `E = G·M²·λ·E_c`, `L_z = ±√G·M^{3/2}·λ^{−1/2}·L_{z,c}`, `R = 1/λ`. `L_z²·|E|/(G²·M⁵)`
//! equals `L_{z,c}²·|E_c|` with no factor.
//!
//! **Region D** ([`RegionD`], [`Row`]; dd_validation_orbits §1.6): Agekyan & Anosova (1968)'s near-rest starts, equal
//! unit masses at `(−½, 0)`, `(½, 0)` and `(ξ, η)` with eq. 3's velocities and `G = 6.67·10⁻⁸`, read from
//! `fixtures/anosova/region_d.json`; Table 1's rows from `fixtures/anosova/table1.csv`. What is asserted is §1.6's
//! list: the computed `M = η·10⁻⁴` on every row ([`angular_eq3`]), the printed `M` column on every row but the
//! fixture's exception list ([`printed_m`]), `E` by eq. 4 ([`energy_eq4`]), the exact-rest control, the invariants
//! through the chart, and a point outside region D refused ([`RegionD::start`]). Column 8 is recorded beside eq. 4
//! ([`column8`]) and never judged (RQ-570).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use kernel::decode::Ic;
use kernel::encode::ingest::{ingest, Ingested};
use kernel::generated::links::Defaults;
use ledger::constants::LITERATURE_IMPORT_TOL;

use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::roundtrip::phys;

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "literature-import";

/// A published start in its paper's units.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    /// The start's tag in the report.
    pub name: String,
    /// The paper it is from.
    pub citation: String,
    /// The gravitational constant in the paper's units.
    pub g: f64,
    /// The masses.
    pub m: [f64; 3],
    /// The positions.
    pub r: [[f64; 2]; 3],
    /// The velocities.
    pub v: [[f64; 2]; 3],
}

/// A state's energy, angular momentum and hyperradius `√(I/M)`, `I` about the centre of mass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Invariants {
    /// `E = Σ ½mᵢ‖vᵢ‖² − G·Σ_{i<j} mᵢmⱼ/rᵢⱼ`.
    pub e: f64,
    /// `L_z = Σ mᵢ(xᵢẏᵢ − yᵢẋᵢ)`.
    pub lz: f64,
    /// `R = √(I/M)`, `I = Σ mᵢ‖rᵢ − R_com‖²`.
    pub hyper: f64,
}

/// The potential `−G·Σ_{i<j} mᵢmⱼ/rᵢⱼ`.
pub fn potential(g: f64, m: [f64; 3], r: [[f64; 2]; 3]) -> f64 {
    let pair = |i: usize, j: usize| m[i] * m[j] / (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]);
    -g * (pair(0, 1) + pair(0, 2) + pair(1, 2))
}

/// The invariants of masses `m`, positions `r` and velocities `v` under the constant `g`, computed directly.
pub fn invariants(g: f64, m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> Invariants {
    let total = m[0] + m[1] + m[2];
    let com = [0, 1].map(|k| (m[0] * r[0][k] + m[1] * r[1][k] + m[2] * r[2][k]) / total);
    let (mut kinetic, mut lz, mut inertia) = (0.0, 0.0, 0.0);
    for i in 0..3 {
        kinetic += 0.5 * m[i] * (v[i][0] * v[i][0] + v[i][1] * v[i][1]);
        lz += m[i] * (r[i][0] * v[i][1] - r[i][1] * v[i][0]);
        let (dx, dy) = (r[i][0] - com[0], r[i][1] - com[1]);
        inertia += m[i] * (dx * dx + dy * dy);
    }
    Invariants {
        e: kinetic + potential(g, m, r),
        lz,
        hyper: (inertia / total).sqrt(),
    }
}

/// The invariants of a state on the canonical section, `G = 1`, momenta `pᵢ = mᵢvᵢ`.
pub fn canonical_invariants(x: &Ic<f64>) -> Invariants {
    invariants(
        1.0,
        x.m,
        x.r,
        std::array::from_fn(|i| x.p[i].map(|p| p / x.m[i])),
    )
}

/// The start in `G = 1` with `Σmᵢ = 1`, its positions unchanged (the module's note), or the reason it cannot be.
pub fn unit_state(s: &Start) -> Result<Ic<f64>, String> {
    let total = s.m[0] + s.m[1] + s.m[2];
    let finite = s.r.iter().chain(&s.v).flatten().all(|c| c.is_finite());
    if !(s.m.iter().all(|&m| m > 0.0 && m.is_finite()) && s.g > 0.0 && s.g.is_finite() && finite) {
        return Err(format!(
            "{}: the masses and G must be positive and every value finite",
            s.name
        ));
    }
    let speed = (s.g * total).sqrt();
    Ok(Ic {
        m: s.m.map(|m| m / total),
        r: s.r,
        p: std::array::from_fn(|i| s.v[i].map(|v| s.m[i] / total * v / speed)),
    })
}

/// The recorded rescale: the total mass `M`, the paper's `G`, `C`'s `λ` and its mirror flag.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rescale {
    /// `M = Σmᵢ` in the paper's units.
    pub mass: f64,
    /// The paper's `G`.
    pub g: f64,
    /// `λ = I_in^{−1/2}`, `C`'s step 0.
    pub lambda: f64,
    /// `C` mirrored the state.
    pub mirrored: bool,
}

impl Rescale {
    /// `E = G·M²·λ·E_c`.
    pub fn energy(&self, e_c: f64) -> f64 {
        self.g * self.mass * self.mass * self.lambda * e_c
    }

    /// `L_z = ±√G·M^{3/2}·λ^{−1/2}·L_{z,c}`, minus where `C` mirrored.
    pub fn lz(&self, lz_c: f64) -> f64 {
        let sign = if self.mirrored { -1.0 } else { 1.0 };
        sign * self.g.sqrt() * self.mass * self.mass.sqrt() / self.lambda.sqrt() * lz_c
    }

    /// `R = 1/λ`.
    pub fn hyper(&self) -> f64 {
        1.0 / self.lambda
    }

    /// The canonical invariants `c` multiplied back.
    pub fn back(&self, c: Invariants) -> Invariants {
        Invariants {
            e: self.energy(c.e),
            lz: self.lz(c.lz),
            hyper: self.hyper(),
        }
    }

    /// The length factor alone, `E = λ·E_c` and `L_z = λ^{−1/2}·L_{z,c}`: the rescale without the mass and `G`
    /// factors, which the gate's negative input uses and must fail.
    pub fn length_only(&self) -> Self {
        Self {
            mass: 1.0,
            g: 1.0,
            ..*self
        }
    }
}

/// A start imported: the ingestion at f64 under the defaults links, the rescale, the canonical decode's invariants and
/// those multiplied back.
#[derive(Clone, Copy, Debug)]
pub struct Imported {
    /// `z`, `C(x)`, `D(z)`, the residual and `κ(z)`.
    pub ingested: Ingested<f64>,
    /// The recorded rescale.
    pub rescale: Rescale,
    /// The invariants of `D(z)`, `G = 1`.
    pub canonical: Invariants,
    /// [`Self::canonical`] multiplied back.
    pub back: Invariants,
}

impl Imported {
    /// `L_z²·|E|` on the canonical section, which equals the paper's `L_z²·|E|/(G²·M⁵)`.
    pub fn scale_free(&self) -> f64 {
        self.canonical.lz * self.canonical.lz * self.canonical.e.abs()
    }
}

/// Imports `s`: its unit state, ingested under the defaults links with Part 4's norm, and multiplied back.
pub fn import(s: &Start) -> Result<Imported, String> {
    let ic = unit_state(s)?;
    let ingested = ingest::<Defaults, f64, _>(&ic, phys);
    let rescale = Rescale {
        mass: s.m[0] + s.m[1] + s.m[2],
        g: s.g,
        lambda: ingested.encoded.canonical.lambda,
        mirrored: ingested.encoded.report.mirrored,
    };
    let canonical = canonical_invariants(&ingested.decoded);
    Ok(Imported {
        ingested,
        rescale,
        canonical,
        back: rescale.back(canonical),
    })
}

/// `|got − want|/|want|`, or `|got|` where `want` is 0.
pub fn relative(got: f64, want: f64) -> f64 {
    if want == 0.0 {
        got.abs()
    } else {
        ((got - want) / want).abs()
    }
}

// ── Region D ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// A row the printed `M` column does not match, as the fixture lists it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exception {
    /// The row.
    pub n: u32,
    /// Column 11 as printed.
    pub printed: String,
    /// `η·10⁻⁴` in column 11's units, rounded to its printed precision.
    pub computed: String,
}

/// Region D's definition, `fixtures/anosova/region_d.json` (dd_validation_orbits §1.6).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionD {
    /// The paper.
    pub citation: String,
    /// `G` in the paper's units.
    pub g: f64,
    /// The equal masses.
    pub m: [f64; 3],
    /// `A₁` and `A₂`.
    pub a: [[f64; 2]; 2],
    /// Eq. 3's velocities.
    pub v: [[f64; 2]; 3],
    /// `K₀`, the initial kinetic energy eq. 4 adds.
    pub k0: f64,
    /// The rows whose printed `M` does not match, each with its printed and computed values.
    pub m_exceptions: Vec<Exception>,
}

/// Whether `(ξ, η)` is in region D: `ξ ≥ 0`, `η ≥ 0` and `(ξ + ½)² + η² ≤ 1` (eq. 8).
pub fn in_region_d(xi: f64, eta: f64) -> bool {
    xi >= 0.0 && eta >= 0.0 && (xi + 0.5) * (xi + 0.5) + eta * eta <= 1.0
}

impl RegionD {
    /// Parses `text`.
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("region_d.json: {e}"))
    }

    /// The positions with `A₃ = (ξ, η)`.
    pub fn positions(&self, xi: f64, eta: f64) -> [[f64; 2]; 3] {
        [self.a[0], self.a[1], [xi, eta]]
    }

    /// The start at `(ξ, η)` with eq. 3's velocities, or a refusal where `(ξ, η)` is outside region D.
    pub fn start(&self, name: &str, xi: f64, eta: f64) -> Result<Start, String> {
        if !in_region_d(xi, eta) {
            return Err(format!(
                "{name}: (ξ, η) = ({xi}, {eta}) is outside region D (eq. 8)"
            ));
        }
        Ok(Start {
            name: name.to_owned(),
            citation: self.citation.clone(),
            g: self.g,
            m: self.m,
            r: self.positions(xi, eta),
            v: self.v,
        })
    }
}

/// Eq. 4: `E = −G(1 + 1/√((ξ + ½)² + η²) + 1/√((ξ − ½)² + η²)) + K₀`.
pub fn energy_eq4(d: &RegionD, xi: f64, eta: f64) -> f64 {
    -d.g * (1.0 + 1.0 / (xi + 0.5).hypot(eta) + 1.0 / (xi - 0.5).hypot(eta)) + d.k0
}

/// `M = Σ m(xẏ − yẋ)` from eq. 3's velocities at `(ξ, η)`.
pub fn angular_eq3(d: &RegionD, xi: f64, eta: f64) -> f64 {
    let r = d.positions(xi, eta);
    (0..3)
        .map(|i| d.m[i] * (r[i][0] * d.v[i][1] - r[i][1] * d.v[i][0]))
        .sum()
}

/// Which paper's `η` a row uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Paper I's, as printed.
    I,
    /// Paper II's, where paper I is the outlier.
    II,
}

/// One row of Table 1, `fixtures/anosova/table1.csv`.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The page of paper I.
    pub page: u32,
    /// `N`.
    pub n: u32,
    /// `ξ`, as printed.
    pub xi: String,
    /// Paper I's `η`, as printed.
    pub eta_i: String,
    /// Paper II's `η`, where it differs.
    pub eta_ii: Option<String>,
    /// The `η` used.
    pub eta: String,
    /// Whose it is.
    pub source: Source,
    /// Column 8, `E·10⁻⁷`, as printed; `None` for "—".
    pub e_col8: Option<String>,
    /// Column 11, `M·10⁻⁴`, as printed; `None` for "—".
    pub m_col11: Option<String>,
    /// Column 4, as text.
    pub outcome: String,
    /// Why the `η` used is used, and any reading note.
    pub note: String,
}

impl Row {
    /// `ξ` as a number.
    pub fn xi(&self) -> f64 {
        self.xi.parse().unwrap_or(f64::NAN)
    }

    /// The `η` used, as a number.
    pub fn eta(&self) -> f64 {
        self.eta.parse().unwrap_or(f64::NAN)
    }
}

/// The fields of one CSV line: separated by commas, a field in double quotes may hold commas and `""`.
pub fn fields(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// The header `table1.csv` carries.
pub const TABLE1_HEADER: &str =
    "page,N,xi,eta_I,eta_II,eta_used,eta_source,E_col8_e7,M_col11_e4,outcome,note";

/// Parses `table1.csv`'s text: every row, in order.
pub fn table1(text: &str) -> Result<Vec<Row>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(TABLE1_HEADER) {
        return Err(format!("table1.csv: the header is not `{TABLE1_HEADER}`"));
    }
    let printed = |s: &str| (s != "—").then(|| s.to_owned());
    lines
        .enumerate()
        .map(|(i, line)| {
            let f = fields(line);
            let bad = |what: &str| format!("table1.csv line {}: {what}", i + 2);
            if f.len() != 11 {
                return Err(bad("not 11 fields"));
            }
            let source = match f[6].as_str() {
                "I" => Source::I,
                "II" => Source::II,
                _ => return Err(bad("eta_source is neither I nor II")),
            };
            Ok(Row {
                page: f[0].parse().map_err(|_| bad("page"))?,
                n: f[1].parse().map_err(|_| bad("N"))?,
                xi: f[2].clone(),
                eta_i: f[3].clone(),
                eta_ii: (!f[4].is_empty()).then(|| f[4].clone()),
                eta: f[5].clone(),
                source,
                e_col8: printed(&f[7]),
                m_col11: printed(&f[8]),
                outcome: f[9].clone(),
                note: f[10].clone(),
            })
        })
        .collect()
}

/// The decimal string `s` (`0.` and digits) rounded half up to `places` decimals, or `None` if it is not one.
pub fn round_decimal(s: &str, places: usize) -> Option<String> {
    let digits = s.strip_prefix("0.")?;
    if digits.len() <= places || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let kept: u64 = digits[..places].parse().ok()?;
    let up = u64::from(digits.as_bytes()[places] >= b'5');
    let v = kept + up;
    let scale = 10u64.pow(u32::try_from(places).ok()?);
    Some(format!("{}.{:0places$}", v / scale, v % scale))
}

/// The rows whose printed column 11 differs from `η·10⁻⁴` rounded to its printed four decimals, each as an
/// [`Exception`] with its printed and computed values; rows printing "—" are not compared.
pub fn printed_m(rows: &[Row]) -> Vec<Exception> {
    rows.iter()
        .filter_map(|r| {
            let printed = r.m_col11.as_ref()?;
            let computed = round_decimal(&r.eta, 4).unwrap_or_default();
            (&computed != printed).then(|| Exception {
                n: r.n,
                printed: printed.clone(),
                computed,
            })
        })
        .collect()
}

/// Checks the mismatches against the fixture's exception list: a mismatch not listed fails, and so does a listed row
/// that matches, or one listed with other values.
pub fn check_exceptions(got: &[Exception], listed: &[Exception]) -> Result<(), String> {
    let mut wrong = Vec::new();
    for g in got {
        match listed.iter().find(|l| l.n == g.n) {
            None => wrong.push(format!("N = {} mismatches and is not listed: {g:?}", g.n)),
            Some(l) if l != g => wrong.push(format!("N = {} is listed as {l:?} but is {g:?}", g.n)),
            Some(_) => {}
        }
    }
    for l in listed {
        if !got.iter().any(|g| g.n == l.n) {
            wrong.push(format!("N = {} is listed but its printed M matches", l.n));
        }
    }
    if wrong.is_empty() {
        Ok(())
    } else {
        Err(wrong.join("; "))
    }
}

/// Column 8 beside eq. 4, one row: recorded, never judged (RQ-570).
#[derive(Clone, Debug, PartialEq)]
pub struct Column8 {
    /// `N`.
    pub n: u32,
    /// Eq. 4 on the row's `ξ` and `η`, times 10⁷.
    pub eq4_e7: f64,
    /// Column 8 as printed.
    pub printed: Option<String>,
}

impl Column8 {
    /// The CSV line: `N`, eq. 4 to four decimals, the printed value or "—", and their ratio to four decimals or "—".
    pub fn line(&self) -> String {
        let ratio = self
            .printed
            .as_ref()
            .and_then(|p| p.parse::<f64>().ok())
            .map_or("—".to_owned(), |p| format!("{:.4}", p / self.eq4_e7));
        format!(
            "{},{:.4},{},{ratio}",
            self.n,
            self.eq4_e7,
            self.printed.as_deref().unwrap_or("—")
        )
    }
}

/// The header of `fixtures/anosova/col8_comparison.csv`.
pub const COLUMN8_HEADER: &str = "N,eq4_e7,col8_printed_e7,ratio_printed_over_eq4";

/// Column 8 beside eq. 4 on every row.
pub fn column8(d: &RegionD, rows: &[Row]) -> Vec<Column8> {
    rows.iter()
        .map(|r| Column8 {
            n: r.n,
            eq4_e7: energy_eq4(d, r.xi(), r.eta()) * 1e7,
            printed: r.e_col8.clone(),
        })
        .collect()
}

/// The comparison's CSV text, header first, one line per row.
pub fn column8_csv(d: &RegionD, rows: &[Row]) -> String {
    let mut out = String::from(COLUMN8_HEADER);
    out.push('\n');
    for c in column8(d, rows) {
        out.push_str(&c.line());
        out.push('\n');
    }
    out
}

// ── The gate ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// A start with the values its paper prints, as text to its printed precision.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Printed {
    /// The start.
    pub start: Start,
    /// The total mass, as printed.
    pub mass: String,
    /// The hyperradius `√(I/M)`, as printed.
    pub hyper: String,
    /// The energy, as printed.
    pub energy: String,
}

/// How the gate multiplies back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Back {
    /// The recorded rescale, `(M, G, λ)` and the mirror.
    Recorded,
    /// The length factor alone, which must fail (PIT-3).
    LengthOnly,
}

/// A `literature-import` input, `fixtures/gates/literature-import/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The starts whose paper prints its values (the Burrau rest start).
    pub printed: Vec<Printed>,
    /// Region D's definition, relative to the workspace root.
    pub region_d: String,
    /// Table 1, relative to the workspace root.
    pub table1: String,
    /// Further region-D points `(ξ, η)`, each imported as the table's rows are.
    #[serde(default)]
    pub points: Vec<[f64; 2]>,
    /// How the gate multiplies back.
    pub back: Back,
}

/// One quantity compared: the paper's value, the import's, and their relative difference in units of f64's epsilon.
#[derive(Clone, Debug, PartialEq)]
pub struct Compared {
    /// The quantity.
    pub what: &'static str,
    /// The paper's value.
    pub paper: f64,
    /// The import's.
    pub imported: f64,
    /// `|imported − paper|/|paper|` in units of `2⁻⁵²`.
    pub eps: f64,
}

/// One start's tagged report: its name, citation, `z`, Part 7's diagnostics, the rescale and what was compared.
#[derive(Clone, Debug)]
pub struct Tagged {
    /// The start's tag.
    pub name: String,
    /// Its paper.
    pub citation: String,
    /// The import.
    pub imported: Imported,
    /// The quantities compared against the tolerance.
    pub compared: Vec<Compared>,
    /// Printed values that do not read back to their printed precision.
    pub misprinted: Vec<String>,
}

impl Tagged {
    /// The largest relative difference, in units of f64's epsilon; NaN where one is NaN.
    pub fn worst(&self) -> f64 {
        self.compared.iter().map(|c| c.eps).fold(0.0, |a, e| {
            if e.is_nan() || a.is_nan() {
                f64::NAN
            } else {
                a.max(e)
            }
        })
    }

    /// One report line.
    pub fn line(&self) -> String {
        let i = &self.imported;
        let parts: Vec<String> = self
            .compared
            .iter()
            .map(|c| {
                format!(
                    "{} {:.6e} vs {:.6e} ({:.2} ε)",
                    c.what, c.imported, c.paper, c.eps
                )
            })
            .collect();
        format!(
            "{} [{}]: z = {:?}; ‖D(z) − C(x)‖_phys = {:.3e}; κ = {:.3e}; rescale M = {}, G = {}, λ = {:.12e}, mirrored = {}; \
             {}{}",
            self.name,
            self.citation,
            i.ingested.encoded.z.z,
            i.ingested.residual,
            i.ingested.kappa,
            i.rescale.mass,
            i.rescale.g,
            i.rescale.lambda,
            i.rescale.mirrored,
            parts.join("; "),
            if self.misprinted.is_empty() {
                String::new()
            } else {
                format!("; printed values not read back: {}", self.misprinted.join(", "))
            }
        )
    }
}

fn compared(what: &'static str, paper: f64, imported: f64) -> Compared {
    Compared {
        what,
        paper,
        imported,
        eps: relative(imported, paper) / f64::EPSILON,
    }
}

/// Imports `s` and multiplies back as `back` says.
fn import_as(s: &Start, back: Back) -> Result<Imported, String> {
    let mut i = import(s)?;
    if back == Back::LengthOnly {
        i.rescale = i.rescale.length_only();
        i.back = i.rescale.back(i.canonical);
    }
    Ok(i)
}

/// The tagged report of a start whose paper prints `M`, `R` and `E`: each compared with the start's own invariants,
/// and each printed value read back to its printed precision.
pub fn printed_report(p: &Printed, back: Back) -> Result<Tagged, String> {
    let s = &p.start;
    let imported = import_as(s, back)?;
    let own = invariants(s.g, s.m, s.r, s.v);
    let b = imported.back;
    let mass = imported.rescale.mass;
    let mut misprinted = Vec::new();
    for (what, value, text) in [
        ("M", mass, &p.mass),
        ("R", b.hyper, &p.hyper),
        ("E", b.e, &p.energy),
    ] {
        let places = text.split('.').nth(1).map_or(0, str::len);
        if format!("{value:.places$}") != *text {
            misprinted.push(format!("{what} reads {value:.places$}, printed {text}"));
        }
    }
    Ok(Tagged {
        name: s.name.clone(),
        citation: s.citation.clone(),
        compared: vec![
            compared("E", own.e, b.e),
            compared("L_z", own.lz, b.lz),
            compared("R", own.hyper, b.hyper),
        ],
        misprinted,
        imported,
    })
}

/// The tagged report of a region-D start at `(ξ, η)`: `E` against eq. 4, `L_z` against `η·10⁻⁴`, both multiplied back,
/// and `L_z²·|E|/(G²·M⁵)` with no factor.
pub fn region_d_report(
    d: &RegionD,
    name: &str,
    xi: f64,
    eta: f64,
    back: Back,
) -> Result<Tagged, String> {
    let s = d.start(name, xi, eta)?;
    let imported = import_as(&s, back)?;
    let (e, lz) = (energy_eq4(d, xi, eta), angular_eq3(d, xi, eta));
    let mass = imported.rescale.mass;
    let scale_free = lz * lz * e.abs() / (d.g * d.g * mass.powi(5));
    Ok(Tagged {
        name: s.name,
        citation: s.citation,
        compared: vec![
            compared("E", e, imported.back.e),
            compared("L_z", lz, imported.back.lz),
            compared("L_z²|E|/(G²M⁵)", scale_free, imported.scale_free()),
        ],
        misprinted: Vec::new(),
        imported,
    })
}

/// Everything the gate measured on one input.
#[derive(Clone, Debug)]
pub struct Measured {
    /// Each start's report, the printed starts first, then Table 1's rows in order, then the extra points.
    pub tagged: Vec<Tagged>,
    /// The printed `M` column's mismatches and their check against the exception list.
    pub exceptions: (Vec<Exception>, Result<(), String>),
    /// Column 8 beside eq. 4, recorded.
    pub column8: Vec<Column8>,
}

/// The workspace root: the directory above `crates/validation`.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> Result<String, String> {
    let p = root().join(path);
    std::fs::read_to_string(&p).map_err(|e| format!("refused: cannot read {}: {e}", p.display()))
}

/// Measures `input`, or refuses it: a start that cannot be imported, a region-D point outside region D included.
pub fn measure(input: &Input) -> Result<Measured, String> {
    let d = RegionD::parse(&read(&input.region_d)?)?;
    let rows = table1(&read(&input.table1)?)?;
    let mut tagged = Vec::new();
    for p in &input.printed {
        tagged.push(printed_report(p, input.back).map_err(|e| format!("refused: {e}"))?);
    }
    for r in &rows {
        let name = format!("Anosova region D, Table 1 N = {}", r.n);
        tagged.push(
            region_d_report(&d, &name, r.xi(), r.eta(), input.back)
                .map_err(|e| format!("refused: {e}"))?,
        );
    }
    for (k, [xi, eta]) in input.points.iter().enumerate() {
        let name = format!("Anosova region D, point {}", k + 1);
        tagged.push(
            region_d_report(&d, &name, *xi, *eta, input.back)
                .map_err(|e| format!("refused: {e}"))?,
        );
    }
    let mismatches = printed_m(&rows);
    let check = check_exceptions(&mismatches, &d.m_exceptions);
    Ok(Measured {
        tagged,
        exceptions: (mismatches, check),
        column8: column8(&d, &rows),
    })
}

/// Whether `t` fails against `k`: a comparison past `k` ε or NaN, or a printed value not read back.
pub fn fails(t: &Tagged, k: f64) -> bool {
    let w = t.worst();
    w.is_nan() || w > k || !t.misprinted.is_empty()
}

/// The `literature-import` gate.
pub struct LiteratureImportGate;

impl LiteratureImportGate {
    /// Measures `input` and judges it against `config`'s threshold, which must be the register's
    /// `literature_import_tol`, or refuses it.
    pub fn judge(config: &GateConfig, name: &str, input: &Input) -> Result<GateReport, String> {
        let k = config.threshold.value;
        let register = LITERATURE_IMPORT_TOL.number();
        if k != register {
            return Err(format!(
                "refused: gate.json's threshold {k} is not the register's literature_import_tol {register} (R-427)"
            ));
        }
        let m = measure(input)?;
        let failing: Vec<&Tagged> = m.tagged.iter().filter(|t| fails(t, k)).collect();
        let verdict = if failing.is_empty() && m.exceptions.1.is_ok() {
            Verdict::Pass
        } else {
            Verdict::Fail
        };
        let mut scatter = vec![format!(
            "each start's E, L_z (and R, or L_z²|E|/(G²M⁵)) after multiplying back, against its paper's, relative, in \
             f64's ε, against {k} ε; multiplied back by {:?}",
            input.back
        )];
        scatter.extend(m.tagged.iter().map(Tagged::line));
        scatter.push(format!(
            "Table 1's printed M against η·10⁻⁴: mismatches {:?}; {}",
            m.exceptions.0.iter().map(|e| e.n).collect::<Vec<_>>(),
            match &m.exceptions.1 {
                Ok(()) => "exactly the exception list".to_owned(),
                Err(e) => e.clone(),
            }
        ));
        scatter.push("column 8 (E·10⁻⁷) beside eq. 4, recorded, not judged (RQ-570):".to_owned());
        scatter.extend(m.column8.iter().map(Column8::line));
        let worst = m.tagged.iter().map(Tagged::worst).fold(0.0, f64::max);
        let mut negative = vec![if failing.is_empty() {
            format!(
                "0 of {} starts beyond {k} ε (worst {worst:.2} ε)",
                m.tagged.len()
            )
        } else {
            format!(
                "failing starts: {}",
                failing
                    .iter()
                    .map(|t| format!("{} ({:.2} ε)", t.name, t.worst()))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        }];
        if input.back == Back::LengthOnly {
            negative.push(
                "multiplied back by the length factor alone, without M and G (PIT-3: it must fail)"
                    .to_owned(),
            );
        }
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict,
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(
                    u32::try_from(m.tagged.len()).map_err(|e| e.to_string())?,
                ),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}

impl Gate for LiteratureImportGate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = Input::deserialize(fixture)
            .map_err(|e| format!("refused: {name} is not a literature-import input: {e}"))?;
        Self::judge(config, name, &input)
    }
}
