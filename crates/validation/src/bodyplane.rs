//! The body plane against the Python reference (TASK-M3-36; chart_reference §5.1; REQ-VAL-028, REQ-VAL-119,
//! REQ-VAL-146): `cargo xtask golden bodyplane` and `tests/bodyplane.rs`.
//!
//! **The fixture** (`fixtures/golden/bodyplane/reference.tsv`) is the Python reference's output, recorded by
//! `crates/validation/xcheck/bodyplane_ref.py` from `docs/reference/prin-rs/reference/` unmodified: the moved body's
//! position at each node of `tb.burrau_grid` (numpy.linspace, nominal copies), and the reference's own algebra on that
//! state (`tb.energy`, `tb.pair_dists`, `tb_ftle.inertia`, `refine_test.shape_vec`). CI has no numpy, so the TSV is
//! committed and read here; `--record` re-runs the script.
//!
//! **What is compared, and how** (each criterion fixed before the first run):
//! - **placement, bitwise** — the one operation whose order is identical on both sides. [`BodyPlane::system_at`] at the
//!   reference's own position gives the reference's state: the moved body there, the held bodies at Burrau's, at rest.
//!   Every component `==`.
//! - **`Φ`'s pixel position, within [`POSITION_ULPS`]** — the order differs. `Φ(i/(n − 1))` forms
//!   `(c − h) + (2h)·u` with `u = fl(i/(n − 1))`; numpy forms `i·fl(fl(fl(c + h) − fl(c − h))/(n − 1)) + fl(c − h)`,
//!   its last node `fl(c + h)` exactly. Bounding each rounding by half an ulp: numpy's endpoint and width cost at most
//!   `ulp(b) + ulp(w)`, its step and product `ulp(w)`, its sum `½ulp(x)`; `Φ`'s `u` and product `ulp(w)`, its sum
//!   `½ulp(x)`. With `w ≤ 2·max(|a|, |b|)` and `|x| ≤ max(|a|, |b|)`, the difference is at most `6·ulp(max(|a|, |b|))`,
//!   `a = c − h`, `b = c + h`: the tolerance, derived, not fitted.
//! - **the algebra, within [`ALGEBRA_TOL`]** — the order differs. At the reference's position (so the placement's
//!   rounding does not enter): the shape vector by `kernel::shape::shape` on the raw state, and by the same map on the
//!   kernel's decode, against `refine_test.shape_vec`; the decode's dimensionless energy `V₀` (`Σm = 1`, `I = 1`)
//!   against `E·√I/M^{5/2}`; the decode's least pair distance `r_min_pair_0` against `min(dᵢⱼ)/R`, `R = √(I/M)` the
//!   reference's hyperradius. These are gauge-free: the decode puts every pixel at `Σm = 1` and `I = 1`, so its length
//!   and time units differ from the reference's per pixel, and only quantities free of those units can agree. The
//!   bound is `1e-13`, absolute on `n`'s components and relative on the energy and the distance: R-257's Python
//!   cross-check asks `~1e-10` of a march; a state's algebra has no march to amplify its `~10` roundings, each `2⁻⁵³`,
//!   times a conditioning of at most the window's largest coordinate over its smallest side (`13/2`).
//!
//! **The shape vector's sign convention.** `refine_test.shape_vec` takes `q = ρ̃_y λ̃_x − ρ̃_x λ̃_y`; the kernel takes the
//! standard cross, `ρ̃_x λ̃_y − ρ̃_y λ̃_x` (R-14, `kernel::shape`). So the reference's `n₂` is the kernel's `−n₂`: a
//! convention, applied as a sign before the comparison, never absorbed in the tolerance (the control without it fails).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use kernel::chart::BodyPlane;
use kernel::shape::shape;

use crate::gate::reregistration::{BURRAU_M, BURRAU_R};

/// The fixture, relative to the workspace root.
pub const FIXTURE: &str = "fixtures/golden/bodyplane/reference.tsv";

