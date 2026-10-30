//! The `decode-agreement` gate (TASK-M2-25, TASK-M2-29; REQ-COL-006, REQ-COL-060, R-133, R-297): the agreement preset
//! `|E(fragment-decode) − E₀|` (`render::presets::agreement`) over every M2 chart, the fragment's generated decode,
//! compiled as the backend compiles it, against the kernel decode stage's own, compiled with compute fast-math off and
//! read from the payload, within REQ-COL-060's factor.
//!
//! **The scene.** Per chart, a `side[0]` × `side[1]` lattice of one-pixel samples at depth `depth`'s cells
//! (`crate::decode_preset::Scene::lattice`), at the chart's fixed parameters (`crate::decode_preset::uniforms`). The
//! kernel decode stage decodes every sample's chart point on the GPU and its `ICDescriptor` is written into the
//! payload (`Scene::stage_into`), so `E₀ = K₀ + V₀` is the decode stage's (R-86, R-133), never `SimState.E_0`. The
//! agreement preset's words view then draws every sample: its ratio, `|E − E₀|` over the tolerance, and whether every
//! operand was finite.
//!
//! **The verdict.** A sample *agrees* when its ratio is at most 1: `|E − E₀| ≤ FACTOR·eps·(max(|K₀|, 1) + max(|V₀|, 1))`
//! (the preset's tolerance, per operand). The input passes when every valid sample agrees and the preset's hatch falls
//! exactly where an operand is non-finite: the payload's `K₀` or `V₀`, or the fragment's own `K` or `V`, which is the
//! decode stage's at the sample's own point (R-436). A hatch over finite operands, or a ratio over finite ones where an
//! operand is non-finite, fails it; so does a chart with no valid sample.
//!
//! **The negative inputs.** `shift` writes each sample's descriptor `shift` samples on (a dispatch scramble): every
//! sample compares its own decode with another's, so the disagreement is spatial, and the gate must fail it, reporting
//! how many samples of each chart disagree. `poison` stores `V₀ = −∞`'s bits at the samples it names, the collision
//! pole as the decode stage writes it: they must draw the hatch and drop out of the comparison, so that input passes.
//!
//! **The measurement** (REQ-COL-060, TASK-M2-29). Each chart's line in the report gives, over its compared samples, the
//! worst `|E − E₀|` in f32 `eps` of the tolerance's scale `max(|K₀|, 1) + max(|V₀|, 1)` ([`Sample::factor`]: the factor
//! that sample alone would need) and absolute, from the preset's own `E` and `E₀` words, and how many samples a
//! non-finite operand excluded (R-436); a last line gives the worst over every chart beside the tolerance. The `survey`
//! input is the measurement's denser lattice.
//!
//! **The threshold** is REQ-COL-060's factor, proposed by TASK-M2-29 and provisional until the human confirms it at the
//! M2 gate (R-71, R-182), the preset's own [`render::presets::agreement::FACTOR`]; a `gate.json` giving any other value
//! is refused, since the preset compiles its factor in.

use serde::Deserialize;

use render::presets::agreement::FACTOR;
use render::presets::{non_finite_bits, DecodeChart};

use crate::decode_f32_eps::{kernel_wgsl, DecodeF32EpsGate};
use crate::decode_preset::Scene;
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::gpu::GpuHarness;

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "decode-agreement";

/// A `decode-agreement` input, `fixtures/gates/decode-agreement/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The lattice's samples on each axis.
    pub side: [u32; 2],
    /// The depth of the cells the samples sit at.
    pub depth: u32,
    /// The dispatch scramble: sample `q`'s descriptor written at sample `(q + shift) mod count`; 0 is none.
    #[serde(default)]
    pub shift: u32,
    /// The samples whose stored `V₀` is set to `−∞`'s bits after the decode stage writes it.
    #[serde(default)]
    pub poison: Vec<u32>,
}

/// One sample as the preset drew it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// The preset's ratio, `|E − E₀|` over the tolerance.
    pub ratio: f32,
    /// Whether the preset found every operand finite.
    pub valid: bool,
    /// Whether every operand is finite by its bits: the payload's `K₀` and `V₀`, and the fragment decode's `K` and `V`.
    pub finite: bool,
    /// The measured difference `|E − E₀|`, at f64 from the preset's own `E` and `E₀` words: the fragment's f32 sums.
    pub diff: f64,
    /// The tolerance's scale at the sample, `eps·(max(|K₀|, 1) + max(|V₀|, 1))` at f64 from the stored `K₀` and `V₀`:
    /// the tolerance is the factor times it.
    pub scale: f64,
}

