//! The step-limit ablation (pitfalls §8, "Two artefacts are not one defect"; TASK-M3-31): `config_stability`
//! (`fixtures/slices.toml`) rendered on the CPU under five arms of the regularised step control, the three switches of
//! prin-rs `examples/wedge_census.rs` off one at a time — none, dtau only, clamp only, limit only, all three — on Heggie
//! × RK4, the shipping occupant, with Aarseth–Zare × RK4 kept for comparison; and two columns measured separately per
//! arm: the **non-finite** count and the **wedge density**.
//!
//! **The switches.** "dtau" is the per-step dτ mode (`kernel::regularisation::{heggie, az}`'s watch `PER_STEP`, a
//! type-level switch: off is the [`Fixed`] watch, which only this module builds); "limit" the predictive step limit
//! (`predictive_f`, `0.02` on, `0` off); "clamp" the landing. The plan is unclear here; chose to turn the landing's
//! secant and bisection retakes off with the clamp, because prin-rs's "clamp" is the whole landing (an unclamped
//! prin-rs step overshoots the boundary and only the clock is corrected), and here a step past the boundary is retaken
//! back onto it even with the clamp off (`HgParams::clamp`'s note): with the clamp alone off the arm would not
//! measure what prin-rs's did. Every arm's other values are the shipping ones, [`engine::field::heggie_params`], at f32,
//! the precision the CPU field marches at.
//!
//! **The march.** Each pixel is decoded at f64 and narrowed once to f32, as `engine::field` does, and marched to the
//! slice's horizon over the sync grid with no cadence: no projection and no terminal detection, so every trajectory
//! that stays finite reaches the horizon or spends its step budget, and no pixel is frozen at its own stopping time
//! (pitfalls §1: a patchwork of stopping times is an artefact of its own, and would be read as one here).
//!
//! **Non-finite** is a march that ended at the explicit finiteness test (`End::NonFinite`): the magenta pixels.
//! **Exhausted** (`End::Exhausted`, the step budget spent) is its own column, never folded into either.
//!
//! **The wedge density.** prin-rs's wedges are pale patches of the rendered `spread_shape` panel, the spread of an
//! ensemble of jittered copies saturating where the copies "diverged to garbage", and its density is the fraction of
//! pixels with at least 25% pale pixels in a 9×9 window at 1024² (`dense_mask`, verbatim). The plan is unclear here;
//! chose a field-space "pale": a pixel is **damaged** when it ended finite with a Cartesian energy error
//! `|E − E₀| / (K₀ + |U₀|)` above [`DAMAGED`], because the render-space mask needs prin-rs's colouring and ensemble,
//! neither of which is in this repository, a render-space criterion is void here (pitfalls §5), and garbage from an
//! unresolved pericentre step is what the energy error sees directly, on one trajectory, chaos or not (energy is
//! conserved in a chaotic field). The dense rule is prin-rs's, its window scaled to hold the same area of the slice at
//! any raster, as `wedge_census` scales it. So the density is this repository's measure of the same object, and
//! prin-rs's numbers (1153 / 0.0026 / 0.0001 on Aarseth–Zare at 512²) are reported beside it, never asserted.
//!
//! **Domain** (pitfalls §10): one slice, one horizon, the shipping step-control values; Principia's decode, not
//! prin-rs's, so the ICs are this repository's `config_stability`, not prin-rs's pixels.

use std::path::Path;

use engine::field::{boundaries, heggie_params};
use engine::first_slice::narrow;
use kernel::chart::{Chart, Latent};
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::State;
use kernel::regularisation::az::{self, AzCount, AzP, AzParams, AzQ, End, ExplicitTest, Watch};
use kernel::regularisation::heggie::{self as hg, HgP, HgParams, HgQ, HgWatch};

use crate::az::Quiet;

// ── The slice ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A latent slice from `fixtures/slices.toml`: the chart over the slice's window, `z = z₀ + (2u − 1)·q₁ + (2v − 1)·q₂`,
/// and its horizon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slice {
    /// The chart, the window folded into its plane.
    pub chart: Latent<f64>,
    /// The horizon.
    pub horizon: f64,
}