/// The recording script, relative to the workspace root.
pub const SCRIPT: &str = "crates/validation/xcheck/bodyplane_ref.py";

/// `Φ`'s pixel position against numpy's linspace node, in units in the last place of `max(|c − h|, |c + h|)` (the
/// module's note).
pub const POSITION_ULPS: f64 = 6.0;

/// The algebra's bound (the module's note).
pub const ALGEBRA_TOL: f64 = 1e-13;

/// The Burrau total mass, `M = 12`.
pub const BURRAU_TOTAL: f64 = 12.0;

/// One window of the fixture.
#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    pub name: String,
    pub body: u32,
    pub centre: [f64; 2],
    pub half: f64,
    pub n: u32,
}

/// One node: the reference's position and algebra there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub case: usize,
    pub i: u32,
    pub j: u32,
    pub x: f64,
    pub y: f64,
    pub energy: f64,
    pub d: [f64; 3],
    pub inertia: f64,
    pub hyperradius: f64,
    pub n: [f64; 3],
}

/// The fixture: its header lines, its windows and its rows.
#[derive(Clone, Debug, PartialEq)]
pub struct Fixture {
    pub header: Vec<String>,
    pub cases: Vec<Case>,
    pub rows: Vec<Row>,
}

/// The columns the fixture's `# columns=` line must name, in order.
pub const COLUMNS: &str = "case,i,j,x,y,energy,d01,d02,d12,inertia,hyperradius,n0,n1,n2";

fn field<'a>(line: &'a str, key: &str) -> Result<&'a str, String> {
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(key).and_then(|v| v.strip_prefix('=')))
        .ok_or_else(|| format!("fixture header `{line}` has no `{key}=`"))
}

fn num(s: &str) -> Result<f64, String> {
    s.parse::<f64>().map_err(|e| format!("`{s}`: {e}"))
}

impl Fixture {
    /// Parses the fixture's text.
    pub fn parse(text: &str) -> Result<Fixture, String> {
        let mut header = Vec::new();
        let mut cases = Vec::new();
        let mut rows = Vec::new();
        let mut columns = None;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            if let Some(rest) = line.strip_prefix("# ") {
                header.push(line.to_owned());
                if let Some(c) = rest.strip_prefix("columns=") {
                    columns = Some(c.to_owned());
                } else if rest.starts_with("case ") {
                    cases.push(Case {
                        name: field(rest, "name")?.to_owned(),
                        body: field(rest, "body")?.parse().map_err(|e| format!("{e}"))?,
                        centre: [num(field(rest, "cx")?)?, num(field(rest, "cy")?)?],
                        half: num(field(rest, "half")?)?,
                        n: field(rest, "n")?.parse().map_err(|e| format!("{e}"))?,
                    });
                }
                continue;
            }
            let cells: Vec<&str> = line.split('\t').collect();
            if cells.len() != 14 {
                return Err(format!(
                    "fixture row has {} cells, not 14: `{line}`",
                    cells.len()
                ));
            }
            let v: Vec<f64> = cells[3..]
                .iter()
                .map(|c| num(c))
                .collect::<Result<_, _>>()?;
            rows.push(Row {
                case: cells[0].parse().map_err(|e| format!("{e}"))?,
                i: cells[1].parse().map_err(|e| format!("{e}"))?,
                j: cells[2].parse().map_err(|e| format!("{e}"))?,
                x: v[0],
                y: v[1],
                energy: v[2],
                d: [v[3], v[4], v[5]],
                inertia: v[6],
                hyperradius: v[7],
                n: [v[8], v[9], v[10]],
            });
        }
        if columns.as_deref() != Some(COLUMNS) {
            return Err(format!("fixture columns {columns:?}, not `{COLUMNS}`"));
        }
        if let Some(r) = rows.iter().find(|r| r.case >= cases.len()) {
            return Err(format!(
                "fixture row names case {}, of {}",
                r.case,
                cases.len()
            ));
        }
        Ok(Fixture {
            header,
            cases,
            rows,
        })
    }

    /// Reads the fixture under the workspace `root`.
    pub fn load(root: &Path) -> Result<Fixture, String> {
        let path = root.join(FIXTURE);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Fixture::parse(&text)
    }
}

