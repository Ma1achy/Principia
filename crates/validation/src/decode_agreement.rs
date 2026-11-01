//! The `decode-agreement` gate (TASK-M2-25, TASK-M2-29; REQ-COL-006, REQ-COL-060, R-133, R-297): the agreement preset
//! `|E(fragment-decode) − E₀|` (`render::presets::agreement`) over every M2 chart, the fragment's generated decode,
//! compiled as the backend compiles it, against the kernel decode stage's own, compiled with compute fast-math off and
//! read from the payload, within REQ-COL-060's factor.
//!
//! **The scene.** Per chart, a `side[0]` × `side[1]` lattice of one-pixel samples at depth `depth`'s cells
//! (`crate::decode_preset::Scene::lattice`), at the chart's fixed parameters (`crate::decode_preset::uniforms`). Or,
//! for an input with `windows`, per window and per depth of its `depths`, a `side[0]` × `side[1]` block of adjacent
//! depth-`d` cells centred on the window's chart point, on its one chart: a zoom into a collision, refined
//! ([`Run`]). The kernel decode stage decodes every sample's chart point on the GPU and its `ICDescriptor` is written
//! into the payload (`Scene::stage_into`), so `E₀ = K₀ + V₀` is the decode stage's (R-86, R-133), never
//! `SimState.E_0`. The agreement preset's words view then draws every sample: its ratio, `|E − E₀|` over the
//! tolerance, and whether every operand was finite.
//!
//! **The verdict.** A sample *agrees* when its ratio is at most 1: `|E − E₀| ≤ FACTOR·eps·S`, `S` colour_composition
//! §6's conditioning-aware scale, `max(|K₀|, 1) + max(|V₀|, 1)` plus the conditioning of the fragment decode's own
//! `(m, r)` (R-72; RQ-623), as the preset computes it. The input passes when every valid sample agrees and the preset's
//! hatch falls exactly where an operand is non-finite: the payload's `K₀` or `V₀`, or the fragment decode's own `K`,
//! `V`, `m` or `r`, the operands the preset tested, read back from the same decode by the operand view
//! (`render::presets::agreement::operands`; [`operands_finite`]), never the decode stage's in their place, which on a
//! fast-math backend need not round alike beside a pole (R-436). A hatch over finite operands, or a ratio
//! over finite ones where an operand is non-finite, fails it; so does a run with no valid sample. An input with
//! `"form": "per_operand"` is judged instead by the per-operand scale alone, `max(|K₀|, 1) + max(|V₀|, 1)`, formed from
//! the same draw's `E` and `E₀` words and the stored `K₀` and `V₀`: the form before the conditioning term, which the
//! `collision_per_operand` input expects to fail (the control that the term is needed). An input with
//! `"stored": "f64"` stores the f64 `decodeOnly()`'s `K₀` and `V₀` at each sample's point, rounded, in place of the
//! decode stage's ([`Stored::F64`]): the fragment's f32 `E` against the exact value, which every f32 evaluation misses
//! by about `κᵢⱼ·eps` beside a collision. On lavapipe the fragment and the decode stage compile alike and agree bit for
//! bit, so only the exact value shows the conditioning there; the `collision_f64` input and the control use it.
//!
//! **The negative inputs.** `shift` writes each sample's descriptor `shift` samples on (a dispatch scramble): every
//! sample compares its own decode with another's, so the disagreement is spatial, and the gate must fail it, reporting
//! how many samples of each chart disagree. An f64 store rewrites every sample's `K₀` and `V₀` at its own point, the
//! only descriptor fields the comparison reads, which would undo the scramble, so [`check`] refuses the two together.
//! `poison` stores `V₀ = −∞`'s bits at the samples it names, the collision pole as the decode stage writes it: they
//! must draw the hatch and drop out of the comparison, so that input passes.
//!
//! **The measurement** (REQ-COL-060, TASK-M2-29). Each run's line in the report gives, over its compared samples, the
//! worst factor in f32 `eps` of the scale as the shader computes it ([`Sample::factor`]: the preset's ratio times
//! [`FACTOR`], the factor that sample alone would need), its median and 99th percentile, the worst `|E − E₀|`
//! absolute, from the preset's own `E` and `E₀` words, the worst factor of the per-operand scale beside it, and how
//! many samples a non-finite operand excluded (R-436); a last line gives the worst over every run beside the
//! tolerance. The `survey` input is the measurement's denser lattice; the `collision` and `collision_f64` inputs'
//! windows zoom on `shape_sphere`'s three binary collisions on its equator, and a line per window gives its worst
//! factor at each depth, which stays flat under refinement where the per-operand form's grows (pitfalls §3).
//!
//! **The threshold** is REQ-COL-060's factor, proposed by TASK-M2-29 and provisional until the human confirms it at the
//! M2 gate (R-71, R-182), the preset's own [`render::presets::agreement::FACTOR`]; a `gate.json` giving any other value
//! is refused, since the preset compiles its factor in.