/// The numbers of `[name]`'s line `key = …` in `text` (a `slices.toml`): a scalar, or an array's elements.
pub fn numbers(text: &str, name: &str, key: &str) -> Result<Vec<f64>, String> {
    let section = format!("[{name}]");
    let mut lines = text.lines().skip_while(|l| l.trim() != section);
    lines
        .next()
        .ok_or_else(|| format!("slices.toml has no {section}"))?;
    let line = lines
        .take_while(|l| !l.trim_start().starts_with('['))
        .find(|l| l.split('=').next().map(str::trim) == Some(key))
        .ok_or_else(|| format!("slices.toml's {section} has no `{key}`"))?;
    let value = line.split_once('=').map(|(_, v)| v).unwrap_or("");
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|x| {
            x.trim()
                .parse::<f64>()
                .map_err(|e| format!("slices.toml {section} `{key}`: {x:?}: {e}"))
        })
        .collect()
}

/// The latent slice `name` of `text` (a `slices.toml`), by its config-slice rule (`slices.toml`'s note, prin-rs
/// `grid.rs:449-468`): the reference's ten-slot `z` renumbered into the spec's order (`z_α = z[1]`, `z_β = z[0]`, `z_q =
/// z[4..8]`, `z_μ = z[8..10]`), the window's centre `2·pan − 1 + zoom` and half-width `zoom` along `mag·ê_β`
/// (horizontal) and `mag·ê_α` (vertical). On this repository's latent chart the window is the plane: `z₀` the base
/// point moved to the window's centre, `q₁ = half·mag·ê_β`, `q₂ = half·mag·ê_α`.
pub fn latent_slice(text: &str, name: &str) -> Result<Slice, String> {
    let z = numbers(text, name, "z_glsl")?;
    let one = |key: &str| -> Result<f64, String> {
        match numbers(text, name, key)?.as_slice() {
            [x] => Ok(*x),
            other => Err(format!(
                "slices.toml [{name}] `{key}`: {other:?} is not one number"
            )),
        }
    };
    let pan = numbers(text, name, "pan")?;
    let (zoom, mag, horizon) = (one("zoom")?, one("mag")?, one("horizon")?);
    if z.len() != 10 || pan.len() != 2 {
        return Err(format!(
            "slices.toml [{name}]: z_glsl or pan has the wrong length"
        ));
    }
    let (cx, cy) = (2.0 * pan[0] - 1.0 + zoom, 2.0 * pan[1] - 1.0 + zoom);
    let (alpha, beta) = (0, 1);
    let mut z0 = [z[1], z[0], z[4], z[5], z[6], z[7], z[8], z[9]];
    z0[beta] += cx * mag;
    z0[alpha] += cy * mag;
    let mut q1 = [0.0; 8];
    let mut q2 = [0.0; 8];
    q1[beta] = zoom * mag;
    q2[alpha] = zoom * mag;
    Ok(Slice {
        chart: Latent { z0, q1, q2 },
        horizon,
    })
}

/// `config_stability` from the workspace at `root`.
pub fn config_stability(root: &Path) -> Result<Slice, String> {
    let path = root.join("fixtures/slices.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    latent_slice(&text, "config_stability")
}

/// A square piece of an `n × n` raster of a slice: its `side × side` pixels from `(x0, y0)`, pixel `(x, y)` of the
/// raster at `u = (x + ½)/n`, `v = (y + ½)/n`, row by row from the bottom left. The dense window is the whole raster's
/// ([`dense_half`] of `n`), so a crop judges its pixels as the full raster would, away from the crop's own border.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Raster {
    /// The whole raster's side.
    pub n: usize,
    /// The crop's first column.
    pub x0: usize,
    /// The crop's first row.
    pub y0: usize,
    /// The crop's side.
    pub side: usize,
}

impl Raster {
    /// The whole `n × n` raster.
    pub fn full(n: usize) -> Self {
        Raster {
            n,
            x0: 0,
            y0: 0,
            side: n,
        }
    }

