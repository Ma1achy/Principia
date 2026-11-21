//! Re-registration (pitfalls §4; TASK-M3-32): the measurements of the mechanism, each through the occupant-comparison
//! harness ([`crate::compare`]), and the regularisation-free control.
//!
//! The task names this file `gates/reregistration.rs`; the gates live in `gate/`, so it is here.
//!
//! - **The control** ([`UNREGULARISED`]): KDK under `none`, the leapfrog, an occupant of the pair table that shares no
//!   coordinate machinery with the regularised ones (no chart, no registration, no time transformation). The FTLE–drift
//!   Spearman correlations it was used to measure ([`PRIOR_FINDINGS`]) are prior findings, not controls: no gate here
//!   reads them.
//! - **`heggie-vs-az`** ([`heggie_vs_az`]): Aarseth–Zare against Heggie under both of its time transformations, on the
//!   32-case matrix (`fixtures/case_matrix.toml`), per case and per IC, the default's (Eq. 22) win against Eq. 20 read
//!   from the same runs.
//! - **`reregistration-doubling`** ([`doubling`]): the sync-boundary re-registration count doubled at fixed step, the
//!   drift field's change measured in decades per IC, for the default occupant against Aarseth–Zare; `config_stability`
//!   rendered under the doubled count, its dense damaged pixels counted (the wedges); `--propose` the acceptance level.
//! - **`logh-falsification`** ([`logh_falsification`]): logH against Heggie on the matrix under the same RK4 and step
//!   control, per case, and the verdict.
//!
//! **The runs.** Every arm is the occupant's direct march at f64 ([`crate::compare::run`]), RK4, the harness's test step
//! control ([`StepControl::TEST`]) on the register's sync quantum ([`DELTA_SYNC`]), marched to the case's horizon with
//! no terminal detection, scored against the Precision ring's RK45 ([`crate::compare::reference`]). The matrix's
//! horizons, 13 and 50, are multiples of `Δ_sync` and of `Δ_sync/2`, so a doubled run ends at the same time as its
//! base. The ICs are this repository's decodes of the matrix's charts, not prin-rs's pixels: a case whose chart no
//! decoder here takes is listed as not run, with the reason, and never folded into a tally ([`Form::NotBuilt`]).
//!
//! **Which arm won an IC** ([`judge`]), fixed before any run: where the reference reached the horizon and one arm did
//! and the other did not (non-finite or out of budget), the arm that arrived won, so that an arm cannot draw by
//! failing on the hard ICs; where both arrived, the harness's verdict ([`crate::compare::verdict`]: better by ten,
//! resolved above the reference's uncertainty). **Beside it, reference-free** ([`energy_judge`]): where both arrived,
//! the arm whose energy error (floored at [`DRIFT_FLOOR`]) is ten times smaller won; this scores the ICs the reference
//! cannot pass (on `far` RK45 at `10⁻¹³` reaches the horizon on no IC: two bodies start at rest and fall almost head
//! on). **Which arm won a case**: the one with more IC wins by the reference; where the reference decided no IC, the
//! one with more by energy; equal counts a tie.
//!
//! **The comparisons with prin-rs** (31 of 32, `err>10` 3915 → 74, AZ ×2 at 4.4e-1 decades, logH 3.2 decades behind)
//! are reported beside these numbers, never asserted: they are measurements of another implementation.

use std::path::Path;

use kernel::chart::burrau::{decode as burrau_decode, Burrau};
use kernel::chart::invariant::{Invariant, InvariantLK};
use kernel::decode::{self, Latent};
use kernel::generated::constants::DELTA_SYNC;
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Occupant, Selection, State};
use kernel::regularisation::az::{self, AzCount, AzParams, End, ExplicitTest};
use kernel::regularisation::heggie::{self as hg, HgParams, DEFAULT_TIME};
use kernel::regularisation::pairs::{Choice, HeggieTime, LogHMode, Pair};

use crate::az::Quiet;
use crate::compare::{
    self, admit, admit_logh_heggie, quantiles, Arm, Comparison, Discretisation, RampConstants,
    Refusal, StepControl, Verdict as Judged,
};
use crate::gate::ablation::{self, dense_half, dense_mask, Raster, Slice, DAMAGED};
use crate::gate::{MeasurementConfig, MeasurementControl, MeasurementGate, Verdict};

// ── The control ────────────────────────────────────────────────────────────────────────────────────────────────

/// The regularisation-free control: KDK (the leapfrog) under `none` (pitfalls §4.3: "keep a regularisation-free
/// occupant permanently").
pub const UNREGULARISED: Pair = Pair::select(Selection::Production(Occupant::Kdk), Choice::None);

/// The FTLE–drift Spearman correlations of pitfalls §4.3, as prin-rs measured them: leapfrog drift against FTLE, AZ
/// drift against FTLE, AZ drift against leapfrog drift, and AZ's against its shifted control. Prior findings, recorded
/// as data: nothing here, and no gate, asserts against them (they are correlations, not controls).
pub const PRIOR_FINDINGS: [(&str, f64); 4] = [
    ("leapfrog drift vs FTLE", 0.305),
    ("AZ drift vs FTLE", -0.082),
    ("AZ drift vs leapfrog drift", -0.096),
    ("AZ drift vs FTLE, shifted control", -0.102),
];

// ── The cases ──────────────────────────────────────────────────────────────────────────────────────────────────

/// `gallery_z0` (fixtures/case_matrix.toml): `z_α, z_β, z_q[4], z_μ[2]`, the latent's block order.
pub const GALLERY_Z0: [f64; 8] = [0.35, -0.45, 0.25, -0.15, 0.40, 0.05, 0.20, -0.30];

/// Burrau's masses and positions (fixtures/slices.toml `[burrau]`), at rest.
pub const BURRAU_M: [f64; 3] = [3.0, 4.0, 5.0];
/// Burrau's positions.
pub const BURRAU_R: [[f64; 2]; 3] = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]];

/// How a case's ICs are made.
#[derive(Clone, Debug)]
pub enum Form {
    /// One body's position over the window, the other two at Burrau's, all at rest (prin-rs `BodyPlane`, and
    /// `plane_for_body(0)`, which reproduces it). Burrau's units, not canonicalised, as prin-rs ran it.
    BodyPlane {
        /// The body that moves.
        body: usize,
        /// The window's centre.
        centre: [f64; 2],
        /// Its half-width.
        half: f64,
    },
    /// A latent plane `z = z₀ + x·q₁ + y·q₂` over the window, decoded by this repository's decoder.
    Latent {
        /// `z₀`.
        z0: [f64; 8],
        /// `q₁`.
        q1: [f64; 8],
        /// `q₂`.
        q2: [f64; 8],
        /// The window's centre.
        centre: [f64; 2],
        /// Its half-width.
        half: f64,
    },
    /// A saved latent slice (`config_stability`), over the chart's unit square.
    Slice(Slice),
    /// The `(ν, K)` Burrau family over the window, this repository's chart.
    BurrauNuK(Burrau<f64>, [f64; 2], f64),
    /// The `(L_z, K)` invariant chart over the window, this repository's chart on the same frozen controls.
    Invariant(InvariantLK<f64>, [f64; 2], f64),
    /// A chart no decoder here takes; the reason.
    NotBuilt(String),
}

/// A case of the matrix.
#[derive(Clone, Debug)]
pub struct Case {
    /// Its name.
    pub name: String,
    /// Its horizon.
    pub horizon: f64,
    /// How its ICs are made.
    pub form: Form,
}

/// The value of `key` in a block of `key = value` lines, comments stripped.
fn value<'a>(block: &'a str, key: &str) -> Option<&'a str> {
    block.lines().find_map(|l| {
        let l = l.split('#').next().unwrap_or("");
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim())
    })
}

fn string(block: &str, key: &str) -> Option<String> {
    value(block, key).map(|v| v.trim_matches('"').to_string())
}

fn number(block: &str, key: &str) -> Result<Option<f64>, String> {
    value(block, key)
        .map(|v| v.parse::<f64>().map_err(|e| format!("`{key} = {v}`: {e}")))
        .transpose()
}

fn array(block: &str, key: &str) -> Result<Option<Vec<f64>>, String> {
    value(block, key)
        .map(|v| {
            v.trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|x| {
                    x.trim()
                        .parse::<f64>()
                        .map_err(|e| format!("`{key}`: {x:?}: {e}"))
                })
                .collect()
        })
        .transpose()
}

/// The block of the section `[name]` in `text`.
fn section<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let header = format!("[{name}]");
    let start = text.find(&format!("\n{header}\n"))? + header.len() + 2;
    let rest = &text[start..];
    let end = rest.find("\n[").map_or(rest.len(), |e| e + 1);
    Some(&rest[..end])
}

fn pair(v: Vec<f64>, what: &str) -> Result<[f64; 2], String> {
    <[f64; 2]>::try_from(v).map_err(|v| format!("{what}: {v:?} is not two numbers"))
}

fn eight(v: Vec<f64>, what: &str) -> Result<[f64; 8], String> {
    <[f64; 8]>::try_from(v).map_err(|v| format!("{what}: {v:?} is not eight numbers"))
}

