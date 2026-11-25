//! BG-4, the butterflies' two published periods decided by closure (dd_replication §5.4, §6, §8): an external target
//! of `cargo xtask gate replication --target BG-4`. A measurement: its outcome blocks nothing. What fails the command
//! is the method's own check on orbits whose period is known, or that check's control not firing.
//!
//! **The question.** The Belgrade gallery prints two periods for butterfly I (6.235641 on the main page, 6.234671 on
//! I.2.A) and two for butterfly II (7.003907, 7.003707 on I.2.B), each pair with one `(p₁, p₂)` and one energy. From
//! that `(p₁, p₂)`, read from the committed gallery CSV and never edited, the time of the return-proximity minimum near
//! one period is measured under the default occupant, with its uncertainty, and set beside both printed periods and
//! the Three Body Orbits atlas's refined periods.
//!
//! **The start.** The gallery's collinear convention, bodies at `(−1, 0)`, `(1, 0)`, `(0, 0)` with velocities
//! `(p₁, p₂)`, `(p₁, p₂)`, `(−2p₁, −2p₂)`, unit masses, `G = 1`, puts body 3 at the midpoint of the other two, the
//! `α = 0` pole the encode clamps. It is relabelled, as the figure-eight is (`tiers::figure_eight`), so the middle body
//! is in the inner pair: `(−1, 0)`, `(0, 0)`, `(1, 0)`. Then Path A's CHECK 1, `D(E(x))`, as a pixel decodes it,
//! refused where the encode clamps. Times convert by `tiers::time_scale`.
//!
//! **The occupant.** The default, Heggie (Eq. 22) × RK4, by its direct march at f64 (`compare::run`), every step-control
//! value [`StepControl::TEST`]'s with `η` from [`LADDER`]; the measurement is read at `η = 0.01`, the field backend's.
//! The plan is unclear here (the field backend marches at f32); chose f64, as the figure-eight anchor does, because the
//! measurement asks what the occupant's method gives, and f32's round-off would be a second, unrelated term.
//!
//! **The minimum.** The return proximity is the phase-space distance in the gallery's own units,
//! `d(t)² = Σᵢ ‖rᵢ(t) − rᵢ(0)‖² + ‖vᵢ(t) − vᵢ(0)‖²`. Its minimum near one period is the zero, where it changes sign
//! from − to +, of `g(t) = ½ d(d²)/dt = Σᵢ (rᵢ(t) − rᵢ(0))·vᵢ(t) + (vᵢ(t) − vᵢ(0))·aᵢ(t)`, `aᵢ` the Newtonian
//! acceleration, all read from the march's end state. A scan on the kernel's `Δ_sync` grid over a window of
//! [`WINDOW`] either side of both printed periods (so neither seeds the answer) must find exactly one − to + change of
//! `g`; its two boundaries bracket the root. The root is then found by the Illinois method, each evaluation a march
//! from the start to `t` on the grid `Δ = t/n`, `n` fixed for the search (as the figure-eight anchor lands on its
//! period), until the bracket is at most [`ROOT_TOL`] wide or [`ROOT_ITERATIONS`] are spent.
//!
//! **The uncertainty**, every term in the gallery's time units, summed linearly:
//! - `u_η`: `|t(0.02) − t(0.01)|` on the ladder [`LADDER`], which is a bound on the error at 0.01 only while the ladder
//!   converges, so the method requires `|t(0.02) − t(0.01)| ≤ |t(0.04) − t(0.02)|` (or both below [`LADDER_FLOOR`]);
//! - `u_grid`: `|t(n) − t(2n)|`, the sync grid halved at fixed `η` (pitfalls §3: each evaluation is its own grid);
//! - `u_norm`: `|t − t_pos|`, `t_pos` the minimum of the position-only distance: an orbit that does not close has a
//!   minimum whose time depends on the norm;
//! - `u_close`: `d_min/‖Ẋ(t)‖`, the time over which the start's own non-closure is passed;
//! - `u_root`: the root's final bracket width;
//! - `u_in`, for a start printed to six decimals: the start is anywhere in the box `(p₁, p₂) ±`[`DIGIT`], and `u_in` is
//!   the largest `|t(corner) − t|` over its four corners. Don't drop it for `u_close`: rounding the start moves its
//!   energy, and the period with it (`T ∝ |E|^{−3/2}`), while the rounded start still returns close to itself, so its
//!   minimum moves far more than its distance (Simó's eight-digit figure-eight: `u_close` `7·10⁻¹⁰`, its minimum
//!   `3·10⁻⁸` after the exact orbit's period, which is the period rescaled to its own energy to `6·10⁻¹⁰`). `u_in`
//!   covers the printed digits' rounding, not a source whose digits are off by more.
//!
//! **The method's check** (it may block), on four orbits whose period is known, all the atlas's refined orbits (the
//! site's own, which test closure only): the figure-eight (V.1.A, the same slice and convention as the gallery's), the
//! figure-eight's refined `(p₁, p₂)` rounded to six decimals, with its box, as the butterflies' starts are printed, and
//! the refined I.2.A and I.2.B. On each, the measured time is within half its uncertainty of the known period,
//! `|t − T| < u/2`. Its control hands the check the period off by the claimed uncertainty, `T + u` and `T − u`, each of
//! which it must reject: it does whenever the check passes, so the check passing is what makes the claimed `u` mean
//! something. The rounded figure-eight is what tests `u_in`: its rounding errors are `1.1·10⁻⁷` and `5.5·10⁻⁸`, under a
//! quarter of [`DIGIT`], so its error is under a quarter of `u_in` where `t` is linear over the box.
//!
//! **The decision.** Only where `u` is smaller than the printed periods' difference is the nearer one read. The
//! outcome: PASS where `u` undercuts the difference and the measured time is within `u` of one printed period, for both
//! butterflies; otherwise UNEXPLAINED, with the numbers. Never a failure.
//!
//! **Like for like.** The energy recomputed from the six-decimal start is printed beside the gallery's. The atlas's
//! refined periods belong to its refined starts, at another energy, so each is reported as printed and rescaled to the
//! gallery start's energy, `T·(|E_atlas|/|E|)^{3/2}`. The gallery prints the figure-eight twice too (FIGURE 8 on the
//! main page, V.1.A in the sequence), and the atlas gives its period exactly: both are measured the same way and
//! reported, as a reading of how the gallery's printed periods relate to its printed `(p₁, p₂)`.
//!
//! **Credits.** The Belgrade gallery, <http://three-body.ipb.ac.rs/>, and Šuvakov & Dmitrašinović, PRL 110, 114301
//! (2013). Periodic orbits from Three Body Orbits (<https://www.threebodyorbits.com>), CC BY 4.0, and the papers they
//! come from (`docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`, families I and V). Initial conditions in part
//! from Ricky Reusser's compilation (MIT).

