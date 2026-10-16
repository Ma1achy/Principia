//! The `invariant-gradient` gate (TASK-M2-12; REQ-VAL-015): the invariant-chart gradient view, `E₀` on an `(L_z, E)`
//! chart, is a perfect axis-aligned gradient (render_contract § "Cross-check views (the seams)"; debug_tooling_plan §G),
//! checked on the decode stage's output: `E₀ = K₀ + V₀` of the `ICDescriptor` the kernel writes (R-133: the decode
//! stage's `E₀`, not `SimState.E_0`), summed at f32 as the view sums it.
//!
//! **The measure**, over a grid of `(s, t) = (i/(n − 1), j/(n − 1))`, the unit square's edges and corners included, at
//! each frozen configuration of the input, in f32 `eps` scaled as `decode-f32-eps` scales
//! ([`crate::decode_f32_eps::error`], `eps·max(|E|, 1)`):
//! - **the target**: each pixel's `E₀` against `E(t) = U + K(t)`, with `U` the frozen configuration's `V₀` from the
//!   same source at f64 and `K(t) = K_max·t^γ_K` at f64, from the chart's own f32 parameters widened;
//! - **constant along `L_z`**: each row's spread, its largest `E₀` less its smallest;
//! - **monotone along `E`**: each row's largest `E₀` less the next row's smallest, where positive.
//!
//! The input passes when all three are within the threshold, the f32 decode's own rounding: REQ-DEC-043's multiple of
//! f32 `eps` ("within f32 rounding of the target", REQ-VAL-015; applied per R-369, as R-133 reads "f32 noise" as that
//! factor), provisional with it until the M2 gate. Every GPU sample's `DEGENERATE` tag must equal the f64 decode's, and
//! the report counts the construction's infeasible pixels on the grid, at f64 and natively at f32, which the warp makes
//! none (REQ-CHART-017).
//!
//! **The control** (pitfalls §3): an input may replace the kernel's construction with a perturbed one,
//! [`Perturb::KMinNotSubtracted`], the mixing amplitude formed as `a = √(2K*)` rather than `√(2(K* − K_min))`, a solve
//! that forgets the rigid rotation's energy, decoded natively at f32. Its `E₀` is `U + K(t) + L_z²/2`, which varies
//! along `L_z`, and the gate must fail it.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed, on this machine's GPU backend; no trajectory.

use serde::Deserialize;

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{decode_only, DecodeStage};
use kernel::chart::invariant::{self, Invariant, InvariantBlocks, InvariantChart};
use kernel::chart::{ChartDecode, InvariantLE, InvariantLK};
use kernel::decode::invariant::Outcome;
use kernel::decode::{self as dec, descriptor};
use kernel::generated::links::Defaults;
use kernel::stage::decode::{
    DecodeMode, DecodeOut, DecodeVariant, InvariantLEDefaults, InvariantLKDefaults, UniformChart,
};

use crate::decode_f32_eps::{error, kernel_wgsl, physics_key, DecodeF32EpsGate};
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::gpu::GpuHarness;

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "invariant-gradient";

/// Which invariant chart an input decodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Which {
    /// `(L_z, E)`.
    LzE,
    /// `(L_z, K)`.
    LzK,
}

/// One frozen configuration: the configuration controls `(z_α, z_β)` and the mass controls `(z_μ₁, z_μ₂)`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frozen {
    pub z_cfg: [f32; 2],
    pub z_mass: [f32; 2],
}

/// A perturbed construction, in place of the kernel's (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perturb {
    /// `a = √(2K*)`: the construction asked for `K* + K_min`, so the rigid rotation's energy is not subtracted.
    KMinNotSubtracted,
}

/// An `invariant-gradient` input, `fixtures/gates/invariant-gradient/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The chart.
    pub chart: Which,
    /// The frozen configurations, each one chart.
    pub frozen: Vec<Frozen>,
    /// The grid's points per side, `n ≥ 2`.
    pub grid: u32,
    /// A perturbed construction in place of the kernel's, if any.
    #[serde(default)]
    pub perturb: Option<Perturb>,
}

/// The warp defaults, `(K_max, γ_K)`, from the constants register (REQ-CHART-044, proposed), at f32 as the kernel's
/// uniform holds them.
pub fn warp_defaults() -> (f32, f32) {
    (
        ledger::constants::K_MAX.number() as f32,
        ledger::constants::GAMMA_K.number() as f32,
    )
}