/// Gram–Schmidt on the seed pair (prin-rs `grid.rs:311`).
fn oblique(a: [f64; 8], b: [f64; 8]) -> ([f64; 8], [f64; 8]) {
    let dot = |x: &[f64; 8], y: &[f64; 8]| (0..8).map(|k| x[k] * y[k]).sum::<f64>();
    let n1 = dot(&a, &a).sqrt();
    let q1 = a.map(|x| x / n1);
    let p = dot(&b, &q1);
    let mut q2 = [0.0; 8];
    for k in 0..8 {
        q2[k] = b[k] - p * q1[k];
    }
    let n2 = dot(&q2, &q2).sqrt();
    (q1, q2.map(|x| x / n2))
}

/// The case of one `[[case]]` block; `slices` is `fixtures/slices.toml`.
fn case(block: &str, slices: &str) -> Result<Case, String> {
    let name = string(block, "name").ok_or("a case with no name")?;
    let err = |e: String| format!("case {name}: {e}");
    let horizon = number(block, "t_max").map_err(err)?.unwrap_or(13.0);
    let window = |b: &str| -> Result<([f64; 2], f64), String> {
        let centre = array(b, "centre")?.ok_or("no centre")?;
        let half = number(b, "half")?.ok_or("no half")?;
        Ok((pair(centre, "centre")?, half))
    };
    let form = if let Some(slice) = string(block, "slice") {
        if slice == "config_stability" {
            Form::Slice(ablation::latent_slice(slices, "config_stability")?)
        } else {
            let b = section(slices, &slice).ok_or_else(|| err(format!("no [{slice}]")))?;
            let (centre, half) = window(b).map_err(err)?;
            match string(b, "chart").as_deref() {
                Some("body_plane") => {
                    let body = number(b, "body").map_err(err)?.unwrap_or(0.0) as usize;
                    Form::BodyPlane {
                        body: body.min(2),
                        centre,
                        half,
                    }
                }
                other => Form::NotBuilt(format!("the region's chart {other:?}")),
            }
        }
    } else {
        let chart = string(block, "chart").ok_or_else(|| err("no chart".into()))?;
        let (centre, half) = window(block).map_err(err)?;
        let axes = chart
            .strip_prefix("latent_axes(gallery_z0, ")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.split_once(", "))
            .and_then(|(i, j)| Some((i.parse::<usize>().ok()?, j.parse::<usize>().ok()?)));
        match chart.as_str() {
            "body_plane" | "plane_for_body(0)" => Form::BodyPlane {
                body: 0,
                centre,
                half,
            },
            "latent_oblique(gallery_z0, a, b)" => {
                let a = eight(array(block, "a").map_err(err)?.ok_or("no a")?, "a")?;
                let b = eight(array(block, "b").map_err(err)?.ok_or("no b")?, "b")?;
                let (q1, q2) = oblique(a, b);
                Form::Latent {
                    z0: GALLERY_Z0,
                    q1,
                    q2,
                    centre,
                    half,
                }
            }
            "preset_shape" | "preset_prho" | "preset_plambda" | "preset_shape_pl" => {
                // prin-rs `grid.rs:352-400`: latent planes through `z₀ = 0`; `shape_pl`'s basis un-normalised,
                // `q₁ = e_α + e_{pΛy}`, `q₂ = e_β + e_{pΛx}` in the spec's order.
                let (mut q1, mut q2) = ([0.0; 8], [0.0; 8]);
                let (i, j) = match chart.as_str() {
                    "preset_shape" => (0, 1),
                    "preset_prho" => (2, 3),
                    "preset_plambda" => (4, 5),
                    _ => {
                        q1[5] = 1.0;
                        q2[4] = 1.0;
                        (0, 1)
                    }
                };
                q1[i] = 1.0;
                q2[j] = 1.0;
                Form::Latent {
                    z0: [0.0; 8],
                    q1,
                    q2,
                    centre,
                    half,
                }
            }
            "invariant" => {
                let k_max = number(block, "k_max").map_err(err)?.ok_or("no k_max")?;
                let gamma_k = number(block, "gamma_k").map_err(err)?.ok_or("no gamma_k")?;
                Form::Invariant(
                    InvariantLK(Invariant {
                        z_cfg: [GALLERY_Z0[0], GALLERY_Z0[1]],
                        z_mass: [GALLERY_Z0[6], GALLERY_Z0[7]],
                        k_max,
                        gamma_k,
                    }),
                    centre,
                    half,
                )
            }
            "mass_simplex" => Form::NotBuilt(
                "prin-rs's mass simplex decodes the configuration from (z_alpha, z_beta) and the momenta from z_q; \
                 this repository's (`kernel::chart::mass_simplex`) fixes the Burrau triangle at rest"
                    .into(),
            ),
            "shape_at_burrau(0.0)" => Form::NotBuilt(
                "prin-rs's shape chart is the tangent plane at Burrau's own triangle, masses [3, 4, 5], at rest; \
                 this repository's shape sphere (`kernel::chart::shape_sphere`) is a latent map with no such window"
                    .into(),
            ),
            "burrau_family" => {
                let lo = number(block, "nu_lo").map_err(err)?.ok_or("no nu_lo")?;
                let hi = number(block, "nu_hi").map_err(err)?.ok_or("no nu_hi")?;
                let k_max = number(block, "k_max").map_err(err)?.ok_or("no k_max")?;
                let gamma_k = number(block, "gamma_k").map_err(err)?.ok_or("no gamma_k")?;
                Form::BurrauNuK(
                    Burrau {
                        nu_range: [lo, hi],
                        k_max,
                        gamma_k,
                    },
                    centre,
                    half,
                )
            }
            _ => match axes {
                Some((i, j)) if i < 8 && j < 8 && i != j => {
                    let (mut q1, mut q2) = ([0.0; 8], [0.0; 8]);
                    q1[i] = 1.0;
                    q2[j] = 1.0;
                    Form::Latent {
                        z0: GALLERY_Z0,
                        q1,
                        q2,
                        centre,
                        half,
                    }
                }
                // The plan is unclear here (the matrix names prin-rs's charts; R-166 transcribes them, but the shape
                // sphere at Burrau, the invariant chart, the mass simplex and the presets are prin-rs constructions this
                // repository's charts do not reproduce point for point); chose to list each such case as not run,
                // because a substituted chart would be a different case under its name.
                _ => Form::NotBuilt(format!(
                    "prin-rs's chart `{chart}`, which no chart here reproduces"
                )),
            },
        }
    };
    Ok(Case {
        name,
        horizon,
        form,
    })
}

/// The matrix's cases, in its order, from the workspace at `root`.
pub fn cases(root: &Path) -> Result<Vec<Case>, String> {
    let read = |p: &str| {
        let path = root.join(p);
        std::fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    };
    let (matrix, slices) = (
        read("fixtures/case_matrix.toml")?,
        read("fixtures/slices.toml")?,
    );
    matrix
        .split("[[case]]")
        .skip(1)
        .map(|block| {
            let block = block.split("\n[").next().unwrap_or(block);
            case(block, &slices)
        })
        .collect()
}

/// The case named `name`.
pub fn case_named(root: &Path, name: &str) -> Result<Case, String> {
    cases(root)?
        .into_iter()
        .find(|c| c.name == name)
        .ok_or_else(|| format!("the matrix has no case `{name}`"))
}

/// A state whose every number is finite.
fn finite(s: &State<f64>) -> bool {
    s.m.iter()
        .chain(s.r.iter().flatten())
        .chain(s.p.iter().flatten())
        .all(|x| x.is_finite())
}

/// `s` in its centre-of-mass frame: the centre of mass moved to the origin and at rest. Heggie's and Aarseth–Zare's
/// marches take and return the centre-of-mass frame (`kernel::regularisation::heggie::march`), and logH's and the
/// reference's keep the frame they are given, so an IC off it scores the frame, not the integration (measured: on
/// `near_field`'s body-plane ICs, uncentred, Heggie and AZ erred identically by the centre of mass's offset, `4.8e-4` to
/// `1.4e-3`, where logH erred by `1.5e-8`).
pub fn centred(s: &State<f64>) -> State<f64> {
    let m: f64 = s.m.iter().sum();
    let c = |f: &dyn Fn(usize) -> f64| (0..3).map(f).sum::<f64>() / m;
    let rc = [c(&|i| s.m[i] * s.r[i][0]), c(&|i| s.m[i] * s.r[i][1])];
    let vc = [c(&|i| s.p[i][0]), c(&|i| s.p[i][1])];
    State {
        m: s.m,
        r: s.r.map(|r| [r[0] - rc[0], r[1] - rc[1]]),
        p: std::array::from_fn(|i| [s.p[i][0] - s.m[i] * vc[0], s.p[i][1] - s.m[i] * vc[1]]),
    }
}

