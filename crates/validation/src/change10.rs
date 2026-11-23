//! The change-10 re-run (TASK-M3-36; R-35; REQ-VAL-036, REQ-VAL-035): the Python cross-check's Aarseth–Zare cases
//! re-run against the patched NumPy reference, and the divergence-vs-horizon table regenerated
//! (`cargo xtask gate change10-rerun`; dd_validation_orbits §0.1 and §5, dd_predictability_horizon §7).
//!
//! **The reference side.** `docs/reference/prin-rs/tools/xcheck/dump_ref.py`, run unmodified on its six AZ cases
//! (`az_t0p5` … `az_t13`): the 3 × 3 near-field window of `tb.burrau_grid`, masses `(3, 4, 5)` at rest, η = 0.01,
//! `dtau_mode = per-step-interval`, `clamp_final = 1`, and `tb_az.integrate_az`'s secant landing on by default
//! (`LAND_ITERATE_DEFAULT`): the overshoot patch adopted. CI has no numpy, so the dumps are recorded under
//! `fixtures/gates/change10-rerun/` (`--record` re-runs them) and read here. The task names `workbench/tb_az.py`; that
//! file is not in the repository, and the imported reference's `reference/tb_az.py` is the one run.
//!
//! **The Rust side.** The kernel's Aarseth–Zare march (`kernel::regularisation::az::march`, RK4), driven as
//! `dump_ref.py` drives the NumPy one: one sync interval per call, interval `k` of length `(k+1)·T/n − k·T/n`, the
//! reference's own floats, from the Cartesian state the last interval returned. The initial positions are read from the
//! body plane's recorded fixture (its `xcheck_near_field` window: the reference's own linspace nodes), so both sides
//! start from bit-identical states. The step control is set to the reference's: η = 0.01, `TINY = 1e-300`, the
//! predictive limit off (`f = 0`; the reference has none), no step budget, the landing clamp on.
//!
//! **What still differs, by construction.** The landing: the reference's secant stops within `1e-14·dt_left` of the
//! boundary and sets the clock to it; the kernel's secant and bisection land with no tolerance (R-998). The state
//! carried between intervals: the reference carries velocities, the kernel momenta (`v = p/m` at registration). Both
//! inject an ulp-sized difference at every boundary, so the intercept is a few ulp per boundary, not one.
//!
//! **The criteria** (the archived kernel brief's, fixed before the run): the largest position or velocity difference
//! over the nine trajectories at most `1e-13` at `t ≤ 2`, where chaos has had no time to amplify, and at most `1e-10`
//! at `t = 13`; and the reference body chosen at every sync boundary the same on both sides. The control runs the
//! kernel without the landing fix (clamp, secant and bisection off: the overshoot the patch removed) against the same
//! patched reference, and must fail them.
//!
//! **The horizon figure.** From the table, `λ` is the least-squares slope of `ln max|Δ|` on `t` over `t ≥ 2`, and the
//! f64 horizon is the time the fitted divergence reaches the system's size, the Burrau hyperradius `R = √5`:
//! `t_f64 = (ln R − b)/λ`, `b` the fit's intercept. It is the method R-119 hands to the GPU measurement: the same fit
//! on a CPU-f64 against GPU-f32 table.

use std::fmt::Write as _;
use std::path::Path;

use kernel::occupant::rk4::Rk4;
use kernel::occupant::State;
use kernel::regularisation::az::{march, AzCount, AzParams, End, ExplicitTest};

use crate::az::Quiet;
use crate::bodyplane::Fixture as BodyFixture;
use crate::gate::report::Verdict;
use crate::gate::reregistration::BURRAU_M;
use crate::gate::{MeasurementConfig, MeasurementControl, MeasurementGate};

/// The gate's name.
pub const NAME: &str = "change10-rerun";

/// The cases, `(name, t_max)`, as `tools/xcheck/cases.py`'s `AZ_HORIZONS`.
pub const CASES: [(&str, f64); 6] = [
    ("az_t0p5", 0.5),
    ("az_t1", 1.0),
    ("az_t2", 2.0),
    ("az_t4", 4.0),
    ("az_t8", 8.0),
    ("az_t13", 13.0),
];

/// `cases.py`'s `SYNC_INTERVAL`, `13/32`.
pub const SYNC_INTERVAL: f64 = 13.0 / 32.0;

/// The step factor the cases run.
pub const ETA: f64 = 0.01;

/// The bound at `t ≤ 2` (the module's note).
pub const EARLY: f64 = 1e-13;