use std::collections::HashMap;
use std::path::Path;

use kernel::driver::advance::Cadence;
use kernel::generated::links::Defaults;
use kernel::occupant::{Geom, Occupant, Selection, State};
use kernel::regularisation::az::End;
use kernel::regularisation::pairs::{Choice, HeggieTime};
use kernel::regularisation::sync::delta_sync;

use crate::compare::{self, Arm, Discretisation, StepControl};
use crate::gate::figure_eight::grid;
use crate::gate::replication::{Outcome, Scored, Target};
use crate::ground_truth::{path_a, tiers};
use crate::import::{invariants, unit_state, Start};

/// The target's id.
pub const ID: &str = "BG-4";

/// The gallery's CSV, under the workspace.
pub const GALLERY: &str = "docs/incoming/2026-10-26_belgrade_gallery/belgrade_gallery.csv";

/// The atlas's CSV, under the workspace.
pub const ATLAS: &str = "docs/incoming/2026-10-26_orbit_catalogues/threebodyorbits_ics.csv";

/// The `η` ladder, coarse to fine; the measurement is read at its last rung, the field backend's `η`.
pub const LADDER: [f64; 3] = [0.04, 0.02, 0.01];

/// Below this, in the gallery's time units, a ladder difference is round-off and the ladder's convergence is not read.
pub const LADDER_FLOOR: f64 = 1e-10;

/// Half the last printed digit of the gallery's `p₁`, `p₂`.
pub const DIGIT: f64 = 5e-7;

/// The scan's margin either side of the printed periods, in the gallery's time units.
pub const WINDOW: f64 = 0.5;

/// The root's bracket width at which the search stops, in the sim's time.
pub const ROOT_TOL: f64 = 1e-12;

/// The most Illinois iterations; past them the bracket's width is the root's uncertainty.
pub const ROOT_ITERATIONS: u32 = 60;

/// The gallery's credit.
pub const CREDIT_GALLERY: &str =
    "Belgrade gallery, http://three-body.ipb.ac.rs/, and Šuvakov & Dmitrašinović, PRL 110, 114301 (2013)";

/// The atlas's credit.
pub const CREDIT_ATLAS: &str = "Periodic orbits from Three Body Orbits (https://www.threebodyorbits.com), CC BY 4.0, \
                                and the papers they come from (docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md, \
                                families I and V: Šuvakov & Dmitrašinović 2013; Hristov et al. 2026; Li & Liao 2017). \
                                Initial conditions in part from Ricky Reusser's compilation (MIT).";