    /// The crop's pixels.
    pub fn len(&self) -> usize {
        self.side * self.side
    }

    /// Whether the crop has no pixel.
    pub fn is_empty(&self) -> bool {
        self.side == 0
    }

    /// The chart point `(u, v)` of the crop's pixel `i`.
    pub fn uv(&self, i: usize) -> (f64, f64) {
        let x = (self.x0 + i % self.side) as f64 + 0.5;
        let y = (self.y0 + i / self.side) as f64 + 0.5;
        (x / self.n as f64, y / self.n as f64)
    }
}

/// The IC of `slice` at the chart point `(u, v)`: decoded at f64 and narrowed once to f32; `None` where it is not
/// finite.
pub fn ic(slice: &Slice, (u, v): (f64, f64)) -> Option<Ic<f32>> {
    let ic = narrow(&slice.chart.decode::<Defaults, f64>(u, v).ic);
    let mut all =
        ic.m.iter()
            .chain(ic.r.iter().flatten())
            .chain(ic.p.iter().flatten());
    all.all(|x| x.is_finite()).then_some(ic)
}

/// The ICs of `raster` on `slice`.
pub fn ics(slice: &Slice, raster: Raster) -> Vec<Option<Ic<f32>>> {
    (0..raster.len()).map(|i| ic(slice, raster.uv(i))).collect()
}

// ── The arms ───────────────────────────────────────────────────────────────────────────────────────────────────

/// The three switches, each on as the occupant ships it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Switches {
    /// The per-step dτ mode (off: every step of an interval at the entry size).
    pub dtau: bool,
    /// The landing: the clamp and its retakes.
    pub clamp: bool,
    /// The predictive step limit.
    pub limit: bool,
}

impl Switches {
    /// The shipping step control: all three on.
    pub const SHIPPING: Switches = Switches {
        dtau: true,
        clamp: true,
        limit: true,
    };
}

/// The ablation's five arms, in pitfalls §8's order.
pub const ARMS: [(&str, Switches); 5] = [
    (
        "none",
        Switches {
            dtau: false,
            clamp: false,
            limit: false,
        },
    ),
    (
        "dtau only",
        Switches {
            dtau: true,
            clamp: false,
            limit: false,
        },
    ),
    (
        "clamp only",
        Switches {
            dtau: false,
            clamp: true,
            limit: false,
        },
    ),
    (
        "limit only",
        Switches {
            dtau: false,
            clamp: false,
            limit: true,
        },
    ),
    ("all three", Switches::SHIPPING),
];

/// The occupant an arm runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Occupant {
    /// Heggie × RK4, the shipping default.
    Heggie,
    /// Aarseth–Zare × RK4, kept for comparison (the occupant pitfalls §8 was measured on).
    AarsethZare,
}

impl Occupant {
    /// Its name in the report.
    pub fn name(self) -> &'static str {
        match self {
            Occupant::Heggie => "Heggie x RK4",
            Occupant::AarsethZare => "Aarseth-Zare x RK4",
        }
    }
}

/// Heggie's values under `switches`: the shipping ones ([`heggie_params`]) with the limit's `f` at 0 where the limit is
/// off, and the clamp and the landing's retakes off where the clamp is (the module's note).
pub fn heggie(switches: Switches) -> HgParams<f32> {
    let ship = heggie_params();
    HgParams {
        predictive_f: if switches.limit {
            ship.predictive_f
        } else {
            0.0
        },
        clamp: switches.clamp,
        secant_tries: if switches.clamp { ship.secant_tries } else { 0 },
        bisect_tries: if switches.clamp { ship.bisect_tries } else { 0 },
        ..ship
    }
}

/// Aarseth–Zare's values under `switches`, read from [`heggie`]'s field by field.
pub fn aarseth_zare(switches: Switches) -> AzParams<f32> {
    let h = heggie(switches);
    AzParams {
        eta: h.eta,
        delta_sync: h.delta_sync,
        tiny: h.tiny,
        predictive_f: h.predictive_f,
        step_budget_rate: h.step_budget_rate,
        n_min: h.n_min,
        clamp: h.clamp,
        secant_tries: h.secant_tries,
        bisect_tries: h.bisect_tries,
    }
}