/// The ICs of `raster` of `case`, each in its centre-of-mass frame ([`centred`]): pixel `i` at the cell centre `(u, v)` of the unit square, mapped onto the window
/// `centre ± half`. `None` where the decode is degenerate or not finite. Empty for a case not built.
pub fn ics(case: &Case, raster: Raster) -> Vec<Option<State<f64>>> {
    let at = |centre: [f64; 2], half: f64, (u, v): (f64, f64)| {
        (
            centre[0] - half + 2.0 * half * u,
            centre[1] - half + 2.0 * half * v,
        )
    };
    let one = |uv: (f64, f64)| -> Option<State<f64>> {
        let s = match &case.form {
            Form::BodyPlane { body, centre, half } => {
                let (x, y) = at(*centre, *half, uv);
                let mut r = BURRAU_R;
                r[*body] = [x, y];
                State {
                    m: BURRAU_M,
                    r,
                    p: [[0.0; 2]; 3],
                }
            }
            Form::Latent {
                z0,
                q1,
                q2,
                centre,
                half,
            } => {
                let (x, y) = at(*centre, *half, uv);
                let z: [f64; 8] = std::array::from_fn(|k| z0[k] + x * q1[k] + y * q2[k]);
                let d = decode::decode::<Defaults, f64>(&Latent::new(z));
                if d.degenerate.is_some() {
                    return None;
                }
                State {
                    m: d.ic.m,
                    r: d.ic.r,
                    p: d.ic.p,
                }
            }
            Form::Slice(slice) => {
                use kernel::chart::Chart as _;
                let d = slice.chart.decode::<Defaults, f64>(uv.0, uv.1);
                if d.degenerate.is_some() {
                    return None;
                }
                State {
                    m: d.ic.m,
                    r: d.ic.r,
                    p: d.ic.p,
                }
            }
            Form::BurrauNuK(chart, centre, half) => {
                let (x, y) = at(*centre, *half, uv);
                let (d, _) = burrau_decode::<f64, f64>(chart, x, y);
                if d.degenerate.is_some() {
                    return None;
                }
                State {
                    m: d.ic.m,
                    r: d.ic.r,
                    p: d.ic.p,
                }
            }
            Form::Invariant(chart, centre, half) => {
                let (x, y) = at(*centre, *half, uv);
                let (d, _) = kernel::chart::invariant::decode::<Defaults, f64, _>(chart, x, y);
                if d.degenerate.is_some() {
                    return None;
                }
                State {
                    m: d.ic.m,
                    r: d.ic.r,
                    p: d.ic.p,
                }
            }
            Form::NotBuilt(_) => return None,
        };
        finite(&s).then(|| centred(&s))
    };
    if matches!(case.form, Form::NotBuilt(_)) {
        return Vec::new();
    }
    (0..raster.len()).map(|i| one(raster.uv(i))).collect()
}

// ── The arms ───────────────────────────────────────────────────────────────────────────────────────────────────

/// RK4 under `regularisation` with the test step control at `eta`, on the sync quantum `delta_sync`, to `horizon`.
pub fn arm(regularisation: Choice, eta: f64, delta_sync: f64, horizon: f64) -> Arm {
    Arm {
        stepper: Selection::Production(Occupant::Rk4),
        regularisation,
        step_control: StepControl {
            eta,
            ..StepControl::TEST
        },
        discretisation: Discretisation {
            delta_sync,
            horizon,
        },
    }
}

/// The step factor every base arm runs: the test step control's.
pub const BASE_ETA: f64 = StepControl::TEST.eta;

/// Aarseth–Zare.
pub const AZ: Choice = Choice::AarsethZare;
/// The default occupant's regularisation: Heggie under Eq. 22 at n = 3/2.
pub const HEGGIE: Choice = Choice::Heggie(DEFAULT_TIME);
/// Heggie under Eq. 20.
pub const HEGGIE_EQ20: Choice = Choice::Heggie(HeggieTime::Eq20);
/// logH in its logH mode.
pub const LOGH: Choice = Choice::LogH(LogHMode::LogH);

/// The number of worker threads.
pub fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

