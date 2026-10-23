//! The `nsub-bucket` gate (TASK-M3-03; REQ-INT-028; dd_integrator §3.3, R-502, R-113): the `N_sub` decision on the
//! CPU at f64 and at f32, through the kernel's substep control (`kernel::driver::substep`) and the host's threshold
//! table (`kernel::driver::nsub_table`), for one sim key per input.
//!
//! Each input names a sim key, `config` (`r_sub`, `gamma_sub`, `N_max`), and may name another, `table`, whose thresholds
//! are generated and then used under `config`: the control, the default table under a changed `r_sub` or `gamma_sub`,
//! which must fail. Three things are measured, each counted, and the gate passes when their sum is within the threshold,
//! 0 (REQ-INT-028):
//! - **Forks** between the legs. `fuzz.pairs` position triples, each with one pair placed at the square root of a
//!   threshold, scaled by a random factor within `2⁻¹⁶` of 1, at a random angle, the third body 10 away: the f64 leg
//!   passes the f64 positions to `d2_quantised::<f64>`, the f32 leg their f32 casts to `d2_quantised::<f32>`, and both
//!   take `n_sub` against the one table. Every `d²` sample below is decided on both legs too, as an f64 position and its
//!   f32 cast.
//! - **The tree at the edges.** At every threshold `THR[n]`, `n = 1 … L_max`, its f32 value and its two f32
//!   neighbours (±1 ulp), the tree's `N_sub` against the table's own count, `min(1 + #{m : d² < THR[m]}, N_max)`.
//! - **The specification away from the edges.** At those `d²`, at `fuzz.random` `d²` drawn log-uniformly from a quarter
//!   of the least threshold to four times the greatest, and at the pairs' exact `d²` (their f32 positions' differences
//!   squared in f64), `N_sub` against `clamp(⌈(r_sub/√d²)^{γ_sub}⌉, 1, N_max)` evaluated in f64, clamped before the cast,
//!   wherever `d²` is more than `2⁻²⁰` (relative) from every exact edge `(r_sub·n^{−1/γ_sub})²`; within it the f32
//!   rounding of `THR[n]`, half an ulp (`2⁻²⁴`) away, legitimately decides, and the sample is counted, not judged.
//!
//! **Domain** (pitfalls §10): identical inputs per step. A fork count of 0 says the two CPU legs decide alike given the
//! same quantised positions and sim key, one macro-step at a time; it says nothing about a trajectory, whose inputs
//! diverge by precision (R-84). The native-GPU and browser legs are REQ-VAL-059's and REQ-VAL-144's (R-113).

use serde::Deserialize;

use kernel::driver::nsub_table::{self, NsubConfig};
use kernel::driver::substep::{d2_quantised, n_sub, NsubThresholds};
use kernel::generated::nsub_thresholds::NSUB_TABLE_MAX;

use crate::decode_f32_eps::SplitMix;
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "nsub-bucket";

/// The relative distance from an exact edge within which a sample is not judged against the specification.
pub const EDGE_BAND: f64 = 1.0 / (1u64 << 20) as f64;

/// A sim key's substep law as an input writes it.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Law {
    pub r_sub: f64,
    pub gamma_sub: f64,
    pub n_max: u32,
}

impl Law {
    /// The law as the host generator takes it.
    pub fn config(self) -> NsubConfig {
        NsubConfig {
            r_sub: self.r_sub,
            gamma_sub: self.gamma_sub,
            n_max: self.n_max,
        }
    }
}

/// The draws an input takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fuzz {
    pub seed: u64,
    pub random: u32,
    pub pairs: u32,
}

/// One input (the module's note).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub description: String,
    pub config: Law,
    #[serde(default)]
    pub table: Option<Law>,
    pub fuzz: Fuzz,
}

/// What an input measured.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Measurement {
    /// Decisions taken on both legs.
    pub decisions: u32,
    /// Of them, those the legs took differently.
    pub forks: u32,
    /// The first fork, described.
    pub first_fork: Option<String>,
    /// Edge samples, three per threshold.
    pub edge_samples: u32,
    /// Of them, those where the tree and the table's count differ.
    pub edge_mismatches: u32,
    /// The first such, described.
    pub first_edge_mismatch: Option<String>,
    /// Samples judged against the specification.
    pub judged: u32,
    /// Of them, those where `N_sub` and the specification differ.
    pub mismatches: u32,
    /// The first such, described.
    pub first_mismatch: Option<String>,
    /// Samples within [`EDGE_BAND`] of an edge, not judged.
    pub near_edge: u32,
    /// The least and greatest `N_sub` seen.
    pub n_sub_range: (u32, u32),
    /// The decisions at `N_max`.
    pub saturated: u32,
}

