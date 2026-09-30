//! Convergence under refinement (philosophy §4.5a; pitfalls §3; REQ-VAL-004): an aggregate quantity computed at
//! successively finer sampling is quotable only if it converges. R-171 defines the gate:
//!
//! - samples are ordered coarse → fine: strides 32, 4, 1, 0, where stride 0 is unstrided (the finest);
//! - r_k = |x_k − x_{k−1}| / |x_{k−1}|;
//! - the gate passes iff r_k is strictly decreasing and the finest r_k is below the threshold.
//!
//! A sequence not ordered coarse → fine (strides strictly decreasing, 0 last) is refused, not judged. The pitfalls §3
//! record, 0.2153 → 0.4423 → 0.5494 → 0.0947 at strides 32, 4, 1, 0, gives r = 1.054, 0.242, 0.828 and fails.
//!
//! Scatter, for this gate, is the r_k sequence per region and the min–max spread of the final r_k across regions
//! (REQ-VAL-169, R-258).

use serde::Deserialize;

use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};

/// The fewest samples a sequence may hold: three give two relative steps, the fewest for which "r_k strictly
/// decreasing" can fail (pitfalls §3, "check the measurement can fire before reading it"; applied per R-204).
pub const MIN_SAMPLES: usize = 3;

/// Refuses `strides` unless it is ordered coarse → fine: strictly decreasing, ending at 0, the unstrided and finest
/// sample (R-171), with at least [`MIN_SAMPLES`] samples.
pub fn check_order(strides: &[u32]) -> Result<(), String> {
    if strides.len() < MIN_SAMPLES {
        return Err(format!(
            "refused: {} sample(s); the gate needs at least {MIN_SAMPLES}, so that r_k strictly decreasing can fail",
            strides.len()
        ));
    }
    if let Some(i) = (1..strides.len()).find(|&i| strides[i] >= strides[i - 1]) {
        return Err(format!(
            "refused: strides {strides:?} are not ordered coarse → fine: stride {} follows stride {} (R-171: strides \
             strictly decreasing, 0 last)",
            strides[i],
            strides[i - 1]
        ));
    }
    if strides.last() != Some(&0) {
        return Err(format!(
            "refused: strides {strides:?} do not end at stride 0, the unstrided and finest sample (R-171)"
        ));
    }
    Ok(())
}

/// r_k = |x_k − x_{k−1}| / |x_{k−1}| for k = 1 … n−1 (R-171). Refuses a non-finite value, and a zero x_{k−1}, for
/// which r_k is undefined.
pub fn relative_steps(values: &[f64]) -> Result<Vec<f64>, String> {
    if let Some(x) = values.iter().find(|x| !x.is_finite()) {
        return Err(format!("refused: the value {x} is not finite"));
    }
    values
        .windows(2)
        .map(|w| {
            if w[0] == 0.0 {
                Err(format!(
                    "refused: x = 0 before {}, so r = |x_k − x_(k−1)| / |x_(k−1)| is undefined",
                    w[1]
                ))
            } else {
                Ok((w[1] - w[0]).abs() / w[0].abs())
            }
        })
        .collect()
}

/// The first k at which r_k does not strictly decrease (`r[k] >= r[k-1]`), as an index into `r`; `None` when r is
/// strictly decreasing.
pub fn first_non_decrease(r: &[f64]) -> Option<usize> {
    (1..r.len()).find(|&k| r[k] >= r[k - 1])
}

/// R-171's pass rule: r strictly decreasing and the finest r below `threshold`.
pub fn passes(r: &[f64], threshold: f64) -> bool {
    first_non_decrease(r).is_none() && r.last().is_some_and(|&finest| finest < threshold)
}

/// The scatter of a set of regions (REQ-VAL-169): r_k per region, and the min–max of the final r_k across them.
#[derive(Debug, Clone, PartialEq)]
pub struct Scatter {
    /// Each region's label and its r_k sequence, in the order given.
    pub per_region: Vec<(String, Vec<f64>)>,
    /// The least final r_k across the regions.
    pub final_min: f64,
    /// The greatest final r_k across the regions.
    pub final_max: f64,
}

impl Scatter {
    /// The scatter of `per_region`; `None` when there is no region, or a region has no r_k.
    pub fn of(per_region: Vec<(String, Vec<f64>)>) -> Option<Self> {
        let finals: Vec<f64> = per_region
            .iter()
            .map(|(_, r)| r.last().copied())
            .collect::<Option<_>>()?;
        let final_min = finals.iter().copied().reduce(f64::min)?;
        let final_max = finals.iter().copied().reduce(f64::max)?;
        Some(Self {
            per_region,
            final_min,
            final_max,
        })
    }