/// The chart parameters of `frozen` under the warp defaults.
pub fn invariant_of(frozen: &Frozen) -> Invariant<f32> {
    let (k_max, gamma_k) = warp_defaults();
    Invariant {
        z_cfg: frozen.z_cfg,
        z_mass: frozen.z_mass,
        k_max,
        gamma_k,
    }
}

/// `c` widened to f64, never narrowed: the f64 reference decodes exactly the parameters the kernel reads.
pub fn widen(c: &Invariant<f32>) -> Invariant<f64> {
    Invariant {
        z_cfg: c.z_cfg.map(f64::from),
        z_mass: c.z_mass.map(f64::from),
        k_max: f64::from(c.k_max),
        gamma_k: f64::from(c.gamma_k),
    }
}

/// The grid's `(s, t)`, row by row: `t` outer, increasing, then `s`.
pub fn grid(n: u32) -> Vec<[f32; 2]> {
    let step = |i: u32| i as f32 / (n - 1) as f32;
    (0..n)
        .flat_map(|j| (0..n).map(move |i| [step(i), step(j)]))
        .collect()
}

/// One frozen configuration's measured grid: each pixel's `E₀` at f32, the f64 target, and the tags compared.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// The grid's points per side.
    pub n: usize,
    /// Each pixel's `E₀ = K₀ + V₀`, summed at f32, row by row.
    pub e0: Vec<f32>,
    /// Each row's target `E(t) = U + K(t)`, at f64.
    pub target: Vec<f64>,
}

/// The worst of each measure over a set of fields, in f32 `eps` scaled by `max(|E|, 1)`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Measurement {
    /// The largest `|E₀ − E(t)|`.
    pub target: f64,
    /// The largest row spread.
    pub spread: f64,
    /// The largest decrease from one row to the next.
    pub monotone: f64,
    /// The pixels measured.
    pub samples: usize,
}

impl Measurement {
    /// The worst of the three.
    pub fn max(&self) -> f64 {
        self.target.max(self.spread).max(self.monotone)
    }
}

/// The three measures over `fields` (the module's note).
pub fn measure(fields: &[Field]) -> Measurement {
    let mut m = Measurement::default();
    for f in fields {
        let row = |j: usize| &f.e0[j * f.n..(j + 1) * f.n];
        let lo = |j: usize| row(j).iter().copied().fold(f32::INFINITY, f32::min);
        let hi = |j: usize| row(j).iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let scale = |x: f64| f64::from(f32::EPSILON) * x.abs().max(1.0);
        for j in 0..f.n {
            let target = f.target[j];
            for &e in row(j) {
                m.target = m.target.max(error(f64::from(e), target));
            }
            let spread = f64::from(hi(j)) - f64::from(lo(j));
            m.spread = m.spread.max(spread / scale(target));
            if j + 1 < f.n {
                let drop = f64::from(hi(j)) - f64::from(lo(j + 1));
                m.monotone = m.monotone.max(drop.max(0.0) / scale(target));
            }
        }
        m.samples += f.e0.len();
    }
    m
}

/// `E₀ = K₀ + V₀` of a decode, at f32, as the view sums it.
fn e0(out: &DecodeOut<f32>) -> f32 {
    descriptor::energy(&out.descriptor)
}

/// The f64 targets of the widened chart `wide` on the `n` rows: `U`, its frozen configuration's `V₀` at f64, plus
/// `K(t)` at f64.
fn targets<W>(wide: &W, n: u32) -> Result<Vec<f64>, String>
where
    W: InvariantChart<Param = f64> + ChartDecode,
{
    let u = decode_only::<_, Defaults>(wide, [0.5, 0.0], &physics_key())
        .map_err(|e| e.to_string())?
        .descriptor
        .V_0;
    Ok(grid(n)
        .iter()
        .step_by(n as usize)
        .map(|p| u + wide.invariant().targets(0.5, f64::from(p[1])).k)
        .collect())
}

/// What one frozen configuration gives: its field, the tags that differ between the GPU and f64, and the infeasible
/// pixels at f64 and natively at f32.
pub struct Measured {
    pub field: Field,
    pub tag_mismatches: usize,
    pub infeasible: [usize; 2],
}

