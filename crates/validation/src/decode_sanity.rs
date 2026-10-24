//! The `decode-sanity` gate (TASK-M2-19; REQ-ENC-027, R-71): lookup's layer-3 residuals (inverse-encode § "Chart-aware
//! validation", layer 3), measured on the decode over every registered chart and over fuzzed latent `z`, at f64 and at
//! f32, against `k·ε_R`, the register's `decode_sanity` (`k`) times the precision's machine epsilon, provisional until
//! the human confirms it at the M2 gate.
//!
//! **The measure** is the lookup's own, `engine::lookup::residuals`: `‖Σmᵢrᵢ‖/R_x` (the CoM) and `‖Σpᵢ‖/P_x` (the total
//! momentum), `R_x` and `P_x` Part 4's extents floored at 1, on the decoded state widened to f64, so the f32 rows
//! measure the f32 decode's rounding. Each is reported in units of the precision's epsilon. A decoded mass that is not
//! positive fails the row too, layer 3's first constraint.
//!
//! **The samples**, from a fixed seed (SplitMix64, `crate::eps_phys`'s draws): `per_chart` points `(u, v)` uniform on
//! `[0, 1]²` on each registered chart, decoded by the chart's own decode, and `fuzz` latent `z` with every component
//! uniform on `[−span, span]`, decoded by the free decode; the f32 run reads the same draws rounded to f32.
//!
//! **The negative input** (`shift`): every decoded position moved by `shift` along `x`, so the CoM sits `shift` from the
//! origin and the gate must fail it.
//!
//! **The proposal** (REQ-ENC-027, R-71): `k` = the register's `decode_sanity`, above the gate's measured worst over
//! `fuzz.json` (the gate prints each row and its margin).

use serde::Deserialize;

use engine::lookup::residuals;
use kernel::decode::{self, Ic, Latent};
use kernel::generated::links::Defaults;
use ledger::constants::DECODE_SANITY;

use crate::eps_phys::{latents, points};
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::roundtrip::{registry, widen, DecodeAt, Precision};

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "decode-sanity";

/// A `decode-sanity` input, `fixtures/gates/decode-sanity/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The seed every draw comes from.
    pub seed: u64,
    /// The points `(u, v)` drawn per registered chart.
    pub per_chart: u32,
    /// The latent `z` drawn.
    pub fuzz: u32,
    /// The latent draws' half-width: every `zₖ` on `[−span, span]`.
    pub span: f64,
    /// The offset added to every decoded position's `x`, 0 for the measurement.
    pub shift: f64,
}

impl Input {
    /// The draws `crate::eps_phys` takes from the same fields.
    fn draws(&self) -> crate::eps_phys::Input {
        crate::eps_phys::Input {
            description: self.description.clone(),
            seed: self.seed,
            per_chart: self.per_chart,
            fuzz: self.fuzz,
            span: self.span,
            encode: crate::eps_phys::EncodeKind::Defaults,
        }
    }
}

/// One row: a source at a precision.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The registered chart's key, or `latent z (8-D)`.
    pub source: String,
    /// 64 or 32.
    pub bits: u32,
    /// The samples.
    pub samples: usize,
    /// The worst CoM residual, in the precision's epsilon; NaN read as `+∞`.
    pub com: f64,
    /// The worst `Σpᵢ` residual, in the precision's epsilon; NaN read as `+∞`.
    pub momentum: f64,
    /// The samples with a decoded mass that is not positive.
    pub nonpositive: usize,
}

/// `x` with every position moved by `shift` along `x`.
pub fn shifted(x: &Ic<f64>, shift: f64) -> Ic<f64> {
    Ic {
        r: x.r.map(|r| [r[0] + shift, r[1]]),
        ..*x
    }
}

/// `x`'s value, NaN read as `+∞`, so no comparison drops it.
fn loud(x: f64) -> f64 {
    if x.is_nan() {
        f64::INFINITY
    } else {
        x
    }
}

/// The row over the decoded states `xs` from `source` at `R`, each shifted by `shift`.
pub fn row<R: Precision>(source: &str, shift: f64, xs: impl Iterator<Item = Ic<R>>) -> Row {
    let mut r = Row {
        source: source.to_owned(),
        bits: 8 * std::mem::size_of::<R>() as u32,
        samples: 0,
        com: 0.0,
        momentum: 0.0,
        nonpositive: 0,
    };
    for x in xs {
        let x = shifted(&widen(&x), shift);
        let (com, momentum) = residuals(&x);
        r.samples += 1;
        r.com = r.com.max(loud(com / R::EPS));
        r.momentum = r.momentum.max(loud(momentum / R::EPS));
        r.nonpositive += usize::from(!x.m.iter().all(|&m| m > 0.0));
    }
    r
}