    /// The report's scatter lines: one per region, then the min–max of the final r_k.
    pub fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .per_region
            .iter()
            .map(|(label, r)| format!("r_k, {label}: {}", fmt_list(r)))
            .collect();
        lines.push(format!(
            "final r_k across {} region(s): min {:.4}, max {:.4} (spread {:.4})",
            self.per_region.len(),
            self.final_min,
            self.final_max,
            self.final_max - self.final_min
        ));
        lines
    }
}

/// `values`, each to four decimal places, comma-separated.
fn fmt_list(values: &[f64]) -> String {
    values
        .iter()
        .map(|v| format!("{v:.4}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A convergence gate input, `fixtures/gates/convergence/<input>.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the sequence is, and where it was recorded.
    pub description: String,
    /// The strides the samples were taken at, in the order given.
    pub strides: Vec<u32>,
    /// One sequence per region, or one aggregate sequence when the region count is not recorded.
    pub series: Vec<Series>,
    /// The number of regions the sequences were drawn from: a count, or "not recorded" (R-258).
    pub region_count: RegionCountField,
}

/// One sequence of an [`Input`].
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Series {
    /// The region, or the aggregate, the sequence is of.
    pub label: String,
    /// The quantity at each stride of the input's `strides`, in the same order.
    pub values: Vec<f64>,
}

/// An input's region count as its JSON gives it: a number, or the string "not recorded".
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RegionCountField {
    /// The regions were counted.
    Count(u32),
    /// Any string; only "not recorded" is accepted.
    Text(String),
}

/// The convergence-under-refinement gate (R-171).
pub struct ConvergenceGate;

impl ConvergenceGate {
    /// Judges `input` against `config`'s threshold, or refuses it.
    pub fn judge(config: &GateConfig, name: &str, input: &Input) -> Result<GateReport, String> {
        check_order(&input.strides)?;
        if input.series.is_empty() {
            return Err("refused: the input holds no series".to_owned());
        }
        let count = match &input.region_count {
            RegionCountField::Count(n) if *n as usize == input.series.len() => {
                RegionCount::Recorded(*n)
            }
            RegionCountField::Count(n) => {
                return Err(format!(
                    "refused: region_count {n} is not the {} series given, one per region",
                    input.series.len()
                ))
            }
            RegionCountField::Text(t) if t == "not recorded" => RegionCount::NotRecorded {
                series: input.series.len(),
            },
            RegionCountField::Text(t) => {
                return Err(format!(
                    "refused: region_count `{t}` is neither a count nor \"not recorded\" (R-258)"
                ))
            }
        };
        let threshold = config.threshold.value;
        let mut per_region = Vec::new();
        let mut negative = Vec::new();
        let mut all_pass = true;
        for series in &input.series {
            if series.values.len() != input.strides.len() {
                return Err(format!(
                    "refused: series `{}` has {} value(s) for {} stride(s)",
                    series.label,
                    series.values.len(),
                    input.strides.len()
                ));
            }
            let r = relative_steps(&series.values)?;
            let finest = r[r.len() - 1];
            let (at, largest) =
                r.iter()
                    .copied()
                    .enumerate()
                    .fold(
                        (0, f64::MIN),
                        |best, (k, v)| if v > best.1 { (k, v) } else { best },
                    );
            negative.push(format!(
                "{}: largest relative step r = {largest:.4}, at stride {} → {}",
                series.label,
                input.strides[at],
                input.strides[at + 1]
            ));
            if let Some(k) = first_non_decrease(&r) {
                all_pass = false;
                negative.push(format!(
                    "{}: r not strictly decreasing: r = {:.4} at stride {} → {} is not below r = {:.4} before it",
                    series.label,
                    r[k],
                    input.strides[k],
                    input.strides[k + 1],
                    r[k - 1]
                ));
            }
            if finest >= threshold {
                all_pass = false;
                negative.push(format!(
                    "{}: finest r = {finest:.4} is not below the threshold {threshold}",
                    series.label
                ));
            }
            per_region.push((series.label.clone(), r));
        }
        let scatter = Scatter::of(per_region)
            .ok_or_else(|| "refused: a series has no relative step".to_owned())?;
        let verdict = if all_pass {
            Verdict::Pass
        } else {
            Verdict::Fail
        };
        Ok(GateReport {
            gate: "convergence".to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict,
            threshold: config.threshold.clone(),
            scatter: scatter.lines(),
            regions: Some(Regions {
                count,
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}

impl Gate for ConvergenceGate {
    fn name(&self) -> &'static str {
        "convergence"
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = Input::deserialize(fixture)
            .map_err(|e| format!("refused: {name} is not a convergence input: {e}"))?;
        Self::judge(config, name, &input)
    }
}