/// [`compare::compare`] over `ics` split across `threads` threads, the pieces joined in order: the same comparison,
/// IC for IC, as one call (each IC's runs and reference depend on that IC alone).
pub fn compare_par(
    arms: &[Arm],
    ics: &[State<f64>],
    threads: usize,
) -> Result<Comparison, Refusal> {
    admit(arms)?;
    let ramp = RampConstants::POSITION_ERROR;
    let threads = threads.clamp(1, ics.len().max(1));
    let chunk = ics.len().div_ceil(threads).max(1);
    let pieces: Vec<Result<Comparison, Refusal>> = std::thread::scope(|s| {
        let handles: Vec<_> = ics
            .chunks(chunk)
            .map(|piece| s.spawn(move || compare::compare(arms, piece, ramp)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a comparison worker panicked"))
            .collect()
    });
    let mut out = compare::compare(arms, &[], ramp)?;
    for piece in pieces {
        let piece = piece?;
        for a in 0..arms.len() {
            out.runs[a].extend_from_slice(&piece.runs[a]);
            out.errors[a].extend_from_slice(&piece.errors[a]);
        }
        out.reference.extend_from_slice(&piece.reference);
    }
    out.panels = compare::panels(&out.errors, ramp);
    Ok(out)
}

// ── Who won ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Which of arms `a`, `b` of `c` won IC `i` (the module's note): an arm that arrived where the other did not won;
/// both arrived, the harness's verdict.
pub fn judge(c: &Comparison, a: usize, b: usize, i: usize) -> Judged {
    if c.reference[i].state.is_none() {
        return Judged::NotJudged;
    }
    let (ra, rb) = (
        c.runs[a][i].reached(c.boundary),
        c.runs[b][i].reached(c.boundary),
    );
    match (ra, rb) {
        (true, false) => Judged::A,
        (false, true) => Judged::B,
        (false, false) => Judged::NotJudged,
        (true, true) => {
            compare::verdict(c.errors[a][i], c.errors[b][i], c.reference[i].uncertainty)
        }
    }
}

/// The tally of [`judge`] over every IC: `[a, b, tie, unresolved, not judged]`.
pub fn tally(c: &Comparison, a: usize, b: usize) -> [usize; 5] {
    let mut t = [0; 5];
    for i in 0..c.reference.len() {
        t[match judge(c, a, b, i) {
            Judged::A => 0,
            Judged::B => 1,
            Judged::Tie => 2,
            Judged::Unresolved => 3,
            Judged::NotJudged => 4,
        }] += 1;
    }
    t
}

/// Which of arms `a`, `b` of `c` won IC `i` by energy (the module's note): as [`judge`] where one arm did not arrive;
/// both arrived, the one whose energy error is ten times smaller; `Tie` otherwise. `ics` are the comparison's ICs.
pub fn energy_judge(c: &Comparison, ics: &[State<f64>], a: usize, b: usize, i: usize) -> Judged {
    let (ra, rb) = (
        c.runs[a][i].reached(c.boundary),
        c.runs[b][i].reached(c.boundary),
    );
    match (ra, rb) {
        (true, false) => Judged::A,
        (false, true) => Judged::B,
        (false, false) => Judged::NotJudged,
        (true, true) => {
            let e = |x: usize| energy_error(&ics[i], &c.runs[x][i].state).max(DRIFT_FLOOR);
            let (ea, eb) = (e(a), e(b));
            if !(ea.is_finite() && eb.is_finite()) {
                Judged::NotJudged
            } else if eb >= compare::BETTER_BY * ea {
                Judged::A
            } else if ea >= compare::BETTER_BY * eb {
                Judged::B
            } else {
                Judged::Tie
            }
        }
    }
}

/// The tally of [`energy_judge`]: `[a, b, tie, unresolved (never), not judged]`.
pub fn energy_tally(c: &Comparison, ics: &[State<f64>], a: usize, b: usize) -> [usize; 5] {
    let mut t = [0; 5];
    for i in 0..ics.len() {
        t[match energy_judge(c, ics, a, b, i) {
            Judged::A => 0,
            Judged::B => 1,
            Judged::Tie => 2,
            Judged::Unresolved => 3,
            Judged::NotJudged => 4,
        }] += 1;
    }
    t
}

/// Who won a case from its reference tally `by_reference` and its energy tally `by_energy` (the module's note).
pub fn case_winner_of(by_reference: [usize; 5], by_energy: [usize; 5]) -> CaseWinner {
    if by_reference[0] + by_reference[1] > 0 {
        case_winner(by_reference)
    } else {
        case_winner(by_energy)
    }
}

/// Who won a case, from its tally (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseWinner {
    /// Arm `a`.
    A,
    /// Arm `b`.
    B,
    /// Neither: equal wins.
    Tie,
}

/// The case's winner from `[a, b, …]`.
pub fn case_winner(t: [usize; 5]) -> CaseWinner {
    match t[0].cmp(&t[1]) {
        std::cmp::Ordering::Greater => CaseWinner::A,
        std::cmp::Ordering::Less => CaseWinner::B,
        std::cmp::Ordering::Equal => CaseWinner::Tie,
    }
}

/// `|E − E₀| / (K₀ + |U₀|)` from `s0` to `s`, at f64 (the step-limit ablation's energy error).
pub fn energy_error(s0: &State<f64>, s: &State<f64>) -> f64 {
    let ku = |s: &State<f64>| {
        let k: f64 = (0..3)
            .map(|i| (s.p[i][0].powi(2) + s.p[i][1].powi(2)) / (2.0 * s.m[i]))
            .sum();
        let u: f64 = [(0, 1), (0, 2), (1, 2)]
            .iter()
            .map(|&(i, j)| -s.m[i] * s.m[j] / (s.r[i][0] - s.r[j][0]).hypot(s.r[i][1] - s.r[j][1]))
            .sum();
        (k, u)
    };
    let (k0, u0) = ku(s0);
    let (k, u) = ku(s);
    ((k + u) - (k0 + u0)).abs() / (k0 + u0.abs())
}

/// The median of the finite values of `x`; `NaN` where none is.
pub fn median(x: impl IntoIterator<Item = f64>) -> f64 {
    let mut v: Vec<f64> = x.into_iter().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn steps_p50(c: &Comparison, a: usize) -> f64 {
    median(c.runs[a].iter().map(|t| f64::from(t.steps)))
}

fn energy_p50(c: &Comparison, ics: &[State<f64>], a: usize) -> f64 {
    median(
        c.runs[a]
            .iter()
            .zip(ics)
            .filter(|(t, _)| t.reached(c.boundary))
            .map(|(t, s0)| energy_error(s0, &t.state)),
    )
}

/// `log₁₀(e_b/e_a)` p50 over the ICs both arms reached with finite, positive errors.
fn log_ratio_p50(c: &Comparison, a: usize, b: usize) -> f64 {
    median((0..c.reference.len()).map(|i| (c.errors[b][i] / c.errors[a][i]).log10()))
}

// ── heggie-vs-az ───────────────────────────────────────────────────────────────────────────────────────────────

/// One case's Heggie-vs-AZ row.
#[derive(Clone, Debug)]
pub struct HvaRow {
    /// The case.
    pub case: String,
    /// The ICs marched (decoded and finite).
    pub ics: usize,
    /// The pixels not decoded.
    pub undecoded: usize,
    /// The default (Eq. 22) against AZ: `[Heggie, AZ, tie, unresolved, not judged]`.
    pub hg_az: [usize; 5],
    /// The default against AZ by energy, same layout.
    pub hg_az_energy: [usize; 5],
    /// Eq. 22 against Eq. 20, same layout.
    pub eq22_eq20: [usize; 5],
    /// `log₁₀(e_AZ/e_Heggie)` p50.
    pub log_ratio: f64,
    /// Steps p50: AZ, Eq. 20, Eq. 22.
    pub steps: [f64; 3],
    /// Energy error p50: AZ, Eq. 20, Eq. 22.
    pub energy: [f64; 3],
}

impl HvaRow {
    /// Who won the case: A is Heggie.
    pub fn winner(&self) -> CaseWinner {
        case_winner_of(self.hg_az, self.hg_az_energy)
    }
}

/// The arms of `heggie-vs-az` at `horizon`: `[AZ, Heggie Eq. 20, Heggie Eq. 22]`.
pub fn hva_arms(horizon: f64) -> [Arm; 3] {
    [AZ, HEGGIE_EQ20, HEGGIE].map(|r| arm(r, BASE_ETA, DELTA_SYNC, horizon))
}

/// The Heggie-vs-AZ row of `case` on `raster`; `None` for a case not built.
pub fn hva_row(case: &Case, raster: Raster, threads: usize) -> Result<Option<HvaRow>, Refusal> {
    if matches!(case.form, Form::NotBuilt(_)) {
        return Ok(None);
    }
    let all = ics(case, raster);
    let ics: Vec<State<f64>> = all.iter().flatten().copied().collect();
    let c = compare_par(&hva_arms(case.horizon), &ics, threads)?;
    Ok(Some(HvaRow {
        case: case.name.clone(),
        ics: ics.len(),
        undecoded: all.len() - ics.len(),
        hg_az: tally(&c, 2, 0),
        hg_az_energy: energy_tally(&c, &ics, 2, 0),
        eq22_eq20: tally(&c, 2, 1),
        log_ratio: log_ratio_p50(&c, 2, 0),
        steps: [0, 1, 2].map(|a| steps_p50(&c, a)),
        energy: [0, 1, 2].map(|a| energy_p50(&c, &ics, a)),
    }))
}

/// The `heggie-vs-az` report on every case at `n × n`.
pub fn heggie_vs_az(root: &Path, n: usize, threads: usize) -> Result<String, String> {
    let mut out = format!(
        "# Heggie vs Aarseth-Zare on the 32-case matrix, {n}x{n} per case\n\nRK4, eta {BASE_ETA}, delta_sync \
         {DELTA_SYNC}, f64, no terminal detection; scored against RK45 at 1e-13 (uncertainty from a 1e-12 run).\nPer \
         IC: an arm that reached the horizon where the other did not won; both reached, better by 10 above the \
         reference's uncertainty. Per case: more IC wins.\n\n| case | ICs | undecoded | Heggie | AZ | tie | \
         unresolved | not judged | log10(e_AZ/e_HG) p50 | by energy HG/AZ/tie | winner | Eq22 vs Eq20 (wins, wins) | \
         steps p50 AZ/Eq20/Eq22 | energy err p50 AZ/Eq20/Eq22 |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n"
    );
    let (mut hg, mut az, mut tie, mut built) = (0, 0, 0, 0);
    let (mut w22, mut w20) = (0, 0);
    let mut not_built = Vec::new();
    for case in cases(root)? {
        let started = std::time::Instant::now();
        match hva_row(&case, Raster::full(n), threads).map_err(|r| format!("{r:?}"))? {
            None => {
                if let Form::NotBuilt(why) = &case.form {
                    not_built.push(format!("- {}: not run: {why}\n", case.name));
                }
            }
            Some(r) => {
                built += 1;
                let w = r.winner();
                match w {
                    CaseWinner::A => hg += 1,
                    CaseWinner::B => az += 1,
                    CaseWinner::Tie => tie += 1,
                }
                w22 += r.eq22_eq20[0];
                w20 += r.eq22_eq20[1];
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} | {} | {} | {:.2} | {}/{}/{} | {} | {}, {} | {:.0}/{:.0}/{:.0} | \
                     {:.1e}/{:.1e}/{:.1e} |\n",
                    r.case,
                    r.ics,
                    r.undecoded,
                    r.hg_az[0],
                    r.hg_az[1],
                    r.hg_az[2],
                    r.hg_az[3],
                    r.hg_az[4],
                    r.log_ratio,
                    r.hg_az_energy[0],
                    r.hg_az_energy[1],
                    r.hg_az_energy[2],
                    match w {
                        CaseWinner::A => "Heggie",
                        CaseWinner::B => "AZ",
                        CaseWinner::Tie => "tie",
                    },
                    r.eq22_eq20[0],
                    r.eq22_eq20[1],
                    r.steps[0],
                    r.steps[1],
                    r.steps[2],
                    r.energy[0],
                    r.energy[1],
                    r.energy[2],
                ));
                eprintln!(
                    "heggie-vs-az: {} in {:.1} s",
                    r.case,
                    started.elapsed().as_secs_f64()
                );
            }
        }
    }
    out.push_str(&format!(
        "\nCases run: {built}. Heggie won {hg}, AZ won {az}, tie {tie}.\nEq. 22 against Eq. 20, IC wins summed over \
         the cases run: {w22} to {w20} ({}).\nThe default occupant (`kernel::regularisation::pairs::Pair::DEFAULT`): \
         {:?}.\n\nprin-rs, for comparison only (never asserted): Heggie won 31 of 32, AZ only on `far` (every pixel); \
         err>10 3915 -> 74 at 8600d45 (3916 -> 73 at 70cfbc4). err>10 is an ensemble statistic this repository's \
         harness does not compute; the per-IC position error against the reference stands in for it.\n\n",
        if w22 >= w20 {
            "the default's transformation wins or draws"
        } else {
            "the default's transformation loses"
        },
        Pair::DEFAULT,
    ));
    for line in not_built {
        out.push_str(&line);
    }
    Ok(out)
}

// ── logh-falsification ─────────────────────────────────────────────────────────────────────────────────────────

/// One case's logH-vs-Heggie row.
#[derive(Clone, Debug)]
pub struct LogHRow {
    /// The case.
    pub case: String,
    /// The ICs marched.
    pub ics: usize,
    /// `[Heggie, logH, tie, unresolved, not judged]`.
    pub tally: [usize; 5],
    /// The same by energy.
    pub energy: [usize; 5],
    /// `log₁₀(e_logH/e_Heggie)` p50: positive where Heggie did better.
    pub log_ratio: f64,
    /// Steps p50: Heggie, logH.
    pub steps: [f64; 2],
    /// Energy error p50: Heggie, logH.
    pub energy_p50: [f64; 2],
    /// Not reached (non-finite or out of budget): Heggie, logH.
    pub dropped: [usize; 2],
}

impl LogHRow {
    /// Whether logH matches or beats Heggie on the case, fixed before any run: Heggie won no more ICs than logH did,
    /// by the reference where it decided any IC, by energy otherwise.
    pub fn logh_matches_or_beats(&self) -> bool {
        case_winner_of(self.tally, self.energy) != CaseWinner::A
    }

    /// Whether either score decided any IC.
    pub fn decided(&self) -> bool {
        self.tally[0] + self.tally[1] + self.energy[0] + self.energy[1] > 0
    }
}

/// The arms of `logh-falsification` at `horizon`, admitted by the logH-vs-Heggie harness: `[Heggie, logH]`.
pub fn logh_arms(horizon: f64) -> Result<[Arm; 2], Refusal> {
    admit_logh_heggie([HEGGIE, LOGH].map(|r| arm(r, BASE_ETA, DELTA_SYNC, horizon)))
}