/// `clamp(⌈(r_sub/√d2)^{γ_sub}⌉, 1, N_max)` in f64, clamped in f64 before the cast (gpu_determinism_note rule 3); `d2 = 0`
/// is `r_min = 0`, so `N_max`.
pub fn spec(law: &Law, d2: f64) -> u32 {
    let x = (law.r_sub / d2.sqrt()).powf(law.gamma_sub).ceil();
    x.clamp(1.0, f64::from(law.n_max)) as u32
}

/// The exact edge `(r_sub·n^{−1/γ_sub})²` in f64, by std's `powf`, apart from the generator's path.
pub fn edge(law: &Law, n: u32) -> f64 {
    let s = law.r_sub * f64::from(n).powf(-1.0 / law.gamma_sub);
    s * s
}

/// Whether `d2` is more than [`EDGE_BAND`] from every exact edge of `law`, `n = 1 … L_max`.
pub fn away_from_edges(law: &Law, d2: f64) -> bool {
    (1..=NSUB_TABLE_MAX as u32).all(|n| (d2 / edge(law, n) - 1.0).abs() > EDGE_BAND)
}

/// The table's own count: `min(1 + #{m : d2 < THR[m]}, n_max)`.
pub fn count(bits: &[u32; NSUB_TABLE_MAX], d2: f32, n_max: u32) -> u32 {
    let k = bits.iter().filter(|&&b| d2 < f32::from_bits(b)).count() as u32;
    (k + 1).min(n_max)
}

/// The two legs' `N_sub` for the positions `r`: the f64 leg on `r`, the f32 leg on its f32 casts.
pub fn legs(r: &[[f64; 2]; 3], table: &NsubThresholds, n_max: u32) -> (u32, u32) {
    let r32 = r.map(|p| p.map(|x| x as f32));
    (
        n_sub(d2_quantised::<f64>(r), table, n_max),
        n_sub(d2_quantised::<f32>(&r32), table, n_max),
    )
}

/// The exact `d²` of the closest pair of `r`'s f32 casts, in f64.
pub fn exact_d2(r: &[[f64; 2]; 3]) -> f64 {
    let q = r.map(|p| p.map(|x| f64::from(x as f32)));
    let d = |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2);
    d(q[1], q[2]).min(d(q[2], q[0])).min(d(q[0], q[1]))
}

/// A position triple whose pair (0, 1) is `d2`'s square root apart, along `angle`, from `at`, the third body 10 away.
fn triple(at: [f64; 2], d2: f64, angle: f64) -> [[f64; 2]; 3] {
    let s = d2.sqrt();
    [
        at,
        [at[0] + s * angle.cos(), at[1] + s * angle.sin()],
        [at[0] + 10.0, at[1] + 10.0],
    ]
}

impl Measurement {
    /// Records one decision on both legs.
    fn decide(&mut self, what: &str, (a, b): (u32, u32), n_max: u32) {
        self.decisions += 1;
        if a != b {
            self.forks += 1;
            self.first_fork
                .get_or_insert_with(|| format!("{what}: f64 leg {a}, f32 leg {b}"));
        }
        let (lo, hi) = self.n_sub_range;
        self.n_sub_range = if self.decisions == 1 {
            (a, a)
        } else {
            (lo.min(a), hi.max(a))
        };
        if a == n_max {
            self.saturated += 1;
        }
    }

    /// Judges `got` against the specification at the exact `d2`, or counts it near an edge.
    fn judge(&mut self, what: &str, law: &Law, d2: f64, got: u32) {
        if !away_from_edges(law, d2) {
            self.near_edge += 1;
            return;
        }
        self.judged += 1;
        let want = spec(law, d2);
        if got != want {
            self.mismatches += 1;
            self.first_mismatch.get_or_insert_with(|| {
                format!("{what}: d² {d2:e}, N_sub {got}, the specification {want}")
            });
        }
    }
}