/// The fixed dτ mode's watch: the occupant's finiteness test, every step of an interval at its entry size.
#[derive(Clone, Copy, Debug)]
pub struct Fixed;

impl<R: az::AzReal> HgWatch<R> for Fixed {
    fn finite(&mut self, q: &HgQ<R>, p: &HgP<R>) -> bool {
        HgWatch::finite(&mut ExplicitTest, q, p)
    }
    const PER_STEP: bool = false;
}

impl<R: az::AzReal> Watch<R> for Fixed {
    fn finite(&mut self, q: &AzQ<R>, p: &AzP<R>) -> bool {
        Watch::finite(&mut ExplicitTest, q, p)
    }
    const PER_STEP: bool = false;
}

// ── One pixel ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A pixel's damage threshold on the Cartesian energy error `|E − E₀| / (K₀ + |U₀|)`: a tenth of the IC's energy
/// scale. Set before the first run, from the object, not from a distribution: a damaged pixel is garbage, an error of
/// the order of the energy itself, where an integration that resolves its pericentres errs by many decades less at
/// `η = 0.01` (the census reports the error's decades, so where this sits against the field's own gap is read off
/// every run).
pub const DAMAGED: f64 = 0.1;

/// How one pixel's march ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pixel {
    /// How the march ended.
    pub end: End,
    /// The energy error `|E − E₀| / (K₀ + |U₀|)` of the state where the march ended, at f64.
    pub energy_error: f64,
    /// The steps whose size the predictive limit cut.
    pub predictive_bound: u32,
    /// The last boundary reached.
    pub boundary: u32,
    /// The state where the march ended, its positions and momenta as IEEE bits.
    pub bits: [u32; 12],
}

impl Pixel {
    /// Whether the march ended at the explicit finiteness test.
    pub fn non_finite(&self) -> bool {
        self.end == End::NonFinite
    }

    /// Whether it spent its step budget.
    pub fn exhausted(&self) -> bool {
        self.end == End::Exhausted
    }

    /// Whether it ended finite with an energy error above [`DAMAGED`].
    pub fn damaged(&self) -> bool {
        !self.non_finite() && (self.energy_error > DAMAGED || self.energy_error.is_nan())
    }
}

/// `(K, U)` of `s`, at f64.
fn energies(s: &State<f32>) -> (f64, f64) {
    let m = s.m.map(f64::from);
    let r = s.r.map(|v| v.map(f64::from));
    let p = s.p.map(|v| v.map(f64::from));
    let k: f64 = (0..3)
        .map(|i| (p[i][0].powi(2) + p[i][1].powi(2)) / (2.0 * m[i]))
        .sum();
    let u: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| -m[i] * m[j] / (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]))
        .sum();
    (k, u)
}

fn bits(s: &State<f32>) -> [u32; 12] {
    let mut out = [0; 12];
    for (o, x) in out
        .iter_mut()
        .zip(s.r.iter().flatten().chain(s.p.iter().flatten()))
    {
        *o = x.to_bits();
    }
    out
}