use serde::Deserialize;

use render::presets::agreement::{FACTOR, OPERAND_WORDS};
use render::presets::{non_finite_bits, DecodeChart};

use crate::decode_f32_eps::{kernel_wgsl, DecodeF32EpsGate};
use crate::decode_preset::{reference, Scene};
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
    /// The lattice's, or each window's, samples on each axis.
    pub side: [u32; 2],
    /// The depth of the lattice's cells; none for an input of `windows`.
    #[serde(default)]
    pub depth: Option<u32>,
    /// The zoom windows, each drawn at each of its depths; none for a lattice input.
    #[serde(default)]
    pub windows: Vec<Window>,
    /// The form the input is judged by.
    #[serde(default)]
    pub form: Form,
    /// What the payload's `K₀` and `V₀` hold.
    #[serde(default)]
    pub stored: Stored,
    /// The dispatch scramble: sample `q`'s descriptor written at sample `(q + shift) mod count`; 0 is none. Refused with
    /// [`Stored::F64`], which would undo it.
    #[serde(default)]
    pub shift: u32,
    /// The samples whose stored `V₀` is set to `−∞`'s bits after the decode stage writes it.
    #[serde(default)]
    pub poison: Vec<u32>,
}

/// A zoom window: a block of adjacent cells centred on `centre`, on one chart, at each of `depths`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    /// The chart, by its section name (`DecodeChart::section`).
    pub chart: String,
    /// The chart point the block is centred on, each coordinate on `[0, 1]`.
    pub centre: [f64; 2],
    /// The depths it is drawn at, each a refinement of the last.
    pub depths: Vec<u32>,
}

/// The form an input is judged by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    /// The preset's own ratio, over colour_composition §6's conditioning-aware scale.
    #[default]
    Conditioned,
    /// The per-operand scale alone, `max(|K₀|, 1) + max(|V₀|, 1)`, from the same draw: the form before the conditioning
    /// term, kept as the control that the term is needed.
    PerOperand,
}

/// What the payload's `K₀` and `V₀` hold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stored {
    /// The kernel decode stage's own, as it writes them (R-86, R-133): the agreement presets' comparison.
    #[default]
    Stage,
    /// The f64 `decodeOnly()`'s at the sample's own point, rounded to f32 (`crate::decode_preset::reference`): the
    /// exact values, which any f32 evaluation misses by about `κᵢⱼ·eps` beside a collision, on every backend, where the
    /// fragment and the decode stage may compile alike and agree bit for bit (lavapipe does).
    F64,
}

/// One drawing of the gate: a chart's lattice, or a window at one depth.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    /// The chart.
    pub chart: DecodeChart,
    /// The depth of the cells.
    pub depth: u32,
    /// For a window, its index in the input's `windows`, its centre and its block's first cell; none for a lattice.
    pub window: Option<(usize, [f64; 2], [u64; 2])>,
}

impl Run {
    /// The run's name in the report: the chart's section, and a window's centre and depth.
    pub fn label(&self) -> String {
        match self.window {
            None => self.chart.section().to_owned(),
            Some((_, c, _)) => format!(
                "{} at ({}, {}) depth {}",
                self.chart.section(),
                c[0],
                c[1],
                self.depth
            ),
        }
    }

    /// The run's scene of `side` samples.
    pub fn scene(&self, side: [u32; 2]) -> Result<Scene, String> {
        match self.window {
            None => Scene::lattice(self.chart, side, self.depth),
            Some((_, _, o)) => {
                let cells: Vec<[u64; 2]> = (0..side[1])
                    .flat_map(|j| {
                        (0..side[0]).map(move |i| [o[0] + u64::from(i), o[1] + u64::from(j)])
                    })
                    .collect();
                Scene::new(self.chart, side, 1, self.depth, &cells)
            }
        }
    }
}

/// The chart whose section is `name`.
pub fn chart_named(name: &str) -> Option<DecodeChart> {
    DecodeChart::ALL.into_iter().find(|c| c.section() == name)
}

/// The deepest depth a run may draw: depth 30's cells still have distinct f32 centres on most of `[0, 1]`.
const MAX_DEPTH: u32 = 30;

