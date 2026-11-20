//! The `encode-eps-phys` gate (TASK-M2-16; REQ-ENC-024, REQ-ENC-004): the T2 round trip's residual in physical units,
//! `‖D(z) − D(E(D(z)))‖_phys` (inverse-encode Part 4; dd_encode §3.5), measured over interior samples of every
//! registered chart and of the 8-D latent interior, at f64 and at f32, against `ε_phys = k·ε_R`, the register's
//! `eps_phys` (`k`) times the precision's machine epsilon, provisional until the human confirms it at the M2 gate (R-71).
//!
//! **The measure** ([`crate::roundtrip`]). At each sample, the chart's own decode `D(·)` at the precision, `E` on it
//! under the defaults links, the free decode of its `z`, and the residual `‖·‖_phys` of Part 4: the largest of
//! `|Δmᵢ|`, `‖Δrᵢ‖/R_x` and `‖Δpᵢ‖/P_x`, `R_x` and `P_x` the decoded state's extent in positions and momenta, floored
//! at 1. It is reported in units of the precision's epsilon, so the two precisions read on one scale. On the shape
//! sphere chart's rows the residual is taken against `C(x)`, `D(E(x))` against `x`'s canonical state, as T3's: its
//! decode at a lower-hemisphere pixel is off `C`'s section (R-544; inverse-encode Part 1's T2;
//! [`crate::roundtrip::Against`]). Every other row is against `x`.
//!
//! **Interior** (Part 1, T2: "away from clamps"). A sample is interior when each of its eight controls lies at least
//! twice its clamp's `ε` inside the clamp, by the oracle [`crate::roundtrip::margins`], which reads Part 3's formulas
//! off the decoded state, not the encode's code. Only interior samples enter the worst residual. Every sample is
//! checked against its clamp flags: a control past its clamp (half its `ε` or more beyond) must set its
//! `lookup_clamped` bit and one inside must leave it clear (Part 8: clamp-adjacent samples assert the flag); a sample
//! that does neither is a mismatch, and a mismatch fails the gate.
//!
//! **The samples**, from a fixed seed (SplitMix64): `per_chart` points `(u, v)` uniform on `[0, 1]²` for each
//! registered chart (`crate::roundtrip::registry`), and `fuzz` latent `z` with every component uniform on
//! `[−span, span]`, decoded by the free decode, at each precision; the f32 run reads the same draws rounded to f32.
//!
//! **The negative input** (`encode: "mismatched"`): `E` under [`Mismatched`], every link of the defaults swapped on
//! the encode side, decoded under the defaults (dd_encode §4, seam 2/13: encode's inverses must be the decode's links),
//! so the gate must fail it (PIT-3).
//!
//! **The proposal** (REQ-ENC-024, R-71): `k` = the register's `eps_phys`, with the gate's measured worst over
//! `interior.json` as its evidence (the gate prints each row, its margin, and its sample counts).

use serde::Deserialize;

use kernel::decode::{self, Ic, Latent};
use kernel::generated::links::{
    Defaults, Select, SoftsignBeta, SoftsignQ, StickBreaking, TanhAlpha, TanhQ,
};

use crate::decode_f32_eps::SplitMix;
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::gauge::uniform;
use crate::oklab::quantile;
use crate::roundtrip::{
    margins, registry, trip_against, widen, Against, DecodeAt, Precision, Registered, Sides, Trip,
};
use ledger::constants::EPS_PHYS;

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "encode-eps-phys";

/// Every link of the defaults swapped, `stick_breaking`, `tanh_alpha`, `softsign_beta` and the `tanh_q` and
/// `softsign_q` momenta: the negative input's encode.
pub type Mismatched =
    Select<StickBreaking, TanhAlpha, SoftsignBeta, TanhQ, SoftsignQ, TanhQ, SoftsignQ>;

/// Which encode the round trip runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncodeKind {
    /// `E` under the defaults, the decode's own links.
    Defaults,
    /// `E` under [`Mismatched`], which must fail.
    Mismatched,
}

/// An `encode-eps-phys` input, `fixtures/gates/encode-eps-phys/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The seed every draw comes from.
    pub seed: u64,
    /// The points `(u, v)` drawn per registered chart.
    pub per_chart: u32,
    /// The latent `z` drawn for the 8-D interior.
    pub fuzz: u32,
    /// The 8-D draws' half-width: every `zₖ` on `[−span, span]`.
    pub span: f64,
    /// The encode the round trip runs.
    pub encode: EncodeKind,
}