/// The Burrau masses at `Σm = 1`, `(3, 4, 5)/12`.
pub fn unit_masses() -> [f64; 3] {
    BURRAU_M.map(|m| m / BURRAU_TOTAL)
}

/// The window `case` as the kernel's chart, at f64.
pub fn chart(case: &Case) -> BodyPlane<f64> {
    BodyPlane {
        body: case.body,
        centre: case.centre,
        half: case.half,
        m: unit_masses(),
        r: BURRAU_R,
    }
}

/// One unit in the last place of `x` (its spacing to the next float away from zero).
pub fn ulp(x: f64) -> f64 {
    let a = x.abs();
    f64::from_bits(a.to_bits() + 1) - a
}

/// How the shape vectors are compared: the reference's `n₂` negated (the module's note), or, for the control, not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convention {
    /// The reference's `n₂` is the kernel's `−n₂`.
    Negated,
    /// No sign applied: the control.
    AsIs,
}

/// `Φ`'s pixel position, computed how: the chart's, or, for the control, another affine form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// [`BodyPlane::position`].
    Chart,
    /// The control: the window shifted by a hundredth of a cell, `Φ(u + 0.01/(n − 1))`.
    Shifted,
}

/// A column's worst agreement.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column {
    pub name: &'static str,
    /// The largest difference, in the column's measure.
    pub worst: f64,
    /// The row where it lies.
    pub at: usize,
    /// The bound.
    pub bound: f64,
    /// Rows past the bound.
    pub failures: usize,
}

impl Column {
    fn new(name: &'static str, bound: f64) -> Column {
        Column {
            name,
            bound,
            ..Column::default()
        }
    }

    fn see(&mut self, row: usize, d: f64) {
        if d.is_nan() || d > self.bound {
            self.failures += 1;
        }
        if d > self.worst || d.is_nan() {
            self.worst = d;
            self.at = row;
        }
    }
}

/// The comparison's result.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// Rows whose placement at the reference's position is not bitwise the reference's state.
    pub placement_mismatches: usize,
    pub columns: Vec<Column>,
    pub rows: usize,
}

impl Report {
    /// Whether every criterion holds.
    pub fn pass(&self) -> bool {
        self.placement_mismatches == 0 && self.columns.iter().all(|c| c.failures == 0)
    }

    /// The report, one line a column.
    pub fn text(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(
            s,
            "placement at the reference's position (bitwise): {} of {} rows differ",
            self.placement_mismatches, self.rows
        );
        let _ = writeln!(
            s,
            "{:<34}{:>14}{:>10}{:>14}{:>10}",
            "column", "worst", "row", "bound", "past"
        );
        for c in &self.columns {
            let _ = writeln!(
                s,
                "{:<34}{:>14.3e}{:>10}{:>14.3e}{:>10}",
                c.name, c.worst, c.at, c.bound, c.failures
            );
        }
        let _ = writeln!(s, "{}", if self.pass() { "PASS" } else { "FAIL" });
        s
    }
}