impl Sample {
    /// The sample from the preset's words `(ratio's bits, valid, E's bits, E₀'s bits)`, the payload's stored `K₀` and
    /// `V₀`, and whether its operands are finite.
    pub fn from_words(w: [u32; 4], stored: [f32; 2], finite: bool) -> Sample {
        let [k_0, v_0] = stored.map(|x| f64::from(x).abs().max(1.0));
        Sample {
            ratio: f32::from_bits(w[0]),
            valid: w[1] != 0,
            finite,
            diff: (f64::from(f32::from_bits(w[2])) - f64::from(f32::from_bits(w[3]))).abs(),
            scale: f64::from(f32::EPSILON) * (k_0 + v_0),
        }
    }

    /// The measured difference in units of the tolerance's scale: the factor this sample alone would need, NaN read as
    /// `+∞`.
    pub fn factor(&self) -> f64 {
        let f = self.diff / self.scale;
        if f.is_nan() {
            f64::INFINITY
        } else {
            f
        }
    }
}

/// One chart's tally.
#[derive(Clone, Debug, PartialEq)]
pub struct Tally {
    /// The chart.
    pub chart: DecodeChart,
    /// The samples drawn.
    pub samples: usize,
    /// The samples the preset compared (every operand finite).
    pub valid: usize,
    /// The valid samples whose ratio is over 1, or not a number.
    pub disagree: usize,
    /// The samples whose hatch does not match their operands: drawn invalid over finite operands, or compared over a
    /// non-finite one.
    pub misplaced: usize,
    /// The worst valid ratio, NaN read as `+∞`; 0 with no valid sample.
    pub worst: f64,
    /// The measurement (REQ-COL-060): the largest factor a valid sample needs, its `|E − E₀|` over the tolerance's
    /// scale ([`Sample::factor`]); 0 with no valid sample.
    pub worst_factor: f64,
    /// The largest valid `|E − E₀|`, NaN read as `+∞`; 0 with no valid sample.
    pub worst_diff: f64,
}

impl Tally {
    /// Counts `samples` of `chart`.
    pub fn of(chart: DecodeChart, samples: &[Sample]) -> Tally {
        let mut t = Tally {
            chart,
            samples: samples.len(),
            valid: 0,
            disagree: 0,
            misplaced: 0,
            worst: 0.0,
            worst_factor: 0.0,
            worst_diff: 0.0,
        };
        for s in samples {
            if s.valid != s.finite {
                t.misplaced += 1;
            }
            if !s.valid {
                continue;
            }
            t.valid += 1;
            let r = if s.ratio.is_nan() {
                f64::INFINITY
            } else {
                f64::from(s.ratio)
            };
            if r > 1.0 {
                t.disagree += 1;
            }
            t.worst = t.worst.max(r);
            t.worst_factor = t.worst_factor.max(s.factor());
            t.worst_diff = t.worst_diff.max(if s.diff.is_nan() {
                f64::INFINITY
            } else {
                s.diff
            });
        }
        t
    }

    /// Whether the chart passes: some sample compared, every compared sample agrees, and the hatch is where it belongs.
    pub fn pass(&self) -> bool {
        self.valid > 0 && self.disagree == 0 && self.misplaced == 0
    }
}

/// Refuses `input` and `config` unless they describe a run the preset can judge.
pub fn check(config: &GateConfig, input: &Input) -> Result<(), String> {
    if config.threshold.value != f64::from(FACTOR) {
        return Err(format!(
            "refused: the threshold {} is not the agreement preset's compiled factor {FACTOR} \
             (render::presets::agreement::FACTOR, REQ-COL-060)",
            config.threshold.value
        ));
    }
    let count = u64::from(input.side[0]) * u64::from(input.side[1]);
    if count == 0 {
        return Err(format!(
            "refused: the lattice {:?} has no sample",
            input.side
        ));
    }
    if input.depth > 30 || u64::from(input.side[0].max(input.side[1])) > (1u64 << input.depth) {
        return Err(format!(
            "refused: depth {}'s cells cannot hold a {:?} lattice of distinct cells (depth at most 30)",
            input.depth, input.side
        ));
    }
    if u64::from(input.shift) >= count {
        return Err(format!(
            "refused: the shift {} is not below the {count} samples",
            input.shift
        ));
    }
    if let Some(p) = input.poison.iter().find(|&&p| u64::from(p) >= count) {
        return Err(format!(
            "refused: the poisoned sample {p} is not one of the {count} samples"
        ));
    }
    Ok(())
}