// ── The CSVs ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The rows of a CSV with a header line, each a map from the header's names: fields may be quoted, with `""` for a
/// quote and commas and line breaks inside.
pub fn csv(text: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let (mut record, mut field) = (Vec::new(), String::new());
    let (mut quoted, mut chars) = (false, text.chars().peekable());
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            (true, '"') => quoted = false,
            (true, c) => field.push(c),
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => record.push(std::mem::take(&mut field)),
            (false, '\r') => {}
            (false, '\n') => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            (false, c) => field.push(c),
        }
    }
    if quoted {
        return Err("an unterminated quoted field".into());
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    let mut records = records.into_iter();
    let header = records.next().ok_or("no header")?;
    records
        .enumerate()
        .map(|(i, r)| {
            if r.len() == header.len() {
                Ok(header.iter().cloned().zip(r).collect())
            } else {
                Err(format!(
                    "row {} has {} fields, the header {}",
                    i + 2,
                    r.len(),
                    header.len()
                ))
            }
        })
        .collect()
}

fn read_csv(root: &Path, path: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let text = std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"))?;
    csv(&text).map_err(|e| format!("{path}: {e}"))
}

fn field<'a>(row: &'a HashMap<String, String>, name: &str) -> Result<&'a str, String> {
    row.get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("no column `{name}`"))
}

fn number(row: &HashMap<String, String>, name: &str) -> Result<f64, String> {
    let s = field(row, name)?;
    s.trim()
        .parse()
        .map_err(|e| format!("column `{name}`: `{s}`: {e}"))
}

type Row = HashMap<String, String>;

/// The one row of `rows` whose `col` is `value`.
fn one(rows: &[Row], col: &str, value: &str, file: &str) -> Result<Row, String> {
    let found: Vec<&Row> = rows
        .iter()
        .filter(|r| r.get(col).map(String::as_str) == Some(value))
        .collect();
    match found.as_slice() {
        [r] => Ok((*r).clone()),
        _ => Err(format!(
            "{file}: {} rows with {col} = {value}, not one",
            found.len()
        )),
    }
}

/// A gallery page's orbit: its name, `(p₁, p₂)`, printed period and energy.
#[derive(Clone, Debug, PartialEq)]
pub struct Printed {
    /// The page's name.
    pub name: String,
    /// `p₁`, `p₂`.
    pub p: [f64; 2],
    /// The printed period.
    pub period: f64,
    /// The printed energy.
    pub energy: f64,
}

fn printed(gallery: &[Row], name: &str) -> Result<Printed, String> {
    let r = one(gallery, "name", name, GALLERY)?;
    Ok(Printed {
        name: name.into(),
        p: [number(&r, "p1")?, number(&r, "p2")?],
        period: number(&r, "period")?,
        energy: number(&r, "energy")?,
    })
}

/// An orbit whose period is known: its start in its own units, that period, its source's energy, and, for a start
/// printed to six decimals, its `(p₁, p₂)`, whose box `u_in` reads.
#[derive(Clone, Debug, PartialEq)]
pub struct Known {
    /// The start.
    pub start: Start,
    /// The period.
    pub period: f64,
    /// The energy its source gives.
    pub energy: f64,
    /// The printed `(p₁, p₂)` of a six-decimal start.
    pub box_p: Option<[f64; 2]>,
}

/// The atlas's refined orbit `key`, relabelled as the gallery's start is: body 3 sits at or near the midpoint of the
/// other two in the atlas's convention too.
fn atlas_known(atlas: &[Row], key: &str, label: &str) -> Result<Known, String> {
    let a = one(atlas, "key", key, ATLAS)?;
    let names = [
        "x1", "y1", "x2", "y2", "x3", "y3", "vx1", "vy1", "vx2", "vy2", "vx3", "vy3",
    ];
    let mut x = [0.0; 12];
    for (xi, n) in x.iter_mut().zip(names) {
        *xi = number(&a, &format!("ref_{n}"))?;
    }
    let m = [number(&a, "m1")?, number(&a, "m2")?, number(&a, "m3")?];
    let order = [0, 2, 1];
    Ok(Known {
        start: Start {
            name: format!("{label} (atlas, refined)"),
            citation: CREDIT_ATLAS.into(),
            g: 1.0,
            m: order.map(|i| m[i]),
            r: order.map(|i| [x[2 * i], x[2 * i + 1]]),
            v: order.map(|i| [x[6 + 2 * i], x[6 + 2 * i + 1]]),
        },
        period: number(&a, "T")?,
        energy: number(&a, "E_recomp")?,
        box_p: None,
    })
}

/// The two butterflies' main-page names, sequence labels, the task's four printed periods (which the CSV must hold),
/// and their atlas keys.
pub const BUTTERFLIES: [(&str, &str, [f64; 2], &str); 2] = [
    (
        "BUTTERFLY I",
        "I.2.A",
        [6.235641, 6.234671],
        "i_butterfly_i_2_a",
    ),
    (
        "BUTTERFLY II",
        "I.2.B",
        [7.003907, 7.003707],
        "i_butterfly_i_2_b",
    ),
];

/// The gallery's two figure-eight pages and the atlas's key for it.
pub const FIGURE_EIGHT: ([&str; 2], &str) = (["FIGURE 8", "V.1.A"], "v_figure_8_v_1_a");