/// The bound at `t = 13` (the module's note).
pub const LATE: f64 = 1e-10;

/// The Burrau hyperradius, `√5`: the system's size, where the horizon is read.
pub const SIZE: f64 = 2.236_067_977_499_79;

/// `cases.py`'s `n_sync_for`: `max(1, round(t/SYNC_INTERVAL))`, Python's round half to even.
pub fn n_sync(t_max: f64) -> u32 {
    let x = t_max / SYNC_INTERVAL;
    let r = x.round();
    let r = if (x - x.trunc()).abs() == 0.5 && r % 2.0 != 0.0 {
        r - x.signum()
    } else {
        r
    };
    (r as u32).max(1)
}

/// Which landing the kernel runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Landing {
    /// The landing fix: clamp, secant (4) and bisection (64), R-998.
    Fix,
    /// The control: no clamp, no secant, no bisection, the overshoot the patch removed.
    Off,
}

/// The step control at the reference's settings, with `landing`, on an interval of length `dt`.
pub fn params(dt: f64, landing: Landing) -> AzParams<f64> {
    let fix = landing == Landing::Fix;
    AzParams {
        eta: ETA,
        delta_sync: dt,
        tiny: 1e-300,
        predictive_f: 0.0,
        step_budget_rate: 0.0,
        n_min: u32::MAX,
        clamp: fix,
        secant_tries: if fix { 4 } else { 0 },
        bisect_tries: if fix { 64 } else { 0 },
    }
}

/// One trajectory's end: positions and velocities in the reference's column order, the reference body per sync, the
/// physical time.
#[derive(Clone, Debug, PartialEq)]
pub struct End12 {
    pub state: [f64; 12],
    pub refs: Vec<u32>,
    pub t: f64,
    pub ended: bool,
}

/// The kernel's march of one trajectory from body 0 at `at` to `t_max` over `n` intervals, as `dump_ref.py` steps it.
pub fn run(at: [f64; 2], t_max: f64, n: u32, landing: Landing) -> End12 {
    let mut r = crate::gate::reregistration::BURRAU_R;
    r[0] = at;
    let mut s = State {
        m: BURRAU_M,
        r,
        p: [[0.0; 2]; 3],
    };
    let mut count = AzCount::<f64>::new();
    let mut refs = Vec::new();
    let mut t_prev = 0.0;
    let mut t = 0.0;
    for k in 0..n {
        let t_target = f64::from(k + 1) * t_max / f64::from(n);
        let p = params(t_target - t_prev, landing);
        let out = march::<Rk4, f64, Quiet, ExplicitTest>(
            &s,
            0,
            1,
            count,
            &p,
            &mut Quiet,
            &mut ExplicitTest,
        );
        refs.push(out.count.reference);
        s = out.state;
        count = out.count;
        t = if out.boundary == 1 {
            t_target
        } else {
            t_prev + out.t
        };
        t_prev = t_target;
        if count.end != End::Marching {
            break;
        }
    }
    let v = |i: usize| [s.p[i][0] / s.m[i], s.p[i][1] / s.m[i]];
    let st = [
        s.r[0][0],
        s.r[0][1],
        s.r[1][0],
        s.r[1][1],
        s.r[2][0],
        s.r[2][1],
        v(0)[0],
        v(0)[1],
        v(1)[0],
        v(1)[1],
        v(2)[0],
        v(2)[1],
    ];
    End12 {
        state: st,
        refs,
        t,
        ended: count.end != End::Marching,
    }
}

/// One recorded reference dump: its header and, per row, the state, the time and the reference bodies.
#[derive(Clone, Debug, PartialEq)]
pub struct Dump {
    pub header: Vec<String>,
    pub rows: Vec<([f64; 12], f64, f64, Vec<u32>)>,
}