/// One pixel from `ic`, marched by `occupant` under `switches` over `k_target` sync boundaries.
pub fn pixel(occupant: Occupant, switches: Switches, ic: &Ic<f32>, k_target: u32) -> Pixel {
    let s0 = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let (state, end, predictive_bound, boundary) = match (occupant, switches.dtau) {
        (Occupant::Heggie, dtau) => {
            let params = heggie(switches);
            let out = if dtau {
                hg::march::<Rk4, f32, _, _>(&s0, k_target, &params, &mut Quiet, &mut ExplicitTest)
            } else {
                hg::march::<Rk4, f32, _, _>(&s0, k_target, &params, &mut Quiet, &mut Fixed)
            };
            (
                out.state,
                out.count.end,
                out.count.predictive_bound,
                out.boundary,
            )
        }
        (Occupant::AarsethZare, dtau) => {
            let params = aarseth_zare(switches);
            let out = if dtau {
                az::march::<Rk4, f32, _, _>(
                    &s0,
                    0,
                    k_target,
                    AzCount::new(),
                    &params,
                    &mut Quiet,
                    &mut ExplicitTest,
                )
            } else {
                az::march::<Rk4, f32, _, _>(
                    &s0,
                    0,
                    k_target,
                    AzCount::new(),
                    &params,
                    &mut Quiet,
                    &mut Fixed,
                )
            };
            (
                out.state,
                out.count.end,
                out.count.predictive_bound,
                out.boundary,
            )
        }
    };
    let (k0, u0) = energies(&s0);
    let (k, u) = energies(&state);
    Pixel {
        end,
        energy_error: ((k + u) - (k0 + u0)).abs() / (k0 + u0.abs()),
        predictive_bound,
        boundary,
        bits: bits(&state),
    }
}

// ── The census ─────────────────────────────────────────────────────────────────────────────────────────────────

/// prin-rs `examples/circled_ics.rs:288`'s dense rule, verbatim (`wedge_census.rs`'s `dense_mask`): a pixel of `mask`
/// (`n × n`) is dense when more than a quarter of the `(2·half + 1)²` window about it is in `mask`; the `half`-wide
/// border is never dense.
pub fn dense_mask(mask: &[bool], n: usize, half: usize) -> Vec<bool> {
    let w = 2 * half + 1;
    let area = w * w;
    let mut d = vec![false; n * n];
    for y in half..n.saturating_sub(half) {
        for x in half..n.saturating_sub(half) {
            if !mask[y * n + x] {
                continue;
            }
            let mut c = 0usize;
            for dy in 0..w {
                for dx in 0..w {
                    if mask[(y + dy - half) * n + (x + dx - half)] {
                        c += 1;
                    }
                }
            }
            d[y * n + x] = c * 4 > area;
        }
    }
    d
}

/// The dense window's half-width at an `n × n` raster: 9×9 at 1024², scaled to hold the same area of the slice, at
/// least 3×3 (`wedge_census.rs`'s `dense_half`).
pub fn dense_half(n: usize) -> usize {
    ((4 * n) / 1024).max(1)
}

/// One arm's two columns, and what beside them.
#[derive(Clone, Debug, PartialEq)]
pub struct Census {
    /// The occupant.
    pub occupant: Occupant,
    /// The arm's name.
    pub arm: &'static str,
    /// The switches.
    pub switches: Switches,
    /// The raster, or the crop of it, measured.
    pub raster: Raster,
    /// The pixels whose decoded IC is not finite, which are not marched.
    pub undecoded: usize,
    /// The non-finite column: marches that ended at the explicit finiteness test.
    pub non_finite: usize,
    /// Marches that spent their step budget.
    pub exhausted: usize,
    /// Damaged pixels ([`Pixel::damaged`]).
    pub damaged: usize,
    /// Dense damaged pixels ([`dense_mask`] at [`dense_half`]).
    pub dense: usize,
    /// The steps the predictive limit cut, over every pixel.
    pub predictive_bound: u64,
    /// The finite pixels' energy errors by decade: `[< 1e-8, 1e-8.., …, ≥ 1e2]`, eleven bins.
    pub decades: [usize; 11],
    /// Each pixel, `None` where its IC was not finite.
    pub pixels: Vec<Option<Pixel>>,
}

impl Census {
    /// The wedge density: dense damaged pixels over the measured pixels.
    pub fn density(&self) -> f64 {
        self.dense as f64 / self.raster.len() as f64
    }

    /// The report's row.
    pub fn row(&self) -> String {
        format!(
            "| {} | {} | {} | {} | {} | {:.4} | {} |",
            self.arm,
            self.non_finite,
            self.exhausted,
            self.damaged,
            self.dense,
            self.density(),
            self.predictive_bound
        )
    }
}