/// One butterfly as the gallery gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct Butterfly {
    /// Its name on the main page.
    pub name: &'static str,
    /// Its sequence label.
    pub label: &'static str,
    /// `p₁`, `p₂`, as printed (identical on both pages).
    pub p: [f64; 2],
    /// The printed periods: the main page's, then the sequence page's.
    pub periods: [f64; 2],
    /// The printed energy.
    pub energy: f64,
    /// The atlas's refined orbit.
    pub atlas: Known,
}

/// What the target reads from the two CSVs.
#[derive(Clone, Debug, PartialEq)]
pub struct Sources {
    /// The butterflies.
    pub butterflies: Vec<Butterfly>,
    /// The atlas's refined figure-eight.
    pub figure_eight: Known,
    /// The gallery's two figure-eight pages.
    pub gallery_eights: Vec<Printed>,
}

/// The gallery's collinear start for `(p₁, p₂)`, relabelled so the middle body is in the inner pair (the module's note).
pub fn gallery_start(name: &str, p: [f64; 2]) -> Start {
    let [p1, p2] = p;
    Start {
        name: name.into(),
        citation: CREDIT_GALLERY.into(),
        g: 1.0,
        m: [1.0; 3],
        r: [[-1.0, 0.0], [0.0, 0.0], [1.0, 0.0]],
        v: [[p1, p2], [-2.0 * p1, -2.0 * p2], [p1, p2]],
    }
}

/// The sources, read from the gallery's and the atlas's CSVs under `root`.
pub fn sources(root: &Path) -> Result<Sources, String> {
    let gallery = read_csv(root, GALLERY)?;
    let atlas = read_csv(root, ATLAS)?;
    let butterflies = BUTTERFLIES
        .iter()
        .map(|&(name, label, periods, key)| {
            let (main, seq) = (printed(&gallery, name)?, printed(&gallery, label)?);
            if main.p != seq.p || main.energy != seq.energy {
                return Err(format!(
                    "{name} and {label} print different starts: {main:?}, {seq:?}"
                ));
            }
            if [main.period, seq.period] != periods {
                return Err(format!(
                    "{name}: the gallery's periods {:?}, not the task's {periods:?}",
                    [main.period, seq.period]
                ));
            }
            Ok(Butterfly {
                name,
                label,
                p: main.p,
                periods,
                energy: main.energy,
                atlas: atlas_known(&atlas, key, label)?,
            })
        })
        .collect::<Result<_, String>>()?;
    let (pages, key) = FIGURE_EIGHT;
    Ok(Sources {
        butterflies,
        figure_eight: atlas_known(&atlas, key, "V.1.A")?,
        gallery_eights: pages
            .iter()
            .map(|name| printed(&gallery, name))
            .collect::<Result<_, String>>()?,
    })
}

/// `x` rounded to six decimals.
pub fn six_decimals(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

/// The known orbits: the atlas's refined figure-eight, the same rounded to six decimals with its box, and the refined
/// butterflies. Refused where the refined figure-eight is not in the gallery's collinear form.
pub fn known_orbits(s: &Sources) -> Result<Vec<Known>, String> {
    let f = &s.figure_eight;
    let p = f.start.v[0];
    if f.start.r != gallery_start("", p).r || f.start.v[2] != p {
        return Err(format!(
            "{}: not in the gallery's collinear form",
            f.start.name
        ));
    }
    let p = p.map(six_decimals);
    let rounded = Known {
        start: gallery_start("V.1.A (atlas, refined, to six decimals)", p),
        box_p: Some(p),
        ..f.clone()
    };
    let mut out = vec![f.clone(), rounded];
    out.extend(s.butterflies.iter().map(|b| b.atlas.clone()));
    Ok(out)
}

// ── The march and the proximity ──────────────────────────────────────────────────────────────────────────────────

/// A start as a pixel decodes it, with the scales back to its own units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prepared {
    /// `D(E(x))`, in the sim's units.
    pub state: State<f64>,
    /// The sim's time per unit of the start's.
    pub time_scale: f64,
    /// `C`'s length scale `λ`: the sim's length per unit of the start's.
    pub lambda: f64,
    /// The start's `G`.
    pub g: f64,
    /// The start's total mass.
    pub mass: f64,
}

/// `s` through Path A's CHECK 1; refused where the encode clamps.
pub fn prepare(s: &Start) -> Result<Prepared, String> {
    let x = unit_state(s)?;
    let c = path_a::check1::<Defaults>(&x);
    if c.clamped {
        return Err(format!("{}: refused: the encode clamped", s.name));
    }
    Ok(Prepared {
        state: State {
            m: c.decoded.m,
            r: c.decoded.r,
            p: c.decoded.p,
        },
        time_scale: tiers::time_scale(s, c.lambda),
        lambda: c.lambda,
        g: s.g,
        mass: s.m.iter().sum(),
    })
}

/// A state in the start's own units: masses, positions, velocities.
type Phys = ([f64; 3], [[f64; 2]; 3], [[f64; 2]; 3]);