/// Compares the kernel's body plane with the fixture, under `convention` and `placement`.
pub fn compare(fx: &Fixture, convention: Convention, placement: Placement) -> Report {
    let mut pos = Column::new("Φ position, ulps of max(|c±h|)", POSITION_ULPS);
    let mut raw = [
        Column::new("n0, raw state", ALGEBRA_TOL),
        Column::new("n1, raw state", ALGEBRA_TOL),
        Column::new("n2, raw state", ALGEBRA_TOL),
    ];
    let mut dec = [
        Column::new("n0, decoded", ALGEBRA_TOL),
        Column::new("n1, decoded", ALGEBRA_TOL),
        Column::new("n2, decoded", ALGEBRA_TOL),
    ];
    let mut energy = Column::new("V₀ vs E·√I/M^(5/2), relative", ALGEBRA_TOL);
    let mut dmin = Column::new("r_min_pair_0 vs min(d)/R, relative", ALGEBRA_TOL);
    let mut kinetic = Column::new("K₀ (at rest), absolute", 0.0);
    let mut mismatches = 0;
    let sign = match convention {
        Convention::Negated => -1.0,
        Convention::AsIs => 1.0,
    };
    for (k, row) in fx.rows.iter().enumerate() {
        let case = &fx.cases[row.case];
        let c = chart(case);
        // Placement at the reference's own position: bitwise.
        let (m, r) = c.system_at([row.x, row.y]);
        let mut want = BURRAU_R;
        want[case.body as usize] = [row.x, row.y];
        let same = r
            .iter()
            .flatten()
            .zip(want.iter().flatten())
            .all(|(a, b)| a.to_bits() == b.to_bits());
        if !same || m != unit_masses() {
            mismatches += 1;
        }
        // Φ at the node.
        let last = f64::from(case.n - 1);
        let shift = match placement {
            Placement::Chart => 0.0,
            Placement::Shifted => 0.01 / last,
        };
        let u = f64::from(row.i) / last + shift;
        let v = f64::from(row.j) / last;
        let p = c.position(u, v);
        for (axis, (got, want)) in p.iter().zip([row.x, row.y]).enumerate() {
            let scale = (case.centre[axis] - case.half)
                .abs()
                .max((case.centre[axis] + case.half).abs());
            pos.see(k, (got - want).abs() / ulp(scale));
        }
        // The algebra at the reference's position.
        let n_raw = shape(r, m);
        let d = BodyPlane::<f64>::decode_system(m, r);
        let n_dec = shape(d.ic.r, d.ic.m);
        let n_ref = [row.n[0], row.n[1], sign * row.n[2]];
        for q in 0..3 {
            raw[q].see(k, (n_raw[q] - n_ref[q]).abs());
            dec[q].see(k, (n_dec[q] - n_ref[q]).abs());
        }
        let e_ref = row.energy * row.inertia.sqrt() / BURRAU_TOTAL.powf(2.5);
        energy.see(k, ((d.descriptor.V_0 - e_ref) / e_ref).abs());
        let dmin_ref = row.d.iter().copied().fold(f64::INFINITY, f64::min) / row.hyperradius;
        dmin.see(k, ((d.descriptor.r_min_pair_0 - dmin_ref) / dmin_ref).abs());
        kinetic.see(k, d.descriptor.K_0.abs());
    }
    let mut columns = vec![pos];
    columns.extend(raw);
    columns.extend(dec);
    columns.extend([energy, dmin, kinetic]);
    Report {
        placement_mismatches: mismatches,
        columns,
        rows: fx.rows.len(),
    }
}