/// The first cell of a block of `side` adjacent cells, of the `n` on an axis, centred on `centre`:
/// `⌊centre·n⌋ − ⌊side/2⌋`; none where the block runs off the axis.
fn block_start(centre: f64, side: u32, n: u64) -> Option<u64> {
    let first = (centre * n as f64).floor() as i64 - i64::from(side / 2);
    let first = u64::try_from(first).ok()?;
    (first + u64::from(side) <= n).then_some(first)
}

/// `input`'s runs, in order: every chart's lattice, or each window at each of its depths; `Err` names what cannot be
/// drawn.
pub fn runs(input: &Input) -> Result<Vec<Run>, String> {
    match (input.depth, input.windows.is_empty()) {
        (Some(depth), true) => {
            if depth > MAX_DEPTH || u64::from(input.side[0].max(input.side[1])) > (1u64 << depth) {
                return Err(format!(
                    "refused: depth {depth}'s cells cannot hold a {:?} lattice of distinct cells (depth at most 30)",
                    input.side
                ));
            }
            Ok(DecodeChart::ALL
                .into_iter()
                .map(|chart| Run {
                    chart,
                    depth,
                    window: None,
                })
                .collect())
        }
        (None, false) => {
            let mut out = Vec::new();
            for (k, w) in input.windows.iter().enumerate() {
                let chart = chart_named(&w.chart).ok_or_else(|| {
                    format!(
                        "refused: window {k}'s chart {:?} is not an M2 chart",
                        w.chart
                    )
                })?;
                if w.depths.is_empty() {
                    return Err(format!("refused: window {k} has no depth"));
                }
                if !w.centre.iter().all(|c| (0.0..=1.0).contains(c)) {
                    return Err(format!(
                        "refused: window {k}'s centre {:?} is not on [0, 1]²",
                        w.centre
                    ));
                }
                for &depth in &w.depths {
                    if depth > MAX_DEPTH {
                        return Err(format!("refused: window {k}'s depth {depth} is past 30"));
                    }
                    let n = 1u64 << depth;
                    let first = |a: usize| {
                        block_start(w.centre[a], input.side[a], n).ok_or_else(|| {
                            format!(
                                "refused: window {k} at depth {depth} runs off the chart: {} cells about cell {} of \
                                 {n}",
                                input.side[a],
                                (w.centre[a] * n as f64).floor()
                            )
                        })
                    };
                    let origin = [first(0)?, first(1)?];
                    out.push(Run {
                        chart,
                        depth,
                        window: Some((k, w.centre, origin)),
                    });
                }
            }
            Ok(out)
        }
        _ => Err("refused: an input gives a lattice's depth or windows, one of the two".to_owned()),
    }
}

/// One sample as the preset drew it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// The preset's ratio, `|E − E₀|` over the tolerance.
    pub ratio: f32,
    /// Whether the preset found every operand finite.
    pub valid: bool,
    /// Whether every operand is finite by its bits: the payload's `K₀` and `V₀`, and the fragment decode's `K`, `V`, `m`
    /// and `r` ([`operands_finite`]).
    pub finite: bool,
    /// The measured difference `|E − E₀|`, at f64 from the preset's own `E` and `E₀` words: the fragment's f32 sums.
    pub diff: f64,
    /// The per-operand scale at the sample, `eps·(max(|K₀|, 1) + max(|V₀|, 1))` at f64 from the stored `K₀` and `V₀`:
    /// the scale before colour_composition §6's conditioning term, which the `per_operand` form judges by.
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

    /// The measured difference in `eps` of the conditioning-aware scale as the shader computes it, the preset's ratio
    /// times [`FACTOR`]: the factor this sample alone would need, NaN read as `+∞`.
    pub fn factor(&self) -> f64 {
        let f = f64::from(self.ratio) * f64::from(FACTOR);
        if f.is_nan() {
            f64::INFINITY
        } else {
            f
        }
    }

    /// The measured difference in `eps` of the per-operand scale ([`Sample::scale`]): the factor the form before the
    /// conditioning term would need, NaN read as `+∞`.
    pub fn per_operand_factor(&self) -> f64 {
        let f = self.diff / self.scale;
        if f.is_nan() {
            f64::INFINITY
        } else {
            f
        }
    }

    /// The sample's ratio under `form`: the preset's own, or the per-operand factor over [`FACTOR`]; NaN read as `+∞`.
    pub fn ratio_in(&self, form: Form) -> f64 {
        let r = match form {
            Form::Conditioned => f64::from(self.ratio),
            Form::PerOperand => self.per_operand_factor() / f64::from(FACTOR),
        };
        if r.is_nan() {
            f64::INFINITY
        } else {
            r
        }
    }
}