impl Prepared {
    /// `x`, a state in the sim's units, in the start's: lengths `/λ`, velocities `·T_s/λ`, masses `·M` (up to `C`'s
    /// rotation, which no distance below sees).
    fn phys(&self, x: &State<f64>) -> Phys {
        let v = |i: usize| x.p[i].map(|p| p / x.m[i] * self.time_scale / self.lambda);
        (
            x.m.map(|m| m * self.mass),
            x.r.map(|r| r.map(|c| c / self.lambda)),
            [v(0), v(1), v(2)],
        )
    }

    /// The Newtonian accelerations in the start's units.
    fn accelerations(&self, (m, r, _): &Phys) -> [[f64; 2]; 3] {
        let mut a = [[0.0; 2]; 3];
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    let d = [r[j][0] - r[i][0], r[j][1] - r[i][1]];
                    let s = d[0].hypot(d[1]);
                    let k = self.g * m[j] / (s * s * s);
                    a[i][0] += k * d[0];
                    a[i][1] += k * d[1];
                }
            }
        }
        a
    }
}

/// The distance whose minimum is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Norm {
    /// Positions and velocities: the return proximity.
    Phase,
    /// Positions only: the comparison `u_norm` reads.
    Position,
}

/// The proximity at one state, in the start's units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Proximity {
    /// `d`.
    pub d: f64,
    /// `g = ½ d(d²)/dt`.
    pub g: f64,
    /// `‖Ẋ‖`, the speed through the norm's space.
    pub speed: f64,
}

/// The proximity of `x` to the start `p.state` under `norm`.
pub fn proximity(p: &Prepared, x: &State<f64>, norm: Norm) -> Proximity {
    let (x0, now) = (p.phys(&p.state), p.phys(x));
    let a = p.accelerations(&now);
    let w = match norm {
        Norm::Phase => 1.0,
        Norm::Position => 0.0,
    };
    let (mut d2, mut g, mut s2) = (0.0, 0.0, 0.0);
    let flat = |x: &[[f64; 2]; 3]| x.concat();
    let (r0, v0, r, v, a) = (
        flat(&x0.1),
        flat(&x0.2),
        flat(&now.1),
        flat(&now.2),
        flat(&a),
    );
    for k in 0..6 {
        let (dr, dv) = (r[k] - r0[k], v[k] - v0[k]);
        d2 += dr * dr + w * dv * dv;
        g += dr * v[k] + w * dv * a[k];
        s2 += v[k] * v[k] + w * a[k] * a[k];
    }
    Proximity {
        d: d2.sqrt(),
        g,
        speed: s2.sqrt(),
    }
}

/// The default occupant's arm at `eta` on `d`.
pub fn arm(eta: f64, d: Discretisation) -> Arm {
    Arm {
        stepper: Selection::Production(Occupant::Rk4),
        regularisation: Choice::Heggie(HeggieTime::Eq22),
        step_control: StepControl {
            eta,
            ..StepControl::TEST
        },
        discretisation: d,
    }
}

/// The state at sim time `t`, marched from the start on the grid `t/n`.
pub fn march_to(p: &Prepared, t: f64, n: u32, eta: f64) -> Result<State<f64>, String> {
    let d = grid(t, n).ok_or_else(|| format!("no grid of {n} boundaries lands on {t}"))?;
    let run = compare::run(&arm(eta, d), &p.state);
    if run.end != End::Marching || run.boundary != n {
        return Err(format!(
            "the march to {t} ended {:?} on boundary {} of {n}",
            run.end, run.boundary
        ));
    }
    Ok(run.state)
}

/// Every sync boundary's state.
struct Collect(Vec<(f64, State<f64>)>);

impl Cadence<f64> for Collect {
    fn after_step(&mut self, _state: &mut State<f64>, _geom: &mut Geom<f64>) {}
    fn done(&self) -> bool {
        false
    }
    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.0.push((f64::from(index) * quantum, *state));
    }
}

/// The scan: the one − to + change of `g` between sim times `lo` and `hi`, on the kernel's `Δ_sync` grid, as the
/// bracketing pair of boundary times. Refused where there is none or more than one.
pub fn scan(p: &Prepared, eta: f64, lo: f64, hi: f64) -> Result<[f64; 2], String> {
    let d = Discretisation {
        delta_sync: delta_sync::<f64>(),
        horizon: hi,
    };
    let k = d.boundaries().ok_or("the scan has no boundary")?;
    let mut collect = Collect(Vec::new());
    let run = compare::run_sampled(&arm(eta, d), &p.state, &mut collect);
    if run.end != End::Marching || run.boundary != k {
        return Err(format!(
            "the scan ended {:?} on boundary {} of {k}",
            run.end, run.boundary
        ));
    }
    let g: Vec<(f64, f64)> = collect
        .0
        .iter()
        .filter(|(t, _)| *t >= lo)
        .map(|(t, s)| (*t, proximity(p, s, Norm::Phase).g))
        .collect();
    let changes: Vec<[f64; 2]> = g
        .windows(2)
        .filter(|w| w[0].1 < 0.0 && w[1].1 >= 0.0)
        .map(|w| [w[0].0, w[1].0])
        .collect();
    match changes.as_slice() {
        [b] => Ok(*b),
        _ => Err(format!(
            "{} minima of the return proximity in the window [{lo}, {hi}] (sim time), not one",
            changes.len()
        )),
    }
}