/// One row of the measurement: a source at a precision.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// The registered chart's key, or `latent z (8-D)`.
    pub source: String,
    /// 64 or 32.
    pub bits: u32,
    /// The interior samples, whose residuals the row's statistics are over.
    pub interior: usize,
    /// The samples with a control past its clamp, each with its clamp bits set.
    pub clamped: usize,
    /// The samples with a control in the band between, neither held to `ε_phys` nor to a flag.
    pub band: usize,
    /// The samples whose clamp bits disagree with the oracle.
    pub mismatches: usize,
    /// The worst interior residual, in the precision's epsilon, and the sample's index.
    pub worst: (f64, usize),
    /// The interior residuals' median and 99th percentile, in the precision's epsilon.
    pub median: f64,
    /// See `median`.
    pub p99: f64,
}

/// The round trip at `x` under the input's encode, its residual against `against`'s state.
fn round<R: Precision>(kind: EncodeKind, x: &Ic<R>, against: Against) -> Trip<R> {
    match kind {
        EncodeKind::Defaults => trip_against::<Defaults, Defaults, R>(x, against),
        EncodeKind::Mismatched => trip_against::<Mismatched, Defaults, R>(x, against),
    }
}

/// Whether the clamp bits `bits` agree with the oracle's `sides`: every control past its clamp sets its bit, and every
/// control inside leaves it clear.
pub fn agrees(sides: Sides, bits: u32) -> bool {
    (sides.past & !bits) == 0 && (sides.inside & bits) == 0
}

/// The row over the decoded states `xs`, from `source` at `R`, each residual against `against`'s state.
fn row<R: Precision>(
    source: &str,
    kind: EncodeKind,
    against: Against,
    xs: impl Iterator<Item = Ic<R>>,
) -> Row {
    let (mut clamped, mut band, mut mismatches) = (0, 0, 0);
    let mut residuals = Vec::new();
    let mut worst = (0.0f64, 0usize);
    for (i, x) in xs.enumerate() {
        let t = round(kind, &x, against);
        let sides = Sides::of(margins(&widen(&x)));
        let bits = t.encoded.report.clamped_controls;
        mismatches += usize::from(!agrees(sides, bits));
        if sides.interior() {
            let e = t.residual / R::EPS;
            if e > worst.0 || e.is_nan() {
                worst = (if e.is_nan() { f64::INFINITY } else { e }, i);
            }
            residuals.push(e);
        } else if sides.past != 0 {
            clamped += 1;
        } else {
            band += 1;
        }
    }
    let q = |p| quantile(&residuals, p).unwrap_or(f64::NAN);
    Row {
        source: source.to_owned(),
        bits: 8 * std::mem::size_of::<R>() as u32,
        interior: residuals.len(),
        clamped,
        band,
        mismatches,
        worst,
        median: q(0.5),
        p99: q(0.99),
    }
}

/// The draws `(u, v)` for `chart`: `per_chart` points uniform on `[0, 1]²` from the input's seed.
pub fn points(input: &Input) -> Vec<(f64, f64)> {
    let mut rng = SplitMix::new(input.seed);
    (0..input.per_chart)
        .map(|_| (uniform(&mut rng, 0.0, 1.0), uniform(&mut rng, 0.0, 1.0)))
        .collect()
}

/// The 8-D draws: `fuzz` latent `z`, every component uniform on `[−span, span]`, from the input's seed plus 1.
pub fn latents(input: &Input) -> Vec<[f64; 8]> {
    let mut rng = SplitMix::new(input.seed.wrapping_add(1));
    (0..input.fuzz)
        .map(|_| std::array::from_fn(|_| uniform(&mut rng, -input.span, input.span)))
        .collect()
}

/// `chart`'s row at `R`, against the chart's own reference ([`Registered::against`]).
pub fn chart_row<R: DecodeAt>(chart: &dyn Registered, input: &Input) -> Row {
    let xs = points(input)
        .into_iter()
        .map(|(u, v)| R::decode_at(chart, u, v).ic);
    row(chart.key(), input.encode, chart.against(), xs)
}

/// The 8-D interior's row at `R`.
pub fn fuzz_row<R: Precision>(input: &Input) -> Row {
    let xs = latents(input)
        .into_iter()
        .map(|z| decode::decode::<Defaults, R>(&Latent::new(z.map(R::of))).ic);
    row("latent z (8-D)", input.encode, Against::Decoded, xs)
}