/// One run's tally.
#[derive(Clone, Debug, PartialEq)]
pub struct Tally {
    /// The chart.
    pub chart: DecodeChart,
    /// The run's name in the report ([`Run::label`]); the chart's section for a lattice.
    pub label: String,
    /// The window's index and the depth, for a window's run.
    pub window: Option<(usize, u32)>,
    /// The samples drawn.
    pub samples: usize,
    /// The samples the preset compared (every operand finite).
    pub valid: usize,
    /// The valid samples whose ratio, in the form judged by, is over 1, or not a number.
    pub disagree: usize,
    /// The samples whose hatch does not match their operands: drawn invalid over finite operands, or compared over a
    /// non-finite one.
    pub misplaced: usize,
    /// The worst valid ratio in the form judged by, NaN read as `+∞`; 0 with no valid sample.
    pub worst: f64,
    /// The measurement (REQ-COL-060): the largest factor a valid sample needs, its `|E − E₀|` over the conditioning-aware
    /// scale as the shader computes it ([`Sample::factor`]); 0 with no valid sample.
    pub worst_factor: f64,
    /// The largest factor a valid sample needs in the per-operand scale ([`Sample::per_operand_factor`]); 0 with no
    /// valid sample.
    pub worst_per_operand: f64,
    /// The largest valid `|E − E₀|`, NaN read as `+∞`; 0 with no valid sample.
    pub worst_diff: f64,
    /// The compared samples' median factor, by [`quantile`]: the distribution the tolerance is set from (pitfalls §3,
    /// "Set thresholds from the observed distribution").
    pub median_factor: f64,
    /// The compared samples' 99th-percentile factor, by [`quantile`].
    pub p99_factor: f64,
}

/// The nearest-rank `q`-quantile of `sorted` (ascending), `0 < q ≤ 1`: its `⌈q·n⌉`-th value; 0 when it is empty.
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    let rank = (q * n as f64).ceil() as usize;
    sorted[rank.clamp(1, n) - 1]
}

impl Tally {
    /// Counts `samples` of `chart`, judged by the preset's own ratio.
    pub fn of(chart: DecodeChart, samples: &[Sample]) -> Tally {
        Tally::judged(chart, samples, Form::Conditioned)
    }

    /// Counts `samples` of `run`, judged by `form`.
    pub fn of_run(run: &Run, samples: &[Sample], form: Form) -> Tally {
        Tally {
            label: run.label(),
            window: run.window.map(|(k, _, _)| (k, run.depth)),
            ..Tally::judged(run.chart, samples, form)
        }
    }

    /// Counts `samples` of `chart`, judged by `form`.
    pub fn judged(chart: DecodeChart, samples: &[Sample], form: Form) -> Tally {
        let mut t = Tally {
            chart,
            label: chart.section().to_owned(),
            window: None,
            samples: samples.len(),
            valid: 0,
            disagree: 0,
            misplaced: 0,
            worst: 0.0,
            worst_factor: 0.0,
            worst_per_operand: 0.0,
            worst_diff: 0.0,
            median_factor: 0.0,
            p99_factor: 0.0,
        };
        let mut factors = Vec::new();
        for s in samples {
            if s.valid != s.finite {
                t.misplaced += 1;
            }
            if !s.valid {
                continue;
            }
            t.valid += 1;
            let r = s.ratio_in(form);
            if r > 1.0 {
                t.disagree += 1;
            }
            t.worst = t.worst.max(r);
            t.worst_factor = t.worst_factor.max(s.factor());
            t.worst_per_operand = t.worst_per_operand.max(s.per_operand_factor());
            factors.push(s.factor());
            t.worst_diff = t.worst_diff.max(if s.diff.is_nan() {
                f64::INFINITY
            } else {
                s.diff
            });
        }
        factors.sort_by(f64::total_cmp);
        t.median_factor = quantile(&factors, 0.5);
        t.p99_factor = quantile(&factors, 0.99);
        t
    }