/// A located minimum, in the sim's time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Minimum {
    /// Its time.
    pub t: f64,
    /// The final bracket's width.
    pub width: f64,
    /// The proximity there.
    pub at: Proximity,
}

/// The root of `g` under `norm` in `bracket`, each evaluation a march on the grid `t/n` (the module's note).
pub fn root(
    p: &Prepared,
    bracket: [f64; 2],
    n: u32,
    eta: f64,
    norm: Norm,
) -> Result<Minimum, String> {
    let eval = |t: f64| march_to(p, t, n, eta).map(|s| proximity(p, &s, norm));
    let [mut a, mut b] = bracket;
    let (mut fa, mut fb) = (eval(a)?.g, eval(b)?.g);
    if !(fa < 0.0 && fb >= 0.0) {
        return Err(format!(
            "g does not change sign from − to + on [{a}, {b}]: {fa:e}, {fb:e}"
        ));
    }
    // The Illinois method: regula falsi, halving the kept end's value when the same end is kept twice.
    let mut side = 0i8;
    for _ in 0..ROOT_ITERATIONS {
        if b - a <= ROOT_TOL || fb == 0.0 {
            break;
        }
        let c = (b - fb * (b - a) / (fb - fa)).clamp(a, b);
        let fc = eval(c)?.g;
        if fc < 0.0 {
            (a, fa) = (c, fc);
            if side == -1 {
                fb /= 2.0;
            }
            side = -1;
        } else {
            (b, fb) = (c, fc);
            if side == 1 {
                fa /= 2.0;
            }
            side = 1;
        }
    }
    let t = if fb == 0.0 { b } else { 0.5 * (a + b) };
    Ok(Minimum {
        t,
        width: if fb == 0.0 { 0.0 } else { b - a },
        at: eval(t)?,
    })
}

// ── The measurement ──────────────────────────────────────────────────────────────────────────────────────────────

/// A measured minimum and its uncertainty's terms, every time in the start's own units.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    /// The minimum's time at `η = 0.01`, the phase-space norm, `n` boundaries.
    pub t: f64,
    /// The time at each rung of [`LADDER`].
    pub ladder: [f64; 3],
    /// The time on the grid `2n`.
    pub t_2n: f64,
    /// The position-only minimum's time.
    pub t_position: f64,
    /// `d` at the minimum.
    pub d_min: f64,
    /// `‖Ẋ‖` there.
    pub speed: f64,
    /// The root's final bracket width.
    pub width: f64,
    /// The times at the four corners of the input box, where there is one.
    pub corners: Option<[f64; 4]>,
}

impl Measured {
    /// `u_η`.
    pub fn u_eta(&self) -> f64 {
        (self.ladder[1] - self.ladder[2]).abs()
    }
    /// `u_grid`.
    pub fn u_grid(&self) -> f64 {
        (self.t - self.t_2n).abs()
    }
    /// `u_norm`.
    pub fn u_norm(&self) -> f64 {
        (self.t - self.t_position).abs()
    }
    /// `u_close`.
    pub fn u_close(&self) -> f64 {
        self.d_min / self.speed
    }
    /// `u_in`: 0 where there is no box.
    pub fn u_in(&self) -> f64 {
        self.corners.map_or(0.0, |c| {
            c.iter().map(|x| (x - self.t).abs()).fold(0.0, f64::max)
        })
    }
    /// The uncertainty, every term summed.
    pub fn u(&self) -> f64 {
        self.u_eta() + self.u_grid() + self.u_norm() + self.u_close() + self.width + self.u_in()
    }
    /// The measurement with every time it read moved by `dt`: the same uncertainty about `t + dt`.
    pub fn shifted(&self, dt: f64) -> Measured {
        Measured {
            t: self.t + dt,
            ladder: self.ladder.map(|t| t + dt),
            t_2n: self.t_2n + dt,
            t_position: self.t_position + dt,
            corners: self.corners.map(|c| c.map(|t| t + dt)),
            ..self.clone()
        }
    }
    /// The terms, as a line.
    pub fn terms(&self) -> String {
        format!(
            "u = {:.2e} = u_η {:.2e} + u_grid {:.2e} + u_norm {:.2e} + u_close {:.2e} + u_root {:.2e} + u_in {:.2e}",
            self.u(),
            self.u_eta(),
            self.u_grid(),
            self.u_norm(),
            self.u_close(),
            self.width,
            self.u_in()
        )
    }
    /// The readings behind the terms, as a line.
    pub fn readings(&self) -> String {
        format!(
            "ladder η {LADDER:?}: {:.12?}; grid 2n {:.12}; position norm {:.12}; d_min {:.2e}, ‖Ẋ‖ {:.3}; corners {:.12?}",
            self.ladder, self.t_2n, self.t_position, self.d_min, self.speed, self.corners
        )
    }
}