/// Every row: each registered chart at f64 and f32, in the registry's order, then the 8-D interior.
pub fn rows(input: &Input) -> Vec<Row> {
    let mut out = Vec::new();
    for chart in registry() {
        out.push(chart_row::<f64>(chart.as_ref(), input));
        out.push(chart_row::<f32>(chart.as_ref(), input));
    }
    if input.fuzz > 0 {
        out.push(fuzz_row::<f64>(input));
        out.push(fuzz_row::<f32>(input));
    }
    out
}

/// Whether the row `r` fails against `k`: it has no interior sample, or a mismatch, or its worst is past `k` or NaN.
pub fn fails(r: &Row, k: f64) -> bool {
    r.interior == 0 || r.mismatches > 0 || r.worst.0.is_nan() || r.worst.0 > k
}

/// The verdict on `rows` against `k`: a pass when no row [`fails`].
pub fn verdict(rows: &[Row], k: f64) -> Verdict {
    if rows.iter().any(|r| fails(r, k)) {
        Verdict::Fail
    } else {
        Verdict::Pass
    }
}

/// The row with the largest worst, the first of equals; `None` when there are no rows.
pub fn worst_row(rows: &[Row]) -> Option<&Row> {
    rows.iter()
        .reduce(|a, r| if r.worst.0 > a.worst.0 { r } else { a })
}

/// The samples `rows` count, interior, clamped and in the band.
pub fn samples(rows: &[Row]) -> usize {
    rows.iter().map(|r| r.interior + r.clamped + r.band).sum()
}

/// The `encode-eps-phys` gate.
pub struct EpsPhysGate;

impl EpsPhysGate {
    /// Measures `input` and judges it against `config`'s threshold, which must be the register's `eps_phys`, or refuses
    /// it.
    pub fn judge(config: &GateConfig, name: &str, input: &Input) -> Result<GateReport, String> {
        let k = config.threshold.value;
        let register = EPS_PHYS.number();
        if k != register {
            return Err(format!(
                "refused: gate.json's threshold {k} is not the register's eps_phys {register} (R-427)"
            ));
        }
        if input.per_chart == 0 {
            return Err("refused: no samples per chart".to_owned());
        }
        if !(input.span.is_finite() && input.span > 0.0) {
            return Err(format!("refused: the span {} is not positive", input.span));
        }
        let rows = rows(input);
        let verdict = verdict(&rows, k);
        let mut scatter = vec![format!(
            "‖D(z) − D(E(D(z)))‖_phys over interior samples (the shape sphere's rows ‖C(D(z)) − D(E(D(z)))‖_phys, \
             R-544), in the precision's ε (f64 2⁻⁵², f32 2⁻²³), against ε_phys = {k}·ε"
        )];
        for r in &rows {
            scatter.push(format!(
                "{} f{}: worst {:.2} at sample {}, median {:.2}, p99 {:.2}; {} interior, {} clamped, {} in the band, \
                 {} flag mismatches",
                r.source,
                r.bits,
                r.worst.0,
                r.worst.1,
                r.median,
                r.p99,
                r.interior,
                r.clamped,
                r.band,
                r.mismatches
            ));
        }
        let samples = samples(&rows);
        let mut negative = Vec::new();
        match (verdict, worst_row(&rows)) {
            (Verdict::Pass, Some(r)) => {
                scatter.push(format!(
                    "margin {:.1}× (k = {k} over the worst, {:.2}, {} f{})",
                    k / r.worst.0,
                    r.worst.0,
                    r.source,
                    r.bits
                ));
                negative.push(format!(
                    "0 of {samples} samples beyond ε_phys or against their clamp flags"
                ));
            }
            _ => {
                let over: Vec<String> = rows
                    .iter()
                    .filter(|r| fails(r, k))
                    .map(|r| {
                        format!(
                            "{} f{} (worst {:.2}, {} interior, {} mismatches)",
                            r.source, r.bits, r.worst.0, r.interior, r.mismatches
                        )
                    })
                    .collect();
                negative.push(format!("failing rows: {}", over.join("; ")));
            }
        }
        if input.encode == EncodeKind::Mismatched {
            negative.push(
                "measured with E under every link swapped and D under the defaults (PIT-3: it must fail)"
                    .to_owned(),
            );
        }
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

impl Gate for EpsPhysGate {
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
            .map_err(|e| format!("refused: {name} is not an encode-eps-phys input: {e}"))?;
        Self::judge(config, name, &input)
    }
}