/// Reads `fixtures/gates/change10-rerun/ref_<case>.tsv` under `root`.
pub fn dump(root: &Path, case: &str) -> Result<Dump, String> {
    let path = root
        .join("fixtures/gates")
        .join(NAME)
        .join(format!("ref_{case}.tsv"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut header = Vec::new();
    let mut rows = Vec::new();
    for line in text.lines().filter(|l| !l.is_empty()) {
        if line.starts_with('#') {
            header.push(line.to_owned());
            continue;
        }
        let v: Vec<f64> = line
            .split('\t')
            .map(|c| {
                c.parse::<f64>()
                    .map_err(|e| format!("{}: `{c}`: {e}", path.display()))
            })
            .collect::<Result<_, _>>()?;
        if v.len() < 18 {
            return Err(format!("{}: a row of {} cells", path.display(), v.len()));
        }
        let mut st = [0.0; 12];
        st.copy_from_slice(&v[1..13]);
        let refs = v[17..].iter().map(|&x| x as u32).collect();
        rows.push((st, v[13], v[15], refs));
    }
    Ok(Dump { header, rows })
}

/// One row of the table.
#[derive(Clone, Debug, PartialEq)]
pub struct TableRow {
    pub case: &'static str,
    pub t_max: f64,
    pub n_sync: u32,
    /// The largest position or velocity difference.
    pub max_abs: f64,
    /// The largest relative difference.
    pub max_rel: f64,
    /// Whether every sync boundary's reference body agrees.
    pub refs_equal: bool,
    /// The largest relative energy drift on the reference side.
    pub drift: f64,
    /// Whether any kernel trajectory ended early.
    pub ended: bool,
}

/// The divergence-vs-horizon table under `landing`.
pub fn table(root: &Path, landing: Landing) -> Result<Vec<TableRow>, String> {
    let fx = BodyFixture::load(root)?;
    let k = fx
        .cases
        .iter()
        .position(|c| c.name == "xcheck_near_field")
        .ok_or("the body plane fixture has no xcheck_near_field window")?;
    let mut ics: Vec<(u32, u32, [f64; 2])> = fx
        .rows
        .iter()
        .filter(|r| r.case == k)
        .map(|r| (r.j, r.i, [r.x, r.y]))
        .collect();
    ics.sort_by_key(|&(j, i, _)| (j, i));
    let mut out = Vec::new();
    for (case, t_max) in CASES {
        let d = dump(root, case)?;
        let n = n_sync(t_max);
        let want = format!("n_sync={n} eta={ETA}");
        if !d
            .header
            .iter()
            .any(|h| h.contains(&want) && h.contains("clamp_final=1"))
        {
            return Err(format!(
                "ref_{case}.tsv was not recorded at {want} with clamp_final=1"
            ));
        }
        if d.rows.len() != ics.len() {
            return Err(format!(
                "ref_{case}.tsv has {} rows, not {}",
                d.rows.len(),
                ics.len()
            ));
        }
        let mut row = TableRow {
            case,
            t_max,
            n_sync: n,
            max_abs: 0.0,
            max_rel: 0.0,
            refs_equal: true,
            drift: 0.0,
            ended: false,
        };
        for ((_, _, at), (want, _t, drift, refs)) in ics.iter().zip(&d.rows) {
            let got = run(*at, t_max, n, landing);
            row.ended |= got.ended;
            row.refs_equal &= &got.refs == refs;
            row.drift = row.drift.max(*drift);
            for (a, b) in got.state.iter().zip(want) {
                let diff = (a - b).abs();
                let diff = if diff.is_nan() { f64::INFINITY } else { diff };
                row.max_abs = row.max_abs.max(diff);
                row.max_rel = row.max_rel.max(diff / a.abs().max(b.abs()).max(1e-300));
            }
        }
        out.push(row);
    }
    Ok(out)
}

/// The least-squares fit `ln max|Δ| = b + λt` over the rows at `t ≥ 2` with a nonzero difference: `(λ, b)`.
pub fn fit(rows: &[TableRow]) -> Option<(f64, f64)> {
    let pts: Vec<(f64, f64)> = rows
        .iter()
        .filter(|r| r.t_max >= 2.0 && r.max_abs > 0.0 && r.max_abs.is_finite())
        .map(|r| (r.t_max, r.max_abs.ln()))
        .collect();
    if pts.len() < 2 {
        return None;
    }
    let n = pts.len() as f64;
    let (sx, sy) = pts.iter().fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
    let (mx, my) = (sx / n, sy / n);
    let sxy: f64 = pts.iter().map(|(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = pts.iter().map(|(x, _)| (x - mx) * (x - mx)).sum();
    let lambda = sxy / sxx;
    Some((lambda, my - lambda * mx))
}

/// Whether the table meets the criteria, and each failure.
pub fn judge(rows: &[TableRow]) -> Vec<String> {
    let mut bad = Vec::new();
    for r in rows {
        if r.t_max <= 2.0 && (r.max_abs.is_nan() || r.max_abs > EARLY) {
            bad.push(format!(
                "{}: max|Δ| {:.3e} > {EARLY:e} at t ≤ 2",
                r.case, r.max_abs
            ));
        }
        if r.t_max == 13.0 && (r.max_abs.is_nan() || r.max_abs > LATE) {
            bad.push(format!(
                "{}: max|Δ| {:.3e} > {LATE:e} at t = 13",
                r.case, r.max_abs
            ));
        }
        if !r.refs_equal {
            bad.push(format!(
                "{}: the reference body differs at a sync boundary",
                r.case
            ));
        }
        if r.ended {
            bad.push(format!(
                "{}: a kernel trajectory ended before t_max",
                r.case
            ));
        }
    }
    bad
}

/// The table as text, with the growth between rows, the fit and the horizon.
pub fn text(rows: &[TableRow], landing: Landing) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "kernel AZ ({}) against the patched NumPy reference (dtau_mode=per-step-interval, clamp_final=1, land_iterate on)",
        match landing {
            Landing::Fix => "landing fix on",
            Landing::Off => "landing fix off: the control",
        }
    );
    let _ = writeln!(
        s,
        "| case | t_max | n_sync | max \\|Δ\\| | max rel | refs | drift (ref) | growth |\n|---|---|---|---|---|---|---|---|"
    );
    let mut prev: Option<&TableRow> = None;
    for r in rows {
        let growth = match prev {
            Some(p) if p.max_abs > 0.0 && r.max_abs > 0.0 => {
                format!(
                    "exp({:.2} t)",
                    (r.max_abs / p.max_abs).ln() / (r.t_max - p.t_max)
                )
            }
            _ => String::new(),
        };
        let _ = writeln!(
            s,
            "| {} | {} | {} | {:.3e} | {:.3e} | {} | {:.2e} | {} |",
            r.case,
            r.t_max,
            r.n_sync,
            r.max_abs,
            r.max_rel,
            if r.refs_equal { "same" } else { "DIFFER" },
            r.drift,
            growth
        );
        prev = Some(r);
    }
    match fit(rows) {
        Some((lambda, b)) => {
            let _ = writeln!(
                s,
                "\nfit over t ≥ 2: ln max|Δ| = {b:.3} + {lambda:.3}·t (intercept e^b = {:.2e}); the divergence reaches R = √5 at t_f64 = {:.1}",
                b.exp(),
                (SIZE.ln() - b) / lambda
            );
        }
        None => {
            let _ = writeln!(s, "\nfit over t ≥ 2: too few nonzero rows");
        }
    }
    let bad = judge(rows);
    let _ = writeln!(
        s,
        "criteria (≤ {EARLY:e} at t ≤ 2, ≤ {LATE:e} at t = 13, the same reference bodies): {}",
        if bad.is_empty() {
            "met".to_owned()
        } else {
            format!("NOT met: {}", bad.join("; "))
        }
    );
    s
}

/// `--record`: re-runs `dump_ref.py` with `python3 -I` for each case, writing the dumps under `root`.
pub fn record(root: &Path) -> Result<(), String> {
    for (case, _) in CASES {
        let out = root
            .join("fixtures/gates")
            .join(NAME)
            .join(format!("ref_{case}.tsv"));
        let status = std::process::Command::new("python3")
            .arg("-I")
            .arg(root.join("docs/reference/prin-rs/tools/xcheck/dump_ref.py"))
            .args(["--case", case, "--out"])
            .arg(&out)
            .env_remove("PRIN_DTAU_MODE")
            .env_remove("PRIN_CLAMP_FINAL")
            .current_dir(root)
            .status()
            .map_err(|e| format!("cannot run python3: {e}"))?;
        if !status.success() {
            return Err(format!("dump_ref.py --case {case} failed ({status})"));
        }
    }
    Ok(())
}

/// A control input: which landing the kernel runs.
#[derive(serde::Deserialize)]
struct Control {
    description: String,
    landing: String,
}

/// The change-10 re-run, a measurement (outside numbers: reported, never blocking).
pub struct Change10Gate;

impl MeasurementGate for Change10Gate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let c: Control =
            serde::Deserialize::deserialize(fixture).map_err(|e| format!("{name}: {e}"))?;
        let landing = match c.landing.as_str() {
            "fix" => Landing::Fix,
            "off" => Landing::Off,
            other => return Err(format!("{name}: no landing `{other}` (fix or off)")),
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let rows = table(&root, landing)?;
        let t = text(&rows, landing);
        println!("{t}");
        Ok(MeasurementControl {
            description: c.description.clone(),
            verdict: if judge(&rows).is_empty() {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            markdown: format!("# {NAME}: {name}\n\n{}\n\n{t}", c.description),
        })
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        match args {
            ["--record"] => record(root),
            ["--shadow"] => {
                println!("{}", shadow_text(&shadow(root)?));
                Ok(())
            }
            ["--table"] => {
                for landing in [Landing::Fix, Landing::Off] {
                    println!("{}", text(&table(root, landing)?, landing));
                }
                Ok(())
            }
            _ => Err(format!(
                "{NAME}: unrecognised flags `{}`; --record, --table or --shadow",
                args.join(" ")
            )),
        }
    }
}

/// The shadow run's horizons.
pub const SHADOW_TIMES: [f64; 9] = [2.0, 4.0, 8.0, 13.0, 16.0, 20.0, 26.0, 32.0, 40.0];

/// The bound-phase rate the shadow run is judged against, dd_predictability_horizon §7.2: `λ ∈ [0.6, 0.8]`.
pub const LAMBDA_72: [f64; 2] = [0.6, 0.8];

/// The shadow run's seed: the moved body's `x` shifted by `1e-14`. Measured: a one-ulp shift is erased at the first
/// registration (the reference body is 2, so `R₁ = (2.2e-16, 4)`, and the Levi-Civita inverse's `(|R| + x)/2` rounds
/// it away), so both copies were bitwise equal to `t = 40`. The seed's size sets the intercept, not `λ`.
pub const SHADOW_SEED: f64 = 1e-14;

/// The shadow run (kernel only, no Python): each of the cross-check's nine trajectories against a copy whose moved
/// body's `x` is [`SHADOW_SEED`] larger, both on the production grid `Δ_sync = 13/32`, η = 0.01, the landing fix on; the largest
/// position or velocity difference over the nine at each of [`SHADOW_TIMES`]. Criterion, fixed before the run: the
/// least-squares `λ` of `ln max|Δ|` over `t ≥ 4` lies in [`LAMBDA_72`]; then `(ln R − b)/λ` is the f64 figure.
pub fn shadow(root: &Path) -> Result<Vec<(f64, f64)>, String> {
    let fx = BodyFixture::load(root)?;
    let k = fx
        .cases
        .iter()
        .position(|c| c.name == "xcheck_near_field")
        .ok_or("the body plane fixture has no xcheck_near_field window")?;
    let ics: Vec<[f64; 2]> = fx
        .rows
        .iter()
        .filter(|r| r.case == k)
        .map(|r| [r.x, r.y])
        .collect();
    let mut out = Vec::new();
    for t in SHADOW_TIMES {
        let n = (t / SYNC_INTERVAL).round() as u32;
        let mut worst: f64 = 0.0;
        for at in &ics {
            let a = run(*at, f64::from(n) * SYNC_INTERVAL, n, Landing::Fix);
            let b = run(
                [at[0] + SHADOW_SEED, at[1]],
                f64::from(n) * SYNC_INTERVAL,
                n,
                Landing::Fix,
            );
            for (x, y) in a.state.iter().zip(&b.state) {
                let d = (x - y).abs();
                worst = worst.max(if d.is_nan() { f64::INFINITY } else { d });
            }
        }
        out.push((f64::from(n) * SYNC_INTERVAL, worst));
    }
    Ok(out)
}

/// The shadow run as text, with its fit and verdict.
pub fn shadow_text(points: &[(f64, f64)]) -> String {
    let mut s = String::from("| t | max |Δ| (x₀ + 1e-14) |\n|---|---|\n");
    for (t, d) in points {
        let _ = writeln!(s, "| {t:.3} | {d:.3e} |");
    }
    let rows: Vec<TableRow> = points
        .iter()
        .filter(|(t, _)| *t >= 4.0)
        .map(|&(t, d)| TableRow {
            case: "",
            t_max: t,
            n_sync: 0,
            max_abs: d,
            max_rel: 0.0,
            refs_equal: true,
            drift: 0.0,
            ended: false,
        })
        .collect();
    if let Some((lambda, b)) = fit(&rows) {
        let inside = (LAMBDA_72[0]..=LAMBDA_72[1]).contains(&lambda);
        let _ = writeln!(
            s,
            "fit over t ≥ 4: ln max|Δ| = {b:.3} + {lambda:.3}·t (e^b = {:.2e}); reaches R = √5 at t = {:.1}; λ in §7.2's [0.6, 0.8]: {}",
            b.exp(),
            (SIZE.ln() - b) / lambda,
            if inside { "yes" } else { "NO" }
        );
    }
    s
}