/// The ladder converges: the finer difference is at most the coarser, or both are below [`LADDER_FLOOR`].
pub fn converges(ladder: [f64; 3]) -> bool {
    let (coarse, fine) = ((ladder[0] - ladder[1]).abs(), (ladder[1] - ladder[2]).abs());
    fine <= coarse || (coarse < LADDER_FLOOR && fine < LADDER_FLOOR)
}

/// The minimum near one period of `s`, its period expected in `[lo, hi]` (the start's units), the input box's corners
/// measured where `box_p` gives a six-decimal start's `(p₁, p₂)`.
pub fn measure(s: &Start, lo: f64, hi: f64, box_p: Option<[f64; 2]>) -> Result<Measured, String> {
    let p = prepare(s)?;
    let ts = p.time_scale;
    let window = [(lo - WINDOW) * ts, (hi + WINDOW) * ts];
    let bracket = scan(&p, LADDER[2], window[0], window[1])?;
    let n = (bracket[1] / delta_sync::<f64>()).ceil() as u32;
    let find = |p: &Prepared, eta: f64, n: u32, norm: Norm| root(p, bracket, n, eta, norm);
    let mut ladder = [0.0; 3];
    for (t, &eta) in ladder.iter_mut().zip(&LADDER) {
        *t = find(&p, eta, n, Norm::Phase)?.t / ts;
    }
    if !converges(ladder) {
        return Err(format!(
            "{}: the η ladder {LADDER:?} does not converge: {ladder:?}",
            s.name
        ));
    }
    let main = find(&p, LADDER[2], n, Norm::Phase)?;
    let corners = match box_p {
        None => None,
        Some([p1, p2]) => {
            let mut c = [0.0; 4];
            let signs = [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)];
            for (ci, (s1, s2)) in c.iter_mut().zip(signs) {
                let corner = gallery_start(&s.name, [p1 + s1 * DIGIT, p2 + s2 * DIGIT]);
                let q = prepare(&corner)?;
                *ci = find(&q, LADDER[2], n, Norm::Phase)?.t / q.time_scale;
            }
            Some(c)
        }
    };
    Ok(Measured {
        t: main.t / ts,
        ladder,
        t_2n: find(&p, LADDER[2], 2 * n, Norm::Phase)?.t / ts,
        t_position: find(&p, LADDER[2], n, Norm::Position)?.t / ts,
        d_min: main.at.d,
        speed: main.at.speed,
        width: main.width / ts,
        corners,
    })
}

// ── The method's check, its control, and the decision ───────────────────────────────────────────────────────────

/// Whether `m` is within half its uncertainty of the period `period`.
pub fn within_half(m: &Measured, period: f64) -> bool {
    (m.t - period).abs() < m.u() / 2.0
}

/// The method's check on a known orbit: panics unless the measured time is within half its uncertainty of `period`.
pub fn check_known(name: &str, m: &Measured, period: f64) {
    assert!(
        within_half(m, period),
        "the method missed the known period of {name}: measured {:.12} against {period:.12}, off by {:.2e}, more \
         than half its uncertainty {:.2e} ({})",
        m.t,
        (m.t - period).abs(),
        m.u(),
        m.terms()
    );
}

/// The control: the check handed the period off by the claimed uncertainty, each way. Fires when both are rejected.
pub fn control_fires(m: &Measured, period: f64) -> bool {
    let u = m.u();
    !within_half(m, period + u) && !within_half(m, period - u)
}

/// What the closure decides between two printed periods.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Decision {
    /// `u` does not undercut the printed periods' difference: nearer is not read.
    Undecided,
    /// The printed period nearer the measurement (its index), and whether the measurement is within `u` of it.
    Nearer(usize, bool),
}

/// The decision for a measurement `t ± u` between `periods`: [`Decision::Undecided`] unless `u` is smaller than their
/// difference, which is checked before anything else is read.
pub fn decide(t: f64, u: f64, periods: [f64; 2]) -> Decision {
    // A NaN `u` is undecided too.
    if u.partial_cmp(&(periods[0] - periods[1]).abs()) != Some(std::cmp::Ordering::Less) {
        return Decision::Undecided;
    }
    let near = usize::from((t - periods[1]).abs() < (t - periods[0]).abs());
    Decision::Nearer(near, (t - periods[near]).abs() <= u)
}

// ── The target ───────────────────────────────────────────────────────────────────────────────────────────────────

/// BG-4.
pub struct Bg4;

/// The target, as [`super::replication::TARGETS`] registers it.
pub const BG4: Bg4 = Bg4;