/// The construction's infeasible pixels of `chart` and its widening `wide` on `points`, at f64 and natively at f32.
pub fn infeasible_counts<C, W>(chart: &C, wide: &W, points: &[[f32; 2]]) -> [usize; 2]
where
    C: InvariantChart<Param = f32>,
    W: InvariantChart<Param = f64>,
{
    let f64s = points
        .iter()
        .filter(|p| {
            invariant::decode::<Defaults, f64, _>(wide, f64::from(p[0]), f64::from(p[1])).1
                == Outcome::Infeasible
        })
        .count();
    let f32s = points
        .iter()
        .filter(|p| {
            invariant::decode::<Defaults, f32, _>(chart, p[0], p[1]).1 == Outcome::Infeasible
        })
        .count();
    [f64s, f32s]
}

/// The perturbed construction's decode of `chart` at `(s, t)`, natively at f32 ([`Perturb::KMinNotSubtracted`]).
fn perturbed<C: InvariantChart<Param = f32>>(chart: &C, s: f32, t: f32) -> DecodeOut<f32> {
    let targets = chart.invariant().targets(s, t);
    let mut blocks = InvariantBlocks::<Defaults, f32>::new(
        targets.lz,
        targets.k + targets.lz * targets.lz / 2.0,
    );
    let d = dec::decode_with(&mut blocks, &chart.map(s, t).z);
    DecodeOut {
        ic: d.ic,
        descriptor: d.descriptor,
        degenerate: d.degenerate,
    }
}