/// Every row: each registered chart at f64 and f32, in the registry's order, then the latent draws.
pub fn rows(input: &Input) -> Vec<Row> {
    fn chart_row<R: DecodeAt>(c: &dyn crate::roundtrip::Registered, input: &Input) -> Row {
        let xs = points(&input.draws())
            .into_iter()
            .map(|(u, v)| R::decode_at(c, u, v).ic);
        row(c.key(), input.shift, xs)
    }
    fn fuzz_row<R: Precision>(input: &Input) -> Row {
        let xs = latents(&input.draws())
            .into_iter()
            .map(|z| decode::decode::<Defaults, R>(&Latent::new(z.map(R::of))).ic);
        row("latent z (8-D)", input.shift, xs)
    }
    let mut out = Vec::new();
    for c in registry() {
        out.push(chart_row::<f64>(c.as_ref(), input));
        out.push(chart_row::<f32>(c.as_ref(), input));
    }
    out.push(fuzz_row::<f64>(input));
    out.push(fuzz_row::<f32>(input));
    out
}

/// Whether `r` fails against `k`: no samples, a mass not positive, or a worst residual past `k`.
pub fn fails(r: &Row, k: f64) -> bool {
    r.samples == 0 || r.nonpositive > 0 || r.com > k || r.momentum > k
}

/// The `decode-sanity` gate.
pub struct DecodeSanityGate;

impl DecodeSanityGate {
    /// Measures `input` and judges it against `config`'s threshold, which must be the register's `decode_sanity`, or
    /// refuses it.
    pub fn judge(config: &GateConfig, name: &str, input: &Input) -> Result<GateReport, String> {
        let k = config.threshold.value;
        let register = DECODE_SANITY.number();
        if k != register {
            return Err(format!(
                "refused: gate.json's threshold {k} is not the register's decode_sanity {register} (R-427)"
            ));
        }
        if input.per_chart == 0 || input.fuzz == 0 {
            return Err("refused: no samples".to_owned());
        }
        if !(input.span.is_finite() && input.span > 0.0) {
            return Err(format!("refused: the span {} is not positive", input.span));
        }
        let rows = rows(input);
        let failing: Vec<&Row> = rows.iter().filter(|r| fails(r, k)).collect();
        let verdict = if failing.is_empty() {
            Verdict::Pass
        } else {
            Verdict::Fail
        };
        let mut scatter = vec![format!(
            "‖Σmᵢrᵢ‖/R_x and ‖Σpᵢ‖/P_x of the decode, worst per row, in the precision's ε (f64 2⁻⁵², f32 2⁻²³), \
             against {k}·ε"
        )];
        for r in &rows {
            scatter.push(format!(
                "{} f{}: CoM {:.2}, Σp {:.2}; {} samples, {} with a mass not positive",
                r.source, r.bits, r.com, r.momentum, r.samples, r.nonpositive
            ));
        }
        let worst = rows
            .iter()
            .map(|r| r.com.max(r.momentum))
            .fold(0.0, f64::max);
        let samples: usize = rows.iter().map(|r| r.samples).sum();
        let negative = if failing.is_empty() {
            scatter.push(format!(
                "margin {:.1}× (k = {k} over the worst, {worst:.2})",
                k / worst
            ));
            vec![format!(
                "0 of {samples} decoded samples past {k}·ε or with a mass not positive"
            )]
        } else {
            let names: Vec<String> = failing
                .iter()
                .map(|r| {
                    format!(
                        "{} f{} (CoM {:.2}, Σp {:.2})",
                        r.source, r.bits, r.com, r.momentum
                    )
                })
                .collect();
            vec![format!("failing rows: {}", names.join("; "))]
        };
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict,
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(u32::try_from(samples).map_err(|e| e.to_string())?),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}

impl Gate for DecodeSanityGate {
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
            .map_err(|e| format!("refused: {name} is not a decode-sanity input: {e}"))?;
        Self::judge(config, name, &input)
    }
}