/// The known orbits under `root`, each measured.
pub fn known(root: &Path) -> Result<Vec<(Known, Measured)>, String> {
    known_orbits(&sources(root)?)?
        .into_iter()
        .map(|k| {
            let m = measure(&k.start, k.period, k.period, k.box_p)?;
            Ok((k, m))
        })
        .collect()
}

/// `t − T` for each of `periods`, in absolute terms and in `u`.
fn against(lines: &mut Vec<String>, m: &Measured, periods: &[(&str, f64)]) {
    for (which, period) in periods {
        lines.push(format!(
            "  {which} {period:.9}: t − T = {:+.2e} ({:+.2} u)",
            m.t - period,
            (m.t - period) / m.u()
        ));
    }
}

impl Target for Bg4 {
    fn id(&self) -> &'static str {
        ID
    }

    fn score(&self, root: &Path) -> Result<Scored, String> {
        let s = sources(root)?;
        let mut lines = vec![
            "the method on orbits of known period (blocking: it must hold, and its control fire):"
                .to_owned(),
        ];
        let mut control_fired = true;
        for k in known_orbits(&s)? {
            let m = measure(&k.start, k.period, k.period, k.box_p)?;
            if !within_half(&m, k.period) {
                return Err(format!(
                    "{ID}: the method missed the known period of {}: {:.12} against {:.12} ({})",
                    k.start.name,
                    m.t,
                    k.period,
                    m.terms()
                ));
            }
            let fired = control_fires(&m, k.period);
            control_fired &= fired;
            lines.push(format!(
                "{}: t = {:.12}, known {:.12}, off by {:.2e} < u/2 = {:.2e}; T ± u rejected: {fired}",
                k.start.name,
                m.t,
                k.period,
                (m.t - k.period).abs(),
                m.u() / 2.0
            ));
            lines.push(format!("  {}", m.terms()));
        }
        lines
            .push("the gallery's own figure-eight, like for like (reported, never judged):".into());
        for g in &s.gallery_eights {
            let exact = s.figure_eight.period;
            let m = measure(
                &gallery_start(&g.name, g.p),
                g.period.min(exact),
                g.period.max(exact),
                Some(g.p),
            )?;
            lines.push(format!(
                "{} ({}, {}): t = {:.9} ± {:.2e}",
                g.name,
                g.p[0],
                g.p[1],
                m.t,
                m.u()
            ));
            lines.push(format!("  {}", m.terms()));
            against(
                &mut lines,
                &m,
                &[("printed", g.period), ("atlas refined V.1.A", exact)],
            );
        }
        lines.push("the butterflies (a measurement: never blocks):".into());
        let mut open = Vec::new();
        for b in &s.butterflies {
            let start = gallery_start(b.name, b.p);
            let e = invariants(start.g, start.m, start.r, start.v).e;
            let (lo, hi) = (
                b.periods[0].min(b.periods[1]),
                b.periods[0].max(b.periods[1]),
            );
            let m = measure(&start, lo, hi, Some(b.p))?;
            let u = m.u();
            let gap = (b.periods[0] - b.periods[1]).abs();
            lines.push(format!(
                "{} ({}), (p1, p2) = ({}, {}): t = {:.9} ± {u:.2e}; the printed periods' difference {gap:.1e}",
                b.name, b.label, b.p[0], b.p[1], m.t
            ));
            lines.push(format!("  {}", m.terms()));
            lines.push(format!("  {}", m.readings()));
            lines.push(format!(
                "  E recomputed {e:.9} against the gallery's {}",
                b.energy
            ));
            let rescaled = b.atlas.period * (b.atlas.energy.abs() / e.abs()).powf(1.5);
            against(
                &mut lines,
                &m,
                &[
                    ("main page (PRL)", b.periods[0]),
                    ("sequence page", b.periods[1]),
                    ("atlas refined, as printed", b.atlas.period),
                    ("atlas refined, at this E", rescaled),
                ],
            );
            match decide(m.t, u, b.periods) {
                Decision::Nearer(i, true) => lines.push(format!(
                    "  closure decides: {} ({}), within u",
                    b.periods[i],
                    ["main page", "sequence page"][i]
                )),
                Decision::Nearer(i, false) => open.push(format!(
                    "{}: nearer {} but {:.2e} from it, more than u = {u:.2e}",
                    b.name,
                    b.periods[i],
                    (m.t - b.periods[i]).abs()
                )),
                Decision::Undecided => open.push(format!(
                    "{}: u = {u:.2e} does not undercut the printed difference {gap:.1e}",
                    b.name
                )),
            }
        }
        lines.push(format!("credit: {CREDIT_GALLERY}"));
        lines.push(format!("credit: {CREDIT_ATLAS}"));
        Ok(Scored {
            outcome: if open.is_empty() {
                Outcome::Pass
            } else {
                Outcome::Unexplained(open.join("; "))
            },
            control_fired,
            lines,
        })
    }
}