/// Measures one frozen configuration on the chart variant `V`, its widening `wide` the f64 reference, the decodes the
/// GPU's, or the perturbed construction's.
fn measure_one<V, W>(
    h: Option<(&GpuHarness, &DecodeStage<V>)>,
    chart: &V::Chart,
    wide: &W,
    n: u32,
) -> Result<Measured, String>
where
    V: DecodeVariant,
    V::Chart: InvariantChart<Param = f32> + UniformChart,
    W: InvariantChart<Param = f64> + ChartDecode,
{
    let points = grid(n);
    let outs: Vec<DecodeOut<f32>> = match h {
        Some((h, stage)) => stage
            .run(h.device(), h.queue(), chart, DecodeMode::Full, &points)
            .map_err(|e| e.to_string())?,
        None => points
            .iter()
            .map(|p| perturbed(chart, p[0], p[1]))
            .collect(),
    };
    let key = physics_key();
    let reference = points
        .iter()
        .map(|p| {
            decode_only::<_, Defaults>(wide, p.map(f64::from), &key)
                .map(|r| r.degenerate)
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, String>>()?;
    let decoded: Vec<_> = outs.iter().map(|o| o.degenerate).collect();
    let tag_mismatches = mismatches(&reference, &decoded);
    Ok(Measured {
        field: Field {
            n: n as usize,
            e0: outs.iter().map(e0).collect(),
            target: targets(wide, n)?,
        },
        tag_mismatches,
        infeasible: infeasible_counts(chart, wide, &points),
    })
}

/// The DEGENERATE tags of `decoded` that differ from `reference`'s, pixel by pixel.
pub fn mismatches<T: PartialEq>(reference: &[T], decoded: &[T]) -> usize {
    reference
        .iter()
        .zip(decoded)
        .filter(|(r, d)| r != d)
        .count()
}

/// The tag mismatches and the infeasible pixels, at f64 and at f32, over every frozen configuration of `measured`.
pub fn totals(measured: &[Measured]) -> (usize, [usize; 2]) {
    measured.iter().fold((0, [0, 0]), |(t, [a, b]), m| {
        (
            t + m.tag_mismatches,
            [a + m.infeasible[0], b + m.infeasible[1]],
        )
    })
}

/// The chart of an input at f32, as the kernel holds it, and at f64, its widening.
type Charts<C, W> = (fn(Invariant<f32>) -> C, fn(Invariant<f64>) -> W);

/// Measures every frozen configuration of `input` on the chart variant `V`, its f64 reference the chart `W`.
fn measure_all<V, W>(
    input: &Input,
    wgsl: &str,
    (chart, wide): Charts<V::Chart, W>,
) -> Result<(Vec<Measured>, String), String>
where
    V: DecodeVariant,
    V::Chart: InvariantChart<Param = f32> + UniformChart,
    W: InvariantChart<Param = f64> + ChartDecode,
{
    let charts = || {
        input.frozen.iter().map(|f| {
            let c = invariant_of(f);
            (chart(c), wide(widen(&c)))
        })
    };
    if input.perturb.is_some() {
        let out = charts()
            .map(|(c, w)| measure_one::<V, W>(None, &c, &w, input.grid))
            .collect::<Result<_, _>>()?;
        return Ok((
            out,
            "adapter: none, the perturbed construction natively at f32".to_owned(),
        ));
    }
    let h = GpuHarness::new().map_err(|e| format!("refused: {e}"))?;
    let stage =
        DecodeStage::<V>::new(h.device(), wgsl, FastMath::default()).map_err(|e| e.to_string())?;
    let out = charts()
        .map(|(c, w)| measure_one::<V, W>(Some((&h, &stage)), &c, &w, input.grid))
        .collect::<Result<_, _>>()?;
    Ok((out, h.adapter_info().to_string()))
}

/// The `invariant-gradient` gate.
pub struct InvariantGradientGate;

impl InvariantGradientGate {
    /// Measures `input`, the kernel's WGSL `wgsl`, and judges it against `config`'s threshold, or refuses it.
    pub fn judge(
        config: &GateConfig,
        name: &str,
        input: &Input,
        wgsl: &str,
    ) -> Result<GateReport, String> {
        if input.grid < 2 || input.frozen.is_empty() {
            return Err(format!(
                "refused: a grid of {} points per side over {} frozen configurations measures no gradient",
                input.grid,
                input.frozen.len()
            ));
        }
        let (measured, adapter) = match input.chart {
            Which::LzE => {
                measure_all::<InvariantLEDefaults, _>(input, wgsl, (InvariantLE, InvariantLE))?
            }
            Which::LzK => {
                measure_all::<InvariantLKDefaults, _>(input, wgsl, (InvariantLK, InvariantLK))?
            }
        };
        let fields: Vec<Field> = measured.iter().map(|m| m.field.clone()).collect();
        let m = measure(&fields);
        let (mismatches, [inf64, inf32]) = totals(&measured);
        let threshold = config.threshold.value;
        let pass = m.max() <= threshold && mismatches == 0;
        let (k_max, gamma_k) = warp_defaults();
        let mut scatter = vec![
            format!(
                "{adapter}; domain: the decode alone, its inputs held fixed, no trajectory (pitfalls §10)"
            ),
            format!(
                "chart {:?} at K_max = {k_max}, γ_K = {gamma_k} (REQ-CHART-044, proposed), {} frozen configurations, a \
                 {n}×{n} grid each, edges and corners included",
                input.chart,
                input.frozen.len(),
                n = input.grid
            ),
            format!(
                "E₀ against U + K(t): worst {:.2} eps; row spread along L_z: worst {:.2} eps; decrease from one row to \
                 the next along E: worst {:.2} eps; over {} pixels",
                m.target, m.spread, m.monotone, m.samples
            ),
        ];
        for (f, x) in input.frozen.iter().zip(&measured) {
            let one = measure(std::slice::from_ref(&x.field));
            scatter.push(format!(
                "z_cfg = {:?}, z_mass = {:?}: U = {:.6}; target {:.2}, spread {:.2}, monotone {:.2} eps",
                f.z_cfg,
                f.z_mass,
                x.field.target[0],
                one.target,
                one.spread,
                one.monotone
            ));
        }
        let mut negative = vec![
            if pass {
                format!(
                    "every measure within {threshold} eps; margin {:.1}× over the worst",
                    threshold / m.max().max(f64::MIN_POSITIVE)
                )
            } else {
                format!(
                    "the worst measure, {:.2} eps, or a tag mismatch ({mismatches}), fails at {threshold} eps",
                    m.max()
                )
            },
            format!("{mismatches} DEGENERATE tags differ between the decodes and the f64 decode"),
            format!(
                "the construction's infeasible pixels on the grids: {inf64} at f64, {inf32} natively at f32 \
                 (REQ-CHART-017: the warp makes none)"
            ),
        ];
        if let Some(p) = input.perturb {
            negative.push(format!(
                "the decodes are a perturbed construction, {p:?}, natively at f32, not the kernel's (pitfalls §3: a \
                 broken solve must fail the gate)"
            ));
        }
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(u32::try_from(m.samples).unwrap_or(u32::MAX)),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}

impl Gate for InvariantGradientGate {
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
            .map_err(|e| format!("refused: {name} is not an invariant-gradient input: {e}"))?;
        let wgsl = kernel_wgsl(&DecodeF32EpsGate::root())?;
        Self::judge(config, name, &input, &wgsl)
    }
}