/// Draws `chart`'s scene for `input` on `h`, the kernel's WGSL `wgsl`: each sample as the preset drew it.
pub fn measure(
    h: &GpuHarness,
    wgsl: &str,
    chart: DecodeChart,
    input: &Input,
) -> Result<Vec<Sample>, String> {
    let mut scene = Scene::lattice(chart, input.side, input.depth)?;
    let outs = scene.stage_into(h.device(), h.queue(), wgsl, input.shift)?;
    let grid = scene.context.grid;
    for &p in &input.poison {
        scene.set.ic(grid.sample_index(p, 0, 0)).V_0 = f32::NEG_INFINITY;
    }
    let words = scene.agreement(h.device(), h.queue())?;
    let finite = |k: f32, v: f32| !(non_finite_bits(k) || non_finite_bits(v));
    Ok((0u32..)
        .zip(&outs)
        .zip(&words)
        .map(|((q, own), w)| {
            let stored = scene.set.ic(grid.sample_index(q, 0, 0));
            let ok =
                finite(stored.K_0, stored.V_0) && finite(own.descriptor.K_0, own.descriptor.V_0);
            Sample::from_words(*w, [stored.K_0, stored.V_0], ok)
        })
        .collect())
}

/// The `decode-agreement` gate.
pub struct DecodeAgreementGate;

impl DecodeAgreementGate {
    /// Judges `tallies` of `input`, named `name`, against `config`.
    pub fn report(
        config: &GateConfig,
        name: &str,
        input: &Input,
        adapter: &str,
        tallies: &[Tally],
    ) -> GateReport {
        let pass = !tallies.is_empty() && tallies.iter().all(Tally::pass);
        let mut scatter = vec![format!(
            "{adapter}; the fragment's generated decode against the kernel decode stage's, both f32 on the GPU, \
             each sample at its quad's centre"
        )];
        let mut negative = Vec::new();
        for t in tallies {
            scatter.push(format!(
                "{}: {} samples, {} compared, {} excluded (a non-finite operand, R-436); measured worst |E − E₀| \
                 {:.3} eps of the scale (max(|K₀|, 1) + max(|V₀|, 1)), {:.3e} absolute; worst ratio {:.3e} of the \
                 tolerance",
                t.chart.section(),
                t.samples,
                t.valid,
                t.samples - t.valid,
                t.worst_factor,
                t.worst_diff,
                t.worst
            ));
            negative.push(format!(
                "{}: {} of {} compared samples disagree (ratio over 1); {} with the hatch misplaced; {} hatched",
                t.chart.section(),
                t.disagree,
                t.valid,
                t.misplaced,
                t.samples - t.valid
            ));
        }
        let measured = tallies.iter().map(|t| t.worst_factor).fold(0.0, f64::max);
        scatter.push(format!(
            "every chart: measured worst |E − E₀| {measured:.3} eps of the scale, against the tolerance's {} eps \
             (REQ-COL-060)",
            config.threshold.value
        ));
        if input.shift != 0 {
            negative.push(format!(
                "dispatch scramble: each descriptor written {} samples on, so each sample compares its own decode with \
                 another point's: the disagreement is spatial",
                input.shift
            ));
        }
        if !input.poison.is_empty() {
            negative.push(format!(
                "samples {:?} store V₀ = −∞'s bits: each must draw the hatch and drop out (R-436)",
                input.poison
            ));
        }
        let compared: usize = tallies.iter().map(|t| t.valid).sum();
        GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(u32::try_from(compared).unwrap_or(u32::MAX)),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        }
    }
}

impl Gate for DecodeAgreementGate {
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
            .map_err(|e| format!("refused: {name} is not a decode-agreement input: {e}"))?;
        check(config, &input)?;
        let wgsl = kernel_wgsl(&DecodeF32EpsGate::root())?;
        let h = GpuHarness::new().map_err(|e| format!("refused: {e}"))?;
        let mut tallies = Vec::new();
        for chart in DecodeChart::ALL {
            let samples = measure(&h, &wgsl, chart, &input)?;
            tallies.push(Tally::of(chart, &samples));
        }
        Ok(Self::report(
            config,
            name,
            &input,
            &h.adapter_info().to_string(),
            &tallies,
        ))
    }
}