/// The logH-vs-Heggie row of `case` on `raster`; `None` for a case not built.
pub fn logh_row(case: &Case, raster: Raster, threads: usize) -> Result<Option<LogHRow>, Refusal> {
    if matches!(case.form, Form::NotBuilt(_)) {
        return Ok(None);
    }
    let ics: Vec<State<f64>> = ics(case, raster).into_iter().flatten().collect();
    let c = compare_par(&logh_arms(case.horizon)?, &ics, threads)?;
    Ok(Some(LogHRow {
        case: case.name.clone(),
        ics: ics.len(),
        tally: tally(&c, 0, 1),
        energy: energy_tally(&c, &ics, 0, 1),
        log_ratio: log_ratio_p50(&c, 0, 1),
        steps: [0, 1].map(|a| steps_p50(&c, a)),
        energy_p50: [0, 1].map(|a| energy_p50(&c, &ics, a)),
        dropped: [0, 1].map(|a| c.runs[a].iter().filter(|t| !t.reached(c.boundary)).count()),
    }))
}

/// The verdict over the rows, fixed before any run (pitfalls §4.4: "if the mechanism is real, logH should match or
/// beat Heggie; if it does not, the mechanism is wrong"): supported where logH matches or beats Heggie on every case
/// run in which at least one IC was decided (won by either arm, by either score); otherwise not supported, naming the
/// cases.
pub fn logh_verdict(rows: &[LogHRow]) -> (bool, Vec<String>) {
    let lost: Vec<String> = rows
        .iter()
        .filter(|r| r.decided() && !r.logh_matches_or_beats())
        .map(|r| r.case.clone())
        .collect();
    (lost.is_empty(), lost)
}

/// The `logh-falsification` report on every case at `n × n`.
pub fn logh_falsification(root: &Path, n: usize, threads: usize) -> Result<String, String> {
    let mut out = format!(
        "# logH vs Heggie on the 32-case matrix, {n}x{n} per case\n\nBoth RK4, one step control (eta {BASE_ETA}, \
         delta_sync {DELTA_SYNC}, f = 0.02, the landing), f64, no terminal detection; admitted by the logH-vs-Heggie \
         harness. Scored as heggie-vs-az is.\n\n| case | ICs | Heggie | logH | tie | unresolved | not judged | \
         log10(e_logH/e_HG) p50 | by energy HG/logH/tie | dropped HG/logH | steps p50 HG/logH | energy err p50 \
         HG/logH | logH matches or beats |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n"
    );
    let mut rows = Vec::new();
    for case in cases(root)? {
        let started = std::time::Instant::now();
        if let Some(r) = logh_row(&case, Raster::full(n), threads).map_err(|r| format!("{r:?}"))? {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {:.2} | {}/{}/{} | {}/{} | {:.0}/{:.0} | {:.1e}/{:.1e} | {} |\n",
                r.case,
                r.ics,
                r.tally[0],
                r.tally[1],
                r.tally[2],
                r.tally[3],
                r.tally[4],
                r.log_ratio,
                r.energy[0],
                r.energy[1],
                r.energy[2],
                r.dropped[0],
                r.dropped[1],
                r.steps[0],
                r.steps[1],
                r.energy_p50[0],
                r.energy_p50[1],
                if r.logh_matches_or_beats() { "yes" } else { "no" },
            ));
            eprintln!(
                "logh-falsification: {} in {:.1} s",
                r.case,
                started.elapsed().as_secs_f64()
            );
            rows.push(r);
        }
    }
    let (supported, lost) = logh_verdict(&rows);
    out.push_str(&format!(
        "\nVerdict: {}.\n\nprin-rs, for comparison only: logh_rk4 3.2 decades behind Heggie, logh_lf 5.3, on a \
         576-pixel grid (NOTES excerpts).\n",
        if supported {
            "logH matches or beats Heggie on every decided case: the mechanism is supported".to_string()
        } else {
            format!(
                "logH loses to Heggie on {} of {} cases run ({}): by the falsification's own terms the mechanism, as \
                 stated, is not supported",
                lost.len(),
                rows.len(),
                lost.join(", ")
            )
        }
    ));
    Ok(out)
}

// ── reregistration-doubling ────────────────────────────────────────────────────────────────────────────────────

/// The steps p50's allowed move under the doubling: within 6% (pitfalls §4.1).
pub const FLAT: f64 = 0.06;

/// The step jitter's factor on `η`: the same grid, every step a little longer, steps p50 inside [`FLAT`]. Its drift
/// change is how far the field moves when only the step sequence is perturbed, with no boundary added; reported beside
/// the doubling, never part of the acceptance rule.
pub const JITTER: f64 = 1.03;

/// The most step-factor adjustments [`calibrate_eta`] makes.
pub const ETA_TRIES: usize = 8;

/// The floor under a drift before its logarithm: about the f64 rounding of a unit-scale energy, so that two
/// round-off drifts do not read as decades apart.
pub const DRIFT_FLOOR: f64 = 1e-15;