/// The census of `occupant` under `switches` on the ICs `ics` of `raster`, over `k_target` sync boundaries, on
/// `threads` threads.
pub fn census(
    occupant: Occupant,
    (arm, switches): (&'static str, Switches),
    ics: &[Option<Ic<f32>>],
    raster: Raster,
    k_target: u32,
    threads: usize,
) -> Census {
    let threads = threads.clamp(1, ics.len().max(1));
    let mut pixels: Vec<Option<Pixel>> = vec![None; ics.len()];
    std::thread::scope(|s| {
        let lanes: Vec<_> = (0..threads)
            .map(|lane| {
                s.spawn(move || {
                    (lane..ics.len())
                        .step_by(threads)
                        .map(|i| (i, ics[i].map(|ic| pixel(occupant, switches, &ic, k_target))))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for lane in lanes {
            for (i, p) in lane.join().expect("an ablation worker panicked") {
                pixels[i] = p;
            }
        }
    });
    let marched = || pixels.iter().flatten();
    let mask: Vec<bool> = pixels
        .iter()
        .map(|p| p.is_some_and(|p| p.damaged()))
        .collect();
    let mut decades = [0usize; 11];
    for p in marched().filter(|p| !p.non_finite()) {
        let e = p.energy_error;
        let bin = if e.is_nan() {
            10
        } else {
            ((e.log10() + 9.0).floor().max(0.0) as usize).min(10)
        };
        decades[bin] += 1;
    }
    Census {
        occupant,
        arm,
        switches,
        raster,
        undecoded: pixels.iter().filter(|p| p.is_none()).count(),
        non_finite: marched().filter(|p| p.non_finite()).count(),
        exhausted: marched().filter(|p| p.exhausted()).count(),
        damaged: mask.iter().filter(|&&b| b).count(),
        dense: dense_mask(&mask, raster.side, dense_half(raster.n))
            .iter()
            .filter(|&&b| b)
            .count(),
        predictive_bound: marched().map(|p| u64::from(p.predictive_bound)).sum(),
        decades,
        pixels,
    }
}

/// The sync boundaries a march to `slice`'s horizon takes, on the shipping grid.
pub fn k_target(slice: &Slice) -> u32 {
    boundaries(slice.horizon, &heggie_params())
}

/// The censuses of `occupant` on `raster` of `slice` under `arms`.
pub fn ablation(
    occupant: Occupant,
    slice: &Slice,
    raster: Raster,
    arms: &[(&'static str, Switches)],
    threads: usize,
) -> Vec<Census> {
    let ics = ics(slice, raster);
    let k = k_target(slice);
    arms.iter()
        .map(|&arm| census(occupant, arm, &ics, raster, k, threads))
        .collect()
}

/// The report's table of `rows`, the two columns separate.
pub fn table(rows: &[Census]) -> String {
    let mut out = String::from(
        "| arm | non-finite | exhausted | damaged | dense | wedge density | limit-cut steps |\n|---|---|---|---|---|---|---|\n",
    );
    for r in rows {
        out.push_str(&r.row());
        out.push('\n');
    }
    out
}

// ── The gate ───────────────────────────────────────────────────────────────────────────────────────────────────

/// prin-rs's ablation on Aarseth–Zare at 512² (pitfalls §8), reported beside this repository's, never asserted.
pub const PRIN_RS: &str = "| arm | non-finite | wedge density |\n|---|---|---|\n| none | 1153 | 0.0026 |\n\
| dtau only | 32 | 0.0026 |\n| clamp only | 1531 | 0.0024 |\n| limit only | 0 | 0.0001 |\n| all three | 0 | 0.0001 |\n";

/// The full report at `n × n` for both occupants: each one's table, its energy-error decades per arm, and prin-rs's.
pub fn report(slice: &Slice, n: usize, threads: usize) -> String {
    let mut out = format!(
        "# The step-limit ablation on config_stability, {n}x{n}, t = {}\n\nDense window {w}x{w}; damaged: energy error \
         above {DAMAGED}.\n",
        slice.horizon,
        w = 2 * dense_half(n) + 1
    );
    for occupant in [Occupant::Heggie, Occupant::AarsethZare] {
        let started = std::time::Instant::now();
        let rows = ablation(occupant, slice, Raster::full(n), &ARMS, threads);
        out.push_str(&format!(
            "\n## {} ({:.1} s)\n\n{}\nEnergy error of the finite pixels by decade (<1e-8, [1e-8, 1e-7), …, >=1e1 or \
             NaN):\n\n",
            occupant.name(),
            started.elapsed().as_secs_f64(),
            table(&rows)
        ));
        for r in &rows {
            out.push_str(&format!("- {}: {:?}\n", r.arm, r.decades));
        }
    }
    out.push_str("\n## prin-rs, Aarseth-Zare at 512^2 (pitfalls §8), for comparison only\n\n");
    out.push_str(PRIN_RS);
    out
}

/// The gate `step-limit-ablation` (a measurement, R-649): its controls judge one arm on a crop of `config_stability`
/// each, passing where the arm leaves no non-finite pixel and no dense damaged pixel, the shipping step control's two
/// columns at zero; its subject control, Aarseth–Zare with all three switches off, is expected to fail, so that a
/// judgement that cannot fail is caught. `--res <n>` runs the full ablation at `n × n` on both occupants and writes
/// the report, asserting nothing.
pub struct StepLimitAblationGate;

/// A control's input.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    description: String,
    occupant: String,
    arm: String,
    raster: usize,
    crop: [usize; 3],
}

fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

impl crate::gate::MeasurementGate for StepLimitAblationGate {
    fn name(&self) -> &'static str {
        "step-limit-ablation"
    }

    fn control(
        &self,
        _config: &crate::gate::MeasurementConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<crate::gate::MeasurementControl, String> {
        let c: Control =
            serde_json::from_value(fixture.clone()).map_err(|e| format!("{name}: {e}"))?;
        let occupant = match c.occupant.as_str() {
            "heggie" => Occupant::Heggie,
            "aarseth-zare" => Occupant::AarsethZare,
            other => return Err(format!("{name}: no occupant `{other}`")),
        };
        let arm = *ARMS
            .iter()
            .find(|(a, _)| *a == c.arm)
            .ok_or_else(|| format!("{name}: no arm `{}`", c.arm))?;
        let [x0, y0, side] = c.crop;
        if x0 + side > c.raster || y0 + side > c.raster || side == 0 {
            return Err(format!(
                "{name}: the crop {:?} is not inside {}²",
                c.crop, c.raster
            ));
        }
        let raster = Raster {
            n: c.raster,
            x0,
            y0,
            side,
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let slice = config_stability(&root)?;
        let census = ablation(occupant, &slice, raster, &[arm], threads()).remove(0);
        let pass = census.non_finite == 0 && census.dense == 0;
        Ok(crate::gate::MeasurementControl {
            description: format!(
                "{} on {}, {} {side}² crop at ({x0}, {y0}) of {}²: non-finite {}, dense {}, damaged {}, exhausted {}",
                c.description,
                occupant.name(),
                c.arm,
                c.raster,
                census.non_finite,
                census.dense,
                census.damaged,
                census.exhausted
            ),
            verdict: if pass {
                crate::gate::Verdict::Pass
            } else {
                crate::gate::Verdict::Fail
            },
            markdown: format!(
                "# {name}\n\n{}\n\n{}\nEnergy-error decades: {:?}\n",
                c.description,
                table(std::slice::from_ref(&census)),
                census.decades
            ),
        })
    }

    fn flags(&self, root: &Path, args: &[&str]) -> Result<(), String> {
        let n = match args {
            ["--res", n] => n.parse::<usize>().map_err(|e| format!("--res {n}: {e}"))?,
            _ => {
                return Err(format!(
                    "step-limit-ablation takes `--res <n>`, not `{}`",
                    args.join(" ")
                ))
            }
        };
        let slice = config_stability(root)?;
        let text = report(&slice, n, threads());
        let dir = crate::gate::out_dir(root).join(self.name());
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = dir.join(format!("ablation_{n}.md"));
        std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{text}\nreport: {}", path.display());
        Ok(())
    }
}