    /// Whether the run passes: some sample compared, every compared sample agrees, and the hatch is where it belongs.
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
    runs(input)?;
    if u64::from(input.shift) >= count {
        return Err(format!(
            "refused: the shift {} is not below the {count} samples",
            input.shift
        ));
    }
    if input.stored == Stored::F64 && input.shift != 0 {
        return Err(format!(
            "refused: the shift {} with the f64 store tests nothing: the store rewrites every sample's K₀ and V₀ at its \
             own point, the only descriptor fields the comparison reads, undoing the scramble",
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

/// Writes every sample's payload `K₀` and `V₀` as the f64 `decodeOnly()`'s at its own point, rounded to f32
/// ([`Stored::F64`]).
pub fn store_exact(scene: &mut Scene) -> Result<(), String> {
    let grid = scene.context.grid;
    for q in 0..grid.quad_count() {
        let exact = reference(scene.chart, &scene.uniforms, scene.point(q))?
            .out
            .descriptor;
        let d = scene.set.ic(grid.sample_index(q, 0, 0));
        d.K_0 = exact.K_0 as f32;
        d.V_0 = exact.V_0 as f32;
    }
    Ok(())
}

/// Whether every operand the preset tests is finite by its bits: the payload's stored `K₀` and `V₀`, and the fragment
/// decode's own `K`, `V`, `m` and `r`, as the operand view writes them (`render::presets::agreement::operands`).
pub fn operands_finite(stored: [f32; 2], fragment: &[u32; OPERAND_WORDS]) -> bool {
    !stored.into_iter().any(non_finite_bits)
        && !fragment.iter().any(|w| non_finite_bits(f32::from_bits(*w)))
}

/// Draws `run`'s scene for `input` on `h`, the kernel's WGSL `wgsl`: each sample as the preset drew it.
pub fn measure(
    h: &GpuHarness,
    wgsl: &str,
    run: &Run,
    input: &Input,
) -> Result<Vec<Sample>, String> {
    let mut scene = run.scene(input.side)?;
    scene.stage_into(h.device(), h.queue(), wgsl, input.shift)?;
    let grid = scene.context.grid;
    if input.stored == Stored::F64 {
        store_exact(&mut scene)?;
    }
    for &p in &input.poison {
        scene.set.ic(grid.sample_index(p, 0, 0)).V_0 = f32::NEG_INFINITY;
    }
    let words = scene.agreement(h.device(), h.queue())?;
    let operands = scene.agreement_operands(h.device(), h.queue())?;
    Ok((0u32..)
        .zip(&words)
        .zip(&operands)
        .map(|((q, w), own)| {
            let stored = scene.set.ic(grid.sample_index(q, 0, 0));
            let stored = [stored.K_0, stored.V_0];
            Sample::from_words(*w, stored, operands_finite(stored, own))
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
                 {:.3} eps of the conditioned scale, {:.3e} absolute, median {:.3} eps, 99th percentile {:.3} eps; \
                 the per-operand scale's worst {:.3} eps; worst ratio {:.3e} of the tolerance",
                t.label,
                t.samples,
                t.valid,
                t.samples - t.valid,
                t.worst_factor,
                t.worst_diff,
                t.median_factor,
                t.p99_factor,
                t.worst_per_operand,
                t.worst
            ));
            negative.push(format!(
                "{}: {} of {} compared samples disagree (ratio over 1); {} with the hatch misplaced; {} hatched",
                t.label,
                t.disagree,
                t.valid,
                t.misplaced,
                t.samples - t.valid
            ));
        }
        for (k, w) in input.windows.iter().enumerate() {
            let by_depth: Vec<String> = tallies
                .iter()
                .filter_map(|t| match t.window {
                    Some((i, d)) if i == k => Some(format!(
                        "{d}: {:.3} ({:.3})",
                        t.worst_factor, t.worst_per_operand
                    )),
                    _ => None,
                })
                .collect();
            scatter.push(format!(
                "window {k}, {} at ({}, {}), under refinement (pitfalls §3): worst eps of the conditioned scale \
                 (of the per-operand scale) by depth: {}",
                w.chart,
                w.centre[0],
                w.centre[1],
                by_depth.join(", ")
            ));
        }
        let measured = tallies.iter().map(|t| t.worst_factor).fold(0.0, f64::max);
        scatter.push(format!(
            "every run: measured worst |E − E₀| {measured:.3} eps of the conditioned scale, against the tolerance's {} \
             eps (REQ-COL-060)",
            config.threshold.value
        ));
        if input.stored == Stored::F64 {
            negative.push(
                "the payload's K₀ and V₀ are the f64 decodeOnly()'s at each sample's point, rounded to f32: the \
                 fragment's f32 E against the exact value"
                    .to_owned(),
            );
        }
        if input.form == Form::PerOperand {
            negative.push(
                "judged by the per-operand scale alone, max(|K₀|, 1) + max(|V₀|, 1), the form before the conditioning \
                 term (colour_composition §6; RQ-623): beside a collision it must disagree"
                    .to_owned(),
            );
        }
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
        for run in runs(&input)? {
            let samples = measure(&h, &wgsl, &run, &input)?;
            tallies.push(Tally::of_run(&run, &samples, input.form));
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