/// One run set of an occupant on a case: the ICs' trajectories under one arm.
fn runs(a: &Arm, ics: &[State<f64>], threads: usize) -> Vec<compare::Trajectory> {
    let threads = threads.clamp(1, ics.len().max(1));
    let chunk = ics.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = ics
            .chunks(chunk)
            .map(|piece| {
                s.spawn(move || {
                    piece
                        .iter()
                        .map(|ic| compare::run(a, ic))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a doubling worker panicked"))
            .collect()
    })
}

/// The step factor `η′` at which `regularisation` on the sync quantum `delta_sync/factor` takes a steps p50 within
/// [`FLAT`] of `base_p50`: from `factor·η` (the step's size is `η·Δ_sync` over the regularised rate, so halving
/// `Δ_sync` halves it), corrected by the measured ratio, at most [`ETA_TRIES`] times. `None` where it never lands.
pub fn calibrate_eta(
    regularisation: Choice,
    factor: u32,
    horizon: f64,
    ics: &[State<f64>],
    base_p50: f64,
    threads: usize,
) -> Option<(f64, f64)> {
    let mut eta = BASE_ETA * f64::from(factor);
    for _ in 0..ETA_TRIES {
        let a = arm(regularisation, eta, DELTA_SYNC / f64::from(factor), horizon);
        let p50 = median(runs(&a, ics, threads).iter().map(|t| f64::from(t.steps)));
        if (p50 / base_p50 - 1.0).abs() <= FLAT / 2.0 {
            return Some((eta, p50));
        }
        eta *= p50 / base_p50;
    }
    None
}

/// An occupant's doubling on a case's ICs.
#[derive(Clone, Debug)]
pub struct Doubling {
    /// The regularisation.
    pub regularisation: Choice,
    /// The re-registration factor (2 for the protocol; 1 for the null).
    pub factor: u32,
    /// The base arm.
    pub base: Arm,
    /// The doubled arm.
    pub doubled: Arm,
    /// Steps p50, base and doubled.
    pub steps: [f64; 2],
    /// Re-registrations at the horizon p50, base and doubled.
    pub re_registrations: [f64; 2],
    /// Whether every IC both arms reached registered exactly twice as often doubled as base (the registration at
    /// `t = 0` counted), and re-registered at all: the protocol's doubling, measured, not assumed from `factor`.
    pub doubled_exactly: bool,
    /// Per IC, `|log₁₀ d₂ − log₁₀ d₁|`, each drift floored at [`DRIFT_FLOOR`]; `NaN` where either arm did not reach.
    pub change: Vec<f64>,
    /// The drift field's median `log₁₀ d`, base and doubled.
    pub median_log_drift: [f64; 2],
}

impl Doubling {
    /// The change's p50 in decades.
    pub fn change_p50(&self) -> f64 {
        median(self.change.iter().copied())
    }

    /// Whether steps p50 stayed flat within [`FLAT`].
    pub fn flat(&self) -> bool {
        (self.steps[1] / self.steps[0] - 1.0).abs() <= FLAT
    }
}

/// The doubling of `regularisation` on `ics` to `horizon`: the base arm at [`BASE_ETA`] on [`DELTA_SYNC`], the
/// doubled on `DELTA_SYNC/factor` at the calibrated `η′` (for `factor` 1, the base arm itself: the null; for `factor`
/// 0, the step jitter, [`JITTER`]). The drift is [`energy_error`] at the horizon. `None` where no `η′` holds steps p50
/// flat.
///
/// For Heggie and logH, registered once, the doubling doubles the sync boundaries and nothing else: what it measures
/// for them is the boundary landings' own effect at fixed step, the cost the mechanism says they do not pay.
///
/// The plan is unclear here (the protocol's "drift-field change in decades" is prin-rs's, whose definition is not in
/// the corpus); chose the per-IC `|Δ log₁₀ drift|` and its p50, with the field's median shift beside it, because a
/// per-pixel distribution is what the field's change is and a median shift alone can hide a field that moved both ways.
pub fn doubling(
    regularisation: Choice,
    factor: u32,
    horizon: f64,
    ics: &[State<f64>],
    threads: usize,
) -> Option<Doubling> {
    let base = arm(regularisation, BASE_ETA, DELTA_SYNC, horizon);
    let b = runs(&base, ics, threads);
    let base_p50 = median(b.iter().map(|t| f64::from(t.steps)));
    let (doubled, d) = if factor == 1 {
        (base, b.clone())
    } else if factor == 0 {
        let jittered = arm(regularisation, BASE_ETA * JITTER, DELTA_SYNC, horizon);
        (jittered, runs(&jittered, ics, threads))
    } else {
        let (eta, _) = calibrate_eta(regularisation, factor, horizon, ics, base_p50, threads)?;
        let doubled = arm(regularisation, eta, DELTA_SYNC / f64::from(factor), horizon);
        (doubled, runs(&doubled, ics, threads))
    };
    let kb = base.discretisation.boundaries()?;
    let kd = doubled.discretisation.boundaries()?;
    let drift = |t: &compare::Trajectory, k: u32, s0: &State<f64>| {
        t.reached(k)
            .then(|| energy_error(s0, &t.state).max(DRIFT_FLOOR).log10())
    };
    let mut change = Vec::with_capacity(ics.len());
    let (mut l1, mut l2) = (Vec::new(), Vec::new());
    let mut exact = true;
    let mut any = false;
    for ((tb, td), s0) in b.iter().zip(&d).zip(ics) {
        match (drift(tb, kb, s0), drift(td, kd, s0)) {
            (Some(x), Some(y)) => {
                change.push((y - x).abs());
                l1.push(x);
                l2.push(y);
                // The harness counts re-registrations after `t = 0`; the registrations, that one included, are one per
                // boundary before the last, `k` base and `2k` doubled, and are what the protocol doubles.
                exact &= td.re_registrations + 1 == 2 * (tb.re_registrations + 1);
                any |= tb.re_registrations > 0;
            }
            _ => change.push(f64::NAN),
        }
    }
    Some(Doubling {
        regularisation,
        factor,
        base,
        doubled,
        steps: [base_p50, median(d.iter().map(|t| f64::from(t.steps)))],
        re_registrations: [
            median(b.iter().map(|t| f64::from(t.re_registrations))),
            median(d.iter().map(|t| f64::from(t.re_registrations))),
        ],
        doubled_exactly: exact && any,
        change,
        median_log_drift: [median(l1), median(l2)],
    })
}

/// The doubling's row.
pub fn doubling_row(case: &str, d: &Doubling) -> String {
    format!(
        "| {case} | {} | {} | {:.4} -> {:.4} | {:.0} -> {:.0} | {:.0} -> {:.0} | {} | {:.2} -> {:.2} | {:.3e} | {} |\n",
        crate::pair_table::label(d.regularisation),
        match d.factor {
            0 => format!("jitter (eta x{JITTER})"),
            1 => "null".to_string(),
            f => format!("x{f}"),
        },
        d.base.step_control.eta,
        d.doubled.step_control.eta,
        d.steps[0],
        d.steps[1],
        d.re_registrations[0],
        d.re_registrations[1],
        d.doubled_exactly,
        d.median_log_drift[0],
        d.median_log_drift[1],
        d.change_p50(),
        quantiles(&d.change),
    )
}

/// The doubling table's header.
pub const DOUBLING_HEADER: &str = "| case | occupant | factor | eta | steps p50 | re-registrations p50 | doubled exactly | \
median log10 drift | change p50 (decades) | change quantiles |\n|---|---|---|---|---|---|---|---|---|---|\n";

/// prin-rs's doubling controls (pitfalls §4.1), reported beside, never asserted.
pub const PRIN_RS_CONTROLS: &str =
    "LC branch unconditioned 2.5e-6 decades; hysteresis (switches 17.8 -> 6.7) 7.5e-5 \
decades; re-registration x2 at fixed step (AZ) 4.4e-1 decades";

/// The default occupant's acceptance level for the drift change under doubled re-registration, in decades (p50 over
/// the ICs). The plan is unclear here (it asks for a calibrated level, and the rule below, fixed before the run, gave
/// none: at 64² on [`DOUBLING_CASES`] the default's change p50 is `4.5e-2` and AZ's `1.0e-1`, no decade gap); chose to
/// propose no value rather than a rule written after the run, so the gate asserts no level and the owner decides.
pub const ACCEPTANCE_LEVEL: Option<f64> = None;

/// The rule for the acceptance level, fixed before any run: where the default occupant's change p50 sits at least a
/// decade below Aarseth–Zare's (a measured gap), the level is their geometric mean rounded up to one significant
/// figure, so that it sits in the gap; with no such gap, no level is proposed.
pub fn propose_level(default_p50: f64, az_p50: f64) -> Option<f64> {
    if !(default_p50.is_finite() && az_p50.is_finite() && az_p50 > 0.0) {
        return None;
    }
    let floor = default_p50.max(DRIFT_FLOOR);
    if az_p50 < 10.0 * floor {
        return None;
    }
    let g = (floor * az_p50).sqrt();
    let scale = 10f64.powf(g.log10().floor());
    Some((g / scale).ceil() * scale)
}

// ── The wedges under the doubled count ─────────────────────────────────────────────────────────────────────────

/// The f32 field's Heggie values (`engine::field::heggie_params`) on `Δ_sync/factor` at `eta`.
pub fn field_params(eta: f32, factor: u32) -> HgParams<f32> {
    let ship = engine::field::heggie_params();
    HgParams {
        eta,
        delta_sync: ship.delta_sync / factor as f32,
        ..ship
    }
}

/// The wedge census of `config_stability` (the step-limit ablation's: damaged is an energy error above [`DAMAGED`],
/// dense is more than a quarter of the window damaged) marched by Heggie × RK4 at f32 under `params`: `(non-finite,
/// damaged, dense)` over the crop.
pub fn heggie_census(
    slice: &Slice,
    raster: Raster,
    params: &HgParams<f32>,
    threads: usize,
) -> (usize, usize, usize) {
    census(slice, raster, &Field::Heggie(*params), threads)
}

/// A field occupant at f32 for the wedge census.
#[derive(Clone, Copy, Debug)]
pub enum Field {
    /// Heggie × RK4 under these values.
    Heggie(HgParams<f32>),
    /// Aarseth–Zare × RK4 under these values with the fixed dτ mode (the step-limit ablation's `none` arm when the
    /// values are its).
    AzFixed(AzParams<f32>),
}

/// The step-limit ablation's subject, Aarseth–Zare with every switch off, on `Δ_sync/factor` at `factor·η`.
pub fn az_none(factor: u32) -> AzParams<f32> {
    let p = ablation::aarseth_zare(ablation::ARMS[0].1);
    AzParams {
        eta: p.eta * factor as f32,
        delta_sync: p.delta_sync / factor as f32,
        ..p
    }
}

/// [`heggie_census`] for any [`Field`].
pub fn census(
    slice: &Slice,
    raster: Raster,
    field: &Field,
    threads: usize,
) -> (usize, usize, usize) {
    let ics = ablation::ics(slice, raster);
    let delta_sync = match field {
        Field::Heggie(p) => f64::from(p.delta_sync),
        Field::AzFixed(p) => f64::from(p.delta_sync),
    };
    let k = kernel::regularisation::sync::boundary_at_or_before(slice.horizon, delta_sync);
    let threads = threads.clamp(1, ics.len().max(1));
    let marched: Vec<Option<(End, f64)>> = std::thread::scope(|s| {
        let lanes: Vec<_> = (0..threads)
            .map(|lane| {
                let ics = &ics;
                s.spawn(move || {
                    (lane..ics.len())
                        .step_by(threads)
                        .map(|i| {
                            let p = ics[i].map(|ic| {
                                let s0 = State {
                                    m: ic.m,
                                    r: ic.r,
                                    p: ic.p,
                                };
                                let (end, state) = match field {
                                    Field::Heggie(p) => {
                                        let o = hg::march::<Rk4, f32, _, _>(
                                            &s0,
                                            k,
                                            p,
                                            &mut Quiet,
                                            &mut ExplicitTest,
                                        );
                                        (o.count.end, o.state)
                                    }
                                    Field::AzFixed(p) => {
                                        let o = az::march::<Rk4, f32, _, _>(
                                            &s0,
                                            0,
                                            k,
                                            AzCount::new(),
                                            p,
                                            &mut Quiet,
                                            &mut ablation::Fixed,
                                        );
                                        (o.count.end, o.state)
                                    }
                                };
                                let wide = |s: &State<f32>| State {
                                    m: s.m.map(f64::from),
                                    r: s.r.map(|v| v.map(f64::from)),
                                    p: s.p.map(|v| v.map(f64::from)),
                                };
                                (end, energy_error(&wide(&s0), &wide(&state)))
                            });
                            (i, p)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let mut out = vec![None; ics.len()];
        for lane in lanes {
            for (i, p) in lane.join().expect("a census worker panicked") {
                out[i] = p;
            }
        }
        out
    });
    let non_finite = marched
        .iter()
        .flatten()
        .filter(|(e, _)| *e == End::NonFinite)
        .count();
    let mask: Vec<bool> = marched
        .iter()
        .map(|p| p.is_some_and(|(e, x)| e != End::NonFinite && (x > DAMAGED || x.is_nan())))
        .collect();
    let damaged = mask.iter().filter(|&&b| b).count();
    let dense = dense_mask(&mask, raster.side, dense_half(raster.n))
        .iter()
        .filter(|&&b| b)
        .count();
    (non_finite, damaged, dense)
}

/// The f32 field's `η′` under the doubled count on `config_stability`'s crop: [`calibrate_eta`]'s rule on the f32
/// march's steps p50.
pub fn field_eta(
    slice: &Slice,
    raster: Raster,
    factor: u32,
    threads: usize,
) -> Option<(f32, f64, f64)> {
    let ics: Vec<State<f32>> = ablation::ics(slice, raster)
        .into_iter()
        .flatten()
        .map(|ic| State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        })
        .collect();
    let p50 = |params: &HgParams<f32>| -> f64 {
        let k = engine::field::boundaries(slice.horizon, params);
        let threads = threads.clamp(1, ics.len().max(1));
        let chunk = ics.len().div_ceil(threads).max(1);
        let steps: Vec<f64> = std::thread::scope(|s| {
            let hs: Vec<_> = ics
                .chunks(chunk)
                .map(|piece| {
                    s.spawn(move || {
                        piece
                            .iter()
                            .map(|s0| {
                                f64::from(
                                    hg::march::<Rk4, f32, _, _>(
                                        s0,
                                        k,
                                        params,
                                        &mut Quiet,
                                        &mut ExplicitTest,
                                    )
                                    .count
                                    .steps,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            hs.into_iter()
                .flat_map(|h| h.join().expect("an eta worker panicked"))
                .collect()
        });
        median(steps)
    };
    let ship = engine::field::heggie_params();
    let base = p50(&ship);
    let mut eta = ship.eta * factor as f32;
    for _ in 0..ETA_TRIES {
        let got = p50(&field_params(eta, factor));
        if (got / base - 1.0).abs() <= FLAT / 2.0 {
            return Some((eta, base, got));
        }
        eta *= (got / base) as f32;
    }
    None
}

// ── The gates ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A control's input: what each of the three gates reads.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    description: String,
    /// What the control checks.
    check: String,
    /// The case, where the check runs one.
    #[serde(default)]
    case: Option<String>,
    /// The raster and the crop `[x0, y0, side]`.
    #[serde(default)]
    raster: Option<usize>,
    #[serde(default)]
    crop: Option<[usize; 3]>,
    /// The re-registration factor, for the doubling's checks.
    #[serde(default)]
    factor: Option<u32>,
    /// The occupants, by label.
    #[serde(default)]
    occupants: Option<Vec<String>>,
}

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn crop(c: &Control, name: &str) -> Result<Raster, String> {
    let n = c.raster.ok_or_else(|| format!("{name}: no raster"))?;
    let [x0, y0, side] = c.crop.unwrap_or([0, 0, n]);
    if x0 + side > n || y0 + side > n || side == 0 {
        return Err(format!("{name}: the crop {:?} is not inside {n}²", c.crop));
    }
    Ok(Raster { n, x0, y0, side })
}

fn choice(label: &str) -> Result<Choice, String> {
    match label {
        "aarseth-zare" => Ok(AZ),
        "heggie" => Ok(HEGGIE),
        "heggie-eq20" => Ok(HEGGIE_EQ20),
        "logh" => Ok(LOGH),
        "none" => Ok(Choice::None),
        other => Err(format!("no occupant `{other}`")),
    }
}

fn verdict(pass: bool) -> Verdict {
    if pass {
        Verdict::Pass
    } else {
        Verdict::Fail
    }
}

fn control_of(name: &str, fixture: &serde_json::Value) -> Result<Control, String> {
    serde_json::from_value(fixture.clone()).map_err(|e| format!("{name}: {e}"))
}

fn write_report(root: &Path, gate: &str, file: &str, text: &str) -> Result<(), String> {
    let dir = crate::gate::out_dir(root).join(gate);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(file);
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{text}\nreport: {}", path.display());
    Ok(())
}

fn res(gate: &str, args: &[&str]) -> Result<usize, String> {
    match args {
        ["--res", n] => n.parse::<usize>().map_err(|e| format!("--res {n}: {e}")),
        _ => Err(format!(
            "{gate} takes `--res <n>`, not `{}`",
            args.join(" ")
        )),
    }
}

/// The gate `heggie-vs-az` (a measurement): its controls check that the default occupant is Heggie under Eq. 22 × RK4,
/// and that the per-IC judgement decides ICs on a crop of a case (a comparison of AZ against the default), its
/// subject control the same comparison with the default against itself, which must decide none. `--res <n>` runs the
/// matrix and writes the report, asserting nothing.
pub struct HeggieVsAzGate;

impl MeasurementGate for HeggieVsAzGate {
    fn name(&self) -> &'static str {
        "heggie-vs-az"
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let c = control_of(name, fixture)?;
        match c.check.as_str() {
            "default" => {
                let ok = Pair::DEFAULT
                    == Pair::select(
                        Selection::Production(Occupant::Rk4),
                        Choice::Heggie(HeggieTime::Eq22),
                    )
                    && DEFAULT_TIME == HeggieTime::Eq22;
                Ok(MeasurementControl {
                    description: format!("{}: Pair::DEFAULT is {:?}", c.description, Pair::DEFAULT),
                    verdict: verdict(ok),
                    markdown: format!(
                        "# {name}\n\n{}\n\nPair::DEFAULT: {:?}\n",
                        c.description,
                        Pair::DEFAULT
                    ),
                })
            }
            "decides" => {
                let case = case_named(&root(), c.case.as_deref().ok_or("no case")?)?;
                let raster = crop(&c, name)?;
                let labels = c
                    .occupants
                    .clone()
                    .ok_or_else(|| format!("{name}: no occupants"))?;
                let [a, b] = <[String; 2]>::try_from(labels)
                    .map_err(|_| format!("{name}: not two occupants"))?;
                let arms =
                    [choice(&a)?, choice(&b)?].map(|r| arm(r, BASE_ETA, DELTA_SYNC, case.horizon));
                let ics: Vec<State<f64>> = ics(&case, raster).into_iter().flatten().collect();
                let cmp =
                    compare_par(&arms, &ics, threads()).map_err(|r| format!("{name}: {r:?}"))?;
                let t = tally(&cmp, 0, 1);
                Ok(MeasurementControl {
                    description: format!(
                        "{}: {a} vs {b} on {} ({} ICs): wins {}/{}, tie {}, unresolved {}, not judged {}",
                        c.description,
                        case.name,
                        ics.len(),
                        t[0],
                        t[1],
                        t[2],
                        t[3],
                        t[4]
                    ),
                    verdict: verdict(t[0] + t[1] > 0),
                    markdown: format!("# {name}\n\n{}\n\n```\n{}```\n", c.description, cmp.report()),
                })
            }
            other => Err(format!("{name}: no check `{other}`")),
        }
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        let n = res(self.name(), args)?;
        let text = heggie_vs_az(root, n, threads())?;
        write_report(root, self.name(), &format!("matrix_{n}.md"), &text)
    }
}

/// The gate `logh-falsification` (a measurement): its controls check that the logH-vs-Heggie harness admits logH and
/// Heggie under one RK4 step control, that it refuses a pair on another stepper (expected refused), and that the
/// judgement decides ICs on a crop. `--res <n>` runs the matrix and writes the report and the verdict, asserting
/// nothing.
pub struct LogHFalsificationGate;

impl MeasurementGate for LogHFalsificationGate {
    fn name(&self) -> &'static str {
        "logh-falsification"
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let c = control_of(name, fixture)?;
        match c.check.as_str() {
            "admits" => {
                let arms = logh_arms(13.0).map_err(|r| format!("{name}: {r:?}"))?;
                Ok(MeasurementControl {
                    description: format!(
                        "{}: admitted as [{}, {}]",
                        c.description,
                        crate::pair_table::label(arms[0].regularisation),
                        crate::pair_table::label(arms[1].regularisation)
                    ),
                    verdict: verdict(
                        arms[0].regularisation == HEGGIE && arms[1].regularisation == LOGH,
                    ),
                    markdown: format!("# {name}\n\n{}\n", c.description),
                })
            }
            "refuses-kdk" => {
                let mut arms = [HEGGIE, LOGH].map(|r| arm(r, BASE_ETA, DELTA_SYNC, 13.0));
                for a in &mut arms {
                    a.stepper = Selection::Production(Occupant::Kdk);
                }
                let got = admit_logh_heggie(arms).map_err(|r| format!("{name}: refused: {r:?}"))?;
                Ok(MeasurementControl {
                    description: format!("{}: admitted {:?}", c.description, got[0].stepper),
                    verdict: Verdict::Pass,
                    markdown: format!("# {name}\n\n{}\n", c.description),
                })
            }
            "decides" => {
                let case = case_named(&root(), c.case.as_deref().ok_or("no case")?)?;
                let raster = crop(&c, name)?;
                let ics: Vec<State<f64>> = ics(&case, raster).into_iter().flatten().collect();
                let arms = logh_arms(case.horizon).map_err(|r| format!("{name}: {r:?}"))?;
                let cmp =
                    compare_par(&arms, &ics, threads()).map_err(|r| format!("{name}: {r:?}"))?;
                let t = tally(&cmp, 0, 1);
                Ok(MeasurementControl {
                    description: format!(
                        "{}: Heggie vs logH on {} ({} ICs): wins {}/{}, tie {}, unresolved {}, not judged {}",
                        c.description,
                        case.name,
                        ics.len(),
                        t[0],
                        t[1],
                        t[2],
                        t[3],
                        t[4]
                    ),
                    verdict: verdict(t[0] + t[1] > 0),
                    markdown: format!("# {name}\n\n{}\n\n```\n{}```\n", c.description, cmp.report()),
                })
            }
            other => Err(format!("{name}: no check `{other}`")),
        }
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        let n = res(self.name(), args)?;
        let text = logh_falsification(root, n, threads())?;
        write_report(root, self.name(), &format!("matrix_{n}.md"), &text)
    }
}

/// The gate `reregistration-doubling` (a measurement): its controls check, on a crop of a case, that the protocol
/// doubles Aarseth–Zare's re-registration count exactly with steps p50 flat within 6% and sees the drift field move
/// (its subject control the null, factor 1, which must fail the doubling check), and that `config_stability` under
/// Heggie at the doubled count has no non-finite and no dense damaged pixel. `--res <n>` measures the doubling on the
/// cases the protocol was measured on, and `--propose --res <n>` adds the acceptance level by [`propose_level`]'s rule;
/// neither asserts anything.
pub struct ReregistrationDoublingGate;

/// The cases the doubling is measured on in `--res`: the slice the wedges were seen on and the five Burrau regions.
pub const DOUBLING_CASES: [&str; 6] = [
    "config_stability",
    "near_field",
    "mid_field",
    "far",
    "body2_core",
    "deep_interior",
];

impl MeasurementGate for ReregistrationDoublingGate {
    fn name(&self) -> &'static str {
        "reregistration-doubling"
    }

    fn control(
        &self,
        _config: &MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<MeasurementControl, String> {
        let c = control_of(name, fixture)?;
        match c.check.as_str() {
            "doubles" => {
                let case = case_named(&root(), c.case.as_deref().ok_or("no case")?)?;
                let raster = crop(&c, name)?;
                let factor = c.factor.ok_or_else(|| format!("{name}: no factor"))?;
                let ics: Vec<State<f64>> = ics(&case, raster).into_iter().flatten().collect();
                let d = doubling(AZ, factor, case.horizon, &ics, threads())
                    .ok_or_else(|| format!("{name}: no eta holds steps p50 flat"))?;
                let pass = d.doubled_exactly && d.flat() && d.change_p50() > 0.0;
                Ok(MeasurementControl {
                    description: format!(
                        "{}: factor {factor}, re-registrations p50 {} -> {}, exact {}, steps p50 {} -> {}, change p50 \
                         {:.3e} decades",
                        c.description,
                        d.re_registrations[0],
                        d.re_registrations[1],
                        d.doubled_exactly,
                        d.steps[0],
                        d.steps[1],
                        d.change_p50()
                    ),
                    verdict: verdict(pass),
                    markdown: format!("# {name}\n\n{}\n\n{DOUBLING_HEADER}{}", c.description, doubling_row(&case.name, &d)),
                })
            }
            "wedges" => {
                let raster = crop(&c, name)?;
                let factor = c.factor.ok_or_else(|| format!("{name}: no factor"))?;
                let slice = ablation::config_stability(&root())?;
                let occupant = c
                    .occupants
                    .as_deref()
                    .and_then(|o| o.first().cloned())
                    .unwrap_or_default();
                let (field, how) = match occupant.as_str() {
                    "heggie" => {
                        let (eta, base, got) = field_eta(&slice, raster, factor, threads())
                            .ok_or_else(|| format!("{name}: no eta holds steps p50 flat"))?;
                        (
                            Field::Heggie(field_params(eta, factor)),
                            format!("Heggie x RK4 f32 at delta_sync/{factor}, eta {eta} (steps p50 {base} -> {got})"),
                        )
                    }
                    "aarseth-zare-none" => (
                        Field::AzFixed(az_none(factor)),
                        format!("Aarseth-Zare x RK4 f32, every switch off, at delta_sync/{factor}, eta x{factor}"),
                    ),
                    other => return Err(format!("{name}: no field occupant `{other}`")),
                };
                let (non_finite, damaged, dense) = census(&slice, raster, &field, threads());
                Ok(MeasurementControl {
                    description: format!(
                        "{}: {how}: non-finite {non_finite}, damaged {damaged}, dense {dense}",
                        c.description
                    ),
                    verdict: verdict(non_finite == 0 && dense == 0),
                    markdown: format!("# {name}\n\n{}\n", c.description),
                })
            }
            other => Err(format!("{name}: no check `{other}`")),
        }
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        let (propose, rest) = match args {
            ["--propose", rest @ ..] => (true, rest),
            rest => (false, rest),
        };
        let n = res(self.name(), rest)?;
        let text = doubling_report(root, n, propose, threads())?;
        write_report(
            root,
            self.name(),
            &format!("{}_{n}.md", if propose { "propose" } else { "doubling" }),
            &text,
        )
    }
}

/// The doubling report on [`DOUBLING_CASES`] at `n × n`: Heggie (the default), Aarseth–Zare and logH, ×2 and the null;
/// with `propose`, the acceptance level by [`propose_level`]'s rule on the pooled per-IC changes, and the wedge census
/// of `config_stability` under the default at the doubled count.
pub fn doubling_report(
    root: &Path,
    n: usize,
    propose: bool,
    threads: usize,
) -> Result<String, String> {
    let mut out = format!(
        "# The re-registration doubling, {n}x{n} per case\n\nBase: RK4, eta {BASE_ETA}, delta_sync {DELTA_SYNC}; \
         doubled: delta_sync / 2, eta adjusted until steps p50 is within {}% (allowed {}%). Drift: |E - E0| / (K0 + \
         |U0|) at the horizon, floored at {DRIFT_FLOOR:e}. Change: per IC |log10 d2 - log10 d1|.\n\n{DOUBLING_HEADER}",
        FLAT * 50.0,
        FLAT * 100.0
    );
    let mut pooled: [Vec<f64>; 3] = Default::default();
    let mut pooled_jitter: [Vec<f64>; 3] = Default::default();
    let occupants = [HEGGIE, AZ, LOGH];
    for name in DOUBLING_CASES {
        let case = case_named(root, name)?;
        let ics: Vec<State<f64>> = ics(&case, Raster::full(n)).into_iter().flatten().collect();
        for (o, r) in occupants.iter().enumerate() {
            let started = std::time::Instant::now();
            for factor in [1, 0, 2] {
                match doubling(*r, factor, case.horizon, &ics, threads) {
                    Some(d) => {
                        match factor {
                            2 => pooled[o].extend(d.change.iter().copied()),
                            0 => pooled_jitter[o].extend(d.change.iter().copied()),
                            _ => {}
                        }
                        out.push_str(&doubling_row(name, &d));
                    }
                    None => out.push_str(&format!(
                        "| {name} | {} | x{factor} | no eta held steps p50 flat: refused | | | | | | |\n",
                        crate::pair_table::label(*r)
                    )),
                }
            }
            eprintln!(
                "doubling: {name} {} in {:.1} s",
                crate::pair_table::label(*r),
                started.elapsed().as_secs_f64()
            );
        }
    }
    out.push_str("\nPooled over the cases, x2, change in decades:\n\n");
    for (o, r) in occupants.iter().enumerate() {
        out.push_str(&format!(
            "- {}: x2 p50 {:.3e} ({}); jitter p50 {:.3e} ({})\n",
            crate::pair_table::label(*r),
            median(pooled[o].iter().copied()),
            quantiles(&pooled[o]),
            median(pooled_jitter[o].iter().copied()),
            quantiles(&pooled_jitter[o])
        ));
    }
    out.push_str(&format!(
        "\nprin-rs's controls, for comparison only: {PRIN_RS_CONTROLS}.\n"
    ));
    if propose {
        let (hg, az) = (
            median(pooled[0].iter().copied()),
            median(pooled[1].iter().copied()),
        );
        out.push_str(&format!(
            "\n## Proposal (asserts nothing)\n\nThe rule, fixed before the run: where the default's change p50 sits a \
             decade or more below AZ's, the level is their geometric mean rounded up to one significant figure.\n\n\
             Default (Heggie Eq. 22 x RK4) change p50 {hg:.3e}; AZ {az:.3e}; ratio {:.1}.\nProposed acceptance level \
             for the default occupant's drift change under doubled re-registration: {}.\n",
            az / hg.max(DRIFT_FLOOR),
            match propose_level(hg, az) {
                Some(l) => format!("{l:e} decades (p50 over the ICs)"),
                None => "none: no decade gap between the default and AZ".into(),
            }
        ));
        let slice = ablation::config_stability(root)?;
        let raster = Raster::full(n);
        match field_eta(&slice, raster, 2, threads) {
            Some((eta, base, got)) => {
                let (nf, damaged, dense) =
                    heggie_census(&slice, raster, &field_params(eta, 2), threads);
                let (nf1, damaged1, dense1) =
                    heggie_census(&slice, raster, &engine::field::heggie_params(), threads);
                out.push_str(&format!(
                    "\nconfig_stability at {n}x{n}, Heggie x RK4 at f32: base non-finite {nf1}, damaged {damaged1}, \
                     dense {dense1}; doubled (delta_sync/2, eta {eta}, steps p50 {base} -> {got}) non-finite {nf}, \
                     damaged {damaged}, dense {dense}.\n"
                ));
            }
            None => out
                .push_str("\nconfig_stability: no f32 eta held steps p50 flat; census not run.\n"),
        }
    }
    Ok(out)
}