/// The calibration proposal (REQ-VAL-146; `--propose`): the operations whose order differs from the reference, each
/// difference measured, and the tolerance stated.
pub fn proposal(fx: &Fixture) -> String {
    let r = compare(fx, Convention::Negated, Placement::Chart);
    let col = |name: &str| r.columns.iter().find(|c| c.name.starts_with(name)).cloned();
    let mut s = String::new();
    let _ = writeln!(
        s,
        "BodyPlane against the Python reference: the calibration proposal (REQ-VAL-146)\n"
    );
    let _ = writeln!(s, "Identical operation order, compared bitwise:");
    let _ = writeln!(
        s,
        "- placement: the moved body at the reference's position, the held bodies at Burrau's, at rest: {} of {} rows differ",
        r.placement_mismatches, r.rows
    );
    let _ = writeln!(s, "\nOperation order that differs, each measured:");
    if let Some(c) = col("Φ position") {
        let _ = writeln!(
            s,
            "- Φ's pixel position: (c − h) + (2h)·fl(i/(n − 1)) against numpy.linspace's i·fl((fl(c + h) − fl(c − h))/(n − 1)) + fl(c − h), last node fl(c + h): worst {:.3} ulp of max(|c ± h|) (row {}); derived bound {} ulp",
            c.worst, c.at, POSITION_ULPS
        );
    }
    let worst = |names: &[&str]| {
        names
            .iter()
            .filter_map(|n| col(n))
            .map(|c| c.worst)
            .fold(0.0, f64::max)
    };
    let _ = writeln!(
        s,
        "- the shape vector on the raw state: kernel::shape weights the squared norms by μ (μ_ρ|ρ|², √(μ_ρμ_λ)ρ·λ) and does not renormalise; refine_test.shape_vec weights the vectors by √μ, squares, and divides by np.linalg.norm; worst {:.3e} absolute",
        worst(&["n0, raw", "n1, raw", "n2, raw"])
    );
    let _ = writeln!(
        s,
        "- the shape vector on the decode: C (CoM, I = 1, ρ onto +x) and the rest-start blocks' reconstruction before the map; worst {:.3e} absolute",
        worst(&["n0, decoded", "n1, decoded", "n2, decoded"])
    );
    if let Some(c) = col("V₀") {
        let _ = writeln!(
            s,
            "- the dimensionless energy: the descriptor's V₀ on the canonical state against tb.energy's pairwise sum on the raw one, times √I/M^(5/2): worst {:.3e} relative (row {})",
            c.worst, c.at
        );
    }
    if let Some(c) = col("r_min_pair_0") {
        let _ = writeln!(
            s,
            "- the least pair distance: the descriptor's on the canonical state against tb.pair_dists over tb_ftle.inertia's hyperradius: worst {:.3e} relative (row {})",
            c.worst, c.at
        );
    }
    let _ = writeln!(
        s,
        "\nProposed tolerances: {POSITION_ULPS} ulp of max(|c ± h|) on Φ's position (derived from the two orders' roundings, \
         not fitted); {ALGEBRA_TOL:e} on the algebra (absolute on n, relative on V₀ and r_min_pair_0). Every measured \
         difference above is inside its bound: {}.",
        if r.pass() { "yes" } else { "NO" }
    );
    let _ = writeln!(
        s,
        "Sign convention, not a tolerance: the reference's n₂ is the kernel's −n₂."
    );
    s
}

/// `--record`: re-runs the recording script with `python3 -I`, writing the fixture under `root`.
pub fn record(root: &Path) -> Result<PathBuf, String> {
    let out = root.join(FIXTURE);
    let status = std::process::Command::new("python3")
        .arg("-I")
        .arg(root.join(SCRIPT))
        .arg("--out")
        .arg(&out)
        .status()
        .map_err(|e| format!("cannot run python3: {e}"))?;
    if status.success() {
        Ok(out)
    } else {
        Err(format!("{SCRIPT} failed ({status})"))
    }
}

/// `golden bodyplane [--propose | --record]` under the workspace `root`.
pub fn cli(root: &Path, args: &[&str]) -> Result<(), String> {
    match args {
        [] => {
            let fx = Fixture::load(root)?;
            let r = compare(&fx, Convention::Negated, Placement::Chart);
            for h in &fx.header {
                println!("{h}");
            }
            print!("{}", r.text());
            if r.pass() {
                Ok(())
            } else {
                Err("the body plane disagrees with the Python reference's fixture".into())
            }
        }
        ["--propose"] => {
            print!("{}", proposal(&Fixture::load(root)?));
            Ok(())
        }
        ["--record"] => {
            let out = record(root)?;
            println!(
                "recorded {}; update its row in fixtures/golden/BASELINES.md with the new SHA-256",
                out.display()
            );
            Ok(())
        }
        _ => Err(format!(
            "unrecognised arguments `{}`; usage: golden bodyplane [--propose | --record]",
            args.join(" ")
        )),
    }
}