/// Runs `input`'s draws (the module's note); `Err` where the host generator refuses a sim key.
pub fn measure(input: &Input) -> Result<Measurement, String> {
    let law = input.config;
    let from = input.table.unwrap_or(law);
    let bits = nsub_table::threshold_bits(&from.config()).map_err(|e| format!("refused: {e}"))?;
    nsub_table::threshold_bits(&law.config()).map_err(|e| format!("refused: {e}"))?;
    let table = nsub_table::pack(&bits);
    let n_max = law.n_max;
    let mut m = Measurement::default();
    let mut rng = SplitMix::new(input.fuzz.seed);
    let mut unit = move || (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    let mut d2_samples = Vec::new();
    for (i, &b) in bits.iter().enumerate() {
        for (side, d2) in [
            ("−1 ulp", b.saturating_sub(1)),
            ("exact", b),
            ("+1 ulp", b + 1),
        ] {
            let d2 = f32::from_bits(d2);
            let what = format!("THR[{}] {side}", i + 1);
            m.edge_samples += 1;
            let got = n_sub(d2, &table, n_max);
            let want = count(&bits, d2, n_max);
            if got != want {
                m.edge_mismatches += 1;
                m.first_edge_mismatch.get_or_insert_with(|| {
                    format!("{what}: d² {d2:e}, the tree {got}, the table's count {want}")
                });
            }
            d2_samples.push((what, d2));
        }
    }
    let (lo, hi) = (
        (f64::from(f32::from_bits(bits[NSUB_TABLE_MAX - 1])) / 4.0).log2(),
        (f64::from(f32::from_bits(bits[0])) * 4.0).log2(),
    );
    for i in 0..input.fuzz.random {
        let d2 = (lo + (hi - lo) * unit()).exp2() as f32;
        d2_samples.push((format!("random d² #{i}"), d2));
    }
    for (what, d2) in &d2_samples {
        // As a position triple at the origin, along x: the f32 cast of `√d²` squared, the legs' common input.
        let r = triple([0.0, 0.0], f64::from(*d2), 0.0);
        m.decide(what, legs(&r, &table, n_max), n_max);
        m.judge(what, &law, f64::from(*d2), n_sub(*d2, &table, n_max));
    }
    for i in 0..input.fuzz.pairs {
        let n = 1 + (unit() * NSUB_TABLE_MAX as f64) as u32 % NSUB_TABLE_MAX as u32;
        let scale = 1.0 + (2.0 * unit() - 1.0) / f64::from(1u32 << 16);
        let d2 = f64::from(f32::from_bits(bits[n as usize - 1])) * scale * scale;
        let at = [2.0 * unit() - 1.0, 2.0 * unit() - 1.0];
        let r = triple(at, d2, std::f64::consts::TAU * unit());
        let what = format!("pair #{i} at THR[{n}]");
        let (a, b) = legs(&r, &table, n_max);
        m.decide(&what, (a, b), n_max);
        m.judge(&what, &law, exact_d2(&r), a);
    }
    Ok(m)
}

/// The `nsub-bucket` gate.
pub struct NsubBucketGate;

impl NsubBucketGate {
    /// The report on `input`, named `name`, against `config`.
    pub fn judge(config: &GateConfig, name: &str, input: &Input) -> Result<GateReport, String> {
        let m = measure(input)?;
        let failures = f64::from(m.forks + m.edge_mismatches + m.mismatches);
        let verdict = if failures <= config.threshold.value {
            Verdict::Pass
        } else {
            Verdict::Fail
        };
        let law = input.config;
        let mut scatter = vec![
            format!(
                "sim key r_sub {}, gamma_sub {}, N_max {}; table from {}",
                law.r_sub,
                law.gamma_sub,
                law.n_max,
                match input.table {
                    Some(t) => format!(
                        "r_sub {}, gamma_sub {} (the control: another sim key's table)",
                        t.r_sub, t.gamma_sub
                    ),
                    None => "the same sim key".to_owned(),
                }
            ),
            format!(
                "forks between CPU-f64 and CPU-f32: {} of {} decisions",
                m.forks, m.decisions
            ),
            format!(
                "the tree against the table's count at every edge (THR[n] and ±1 ulp, n = 1…{NSUB_TABLE_MAX}): {} of {} \
                 differ",
                m.edge_mismatches, m.edge_samples
            ),
            format!(
                "against the specification more than 2⁻²⁰ from every edge: {} of {} differ; {} within it, not judged",
                m.mismatches, m.judged, m.near_edge
            ),
            format!(
                "N_sub seen from {} to {}; {} decisions at N_max",
                m.n_sub_range.0, m.n_sub_range.1, m.saturated
            ),
        ];
        scatter.extend(m.first_fork.iter().map(|s| format!("first fork: {s}")));
        scatter.extend(
            m.first_edge_mismatch
                .iter()
                .map(|s| format!("first edge difference: {s}")),
        );
        scatter.extend(
            m.first_mismatch
                .iter()
                .map(|s| format!("first difference from the specification: {s}")),
        );
        let negative_results = vec![
            "domain: identical inputs per step (pitfalls §10): the two CPU legs decide alike given the same quantised \
             positions and sim key, one macro-step at a time; this says nothing about a trajectory, whose inputs \
             diverge by precision (R-84)"
                .to_owned(),
            "CPU-f64 and CPU-f32 only: the native-GPU leg is REQ-VAL-059's, the browser's REQ-VAL-144's (R-113)"
                .to_owned(),
            format!(
                "{} samples within 2⁻²⁰ of an edge are not judged against the specification: there the f32 rounding of \
                 THR[n] decides",
                m.near_edge
            ),
        ];
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict,
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(NSUB_TABLE_MAX as u32),
                minimum: config.min_regions.clone(),
            }),
            negative_results,
        })
    }
}

impl Gate for NsubBucketGate {
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
            .map_err(|e| format!("refused: {name} is not an nsub-bucket input: {e}"))?;
        Self::judge(config, name, &input)
    }
}
