//! The `shape-roundtrip` gate (TASK-M2-08; REQ-CHART-020, REQ-CHART-053): the shape-sphere chart's round trip,
//! `shape(Φ(u, v)) = n(u, v)`, with `Φ` alone, no canonicaliser (R-178), over random masses and shapes on both
//! hemispheres and at both poles, against the bound R-181 sets provisionally, `shape_roundtrip_bound` in the constants
//! register (R-427), until the human confirms it at the M2 gate.
//!
//! **The measure.** At each sample `n`, `Φ`'s configuration under the masses ([`place`]: chart_reference §3.2's
//! closed-form inverse at `I` and `φ_f`, unweighted and reconstructed per §0.2), and the larger of two distances:
//! `‖shape(r, m) − n‖`, the shape map on the reconstructed positions, and `‖shape_vec(ρ̃, λ̃) − n‖`, §3.1's map on the
//! inverse's own Jacobi vectors. The gate passes when the worst over the input is within the threshold.
//!
//! **The samples**, from a fixed seed (SplitMix64): masses through the shared mass block on `z_μ ∈ [−4, 4]²` (the
//! defaults link set), `I ∈ [¼, 4]`, `φ_f ∈ [−π, π]`; half of the points from the spherical map at `(s, t)` uniform on
//! `[0, 1]²`, so both hemispheres, half from the exponential map about a random orthonormal frame with extent up to `π`;
//! `near_pole_samples` draws about each of `n₀ = ±1` at a polar radius `10⁻ᵏ`, `k` uniform on `[2, 15]`, where `n₀`
//! rounds to `±1` from `k ≈ 8` on (the inverse keeps the radius through `n₁² + n₂²`, `kernel::chart::shape_sphere`);
//! and `pole_samples` draws at each of `n = (±1, ±0, ±0)`, every sign of the zeros, where R-437's select gives
//! `ψ = φ_f`.
//!
//! **The negative input** (`inverse: "negative_cross"`): `Φ` built with the earlier `ψ = φ_f + atan2(−q, p)`, the
//! inverse of §3.1's superseded negative-cross `q` (chart_reference §3.2), which is the closed form at `(n₀, n₁, −n₂)`;
//! it misses every point off the equator, so the gate must fail it (REQ-CHART-020; PIT-3).
//!
//! **The proposal** (REQ-CHART-053, R-71, R-181): 10⁻¹³ absolute, R-181's value, with the measured worst over
//! `random.json` as its evidence (the gate prints it and its margin).

use std::path::Path;

use serde::Deserialize;

use kernel::chart::shape_sphere::{exponential, place, spherical};
use kernel::decode::mass;
use kernel::generated::links::Defaults;
use kernel::shape::{shape, shape_vec};

use crate::decode_f32_eps::SplitMix;
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::oklab::quantile;
use crate::SHAPE_ROUNDTRIP_BOUND;

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "shape-roundtrip";

const PI: f64 = std::f64::consts::PI;

/// Which inverse `Φ` is built with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InverseKind {
    /// chart_reference §3.2's, `ψ = φ_f + atan2(q, p)`.
    ClosedForm,
    /// The earlier `ψ = φ_f + atan2(−q, p)`, which must fail.
    NegativeCross,
}

/// A `shape-roundtrip` input, `fixtures/gates/shape-roundtrip/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The seed every draw comes from.
    pub seed: u64,
    /// The random samples, half from each map.
    pub samples: u32,
    /// The draws about each of `n₀ = ±1` at polar radii `10⁻²` to `10⁻¹⁵`; 0 for none.
    pub near_pole_samples: u32,
    /// The draws at each of the eight poles `(±1, ±0, ±0)`; 0 for none.
    pub pole_samples: u32,
    /// The inverse `Φ` is built with.
    pub inverse: InverseKind,
}

/// The input `name` of the gate's fixtures under `root`, its `"expected"` set aside.
pub fn input(root: &Path, name: &str) -> Result<Input, String> {
    let path = root.join("fixtures/gates").join(NAME).join(name);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(o) = value.as_object_mut() {
        o.remove("expected");
    }
    Input::deserialize(&value).map_err(|e| format!("{}: {e}", path.display()))
}

/// One sample: the point, the masses, `I` and `φ_f`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// The shape-sphere point `n`.
    pub n: [f64; 3],
    /// The masses' controls `(z_μ1, z_μ2)`.
    pub z_mass: [f64; 2],
    /// The moment of inertia `I`.
    pub inertia: f64,
    /// The fibre phase `φ_f`.
    pub phase: f64,
}

/// A uniform draw on `[lo, hi]`.
fn on(rng: &mut SplitMix, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * ((rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64)
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let k = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    a.map(|x| x / k)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The input's samples, in draw order: the random ones (even spherical, odd exponential), then those near `n₀ = ±1`,
/// then the poles.
pub fn samples(input: &Input) -> Vec<Sample> {
    let mut rng = SplitMix::new(input.seed);
    let gauge = |rng: &mut SplitMix, n| Sample {
        n,
        z_mass: [on(rng, -4.0, 4.0), on(rng, -4.0, 4.0)],
        inertia: on(rng, 0.25, 4.0),
        phase: on(rng, -PI, PI),
    };
    let mut out = Vec::new();
    for k in 0..input.samples {
        let n = if k % 2 == 0 {
            spherical(on(&mut rng, 0.0, 1.0), on(&mut rng, 0.0, 1.0), PI)
        } else {
            let n0 = unit([0, 1, 2].map(|_| on(&mut rng, -1.0, 1.0)));
            let e1 = unit(cross(n0, [0, 1, 2].map(|_| on(&mut rng, -1.0, 1.0))));
            let (scale, u, v) = (
                on(&mut rng, 0.0, PI),
                on(&mut rng, 0.0, 1.0),
                on(&mut rng, 0.0, 1.0),
            );
            exponential(n0, e1, cross(n0, e1), scale, u, v)
        };
        out.push(gauge(&mut rng, n));
    }
    for sign in [1.0, -1.0] {
        for _ in 0..input.near_pole_samples {
            let r = 10f64.powf(-on(&mut rng, 2.0, 15.0));
            let chi = on(&mut rng, -PI, PI);
            let n = [sign * r.cos(), r.sin() * chi.cos(), r.sin() * chi.sin()];
            out.push(gauge(&mut rng, n));
        }
    }
    for n0 in [1.0, -1.0] {
        for n1 in [0.0, -0.0] {
            for n2 in [0.0, -0.0] {
                for _ in 0..input.pole_samples {
                    out.push(gauge(&mut rng, [n0, n1, n2]));
                }
            }
        }
    }
    out
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// The round trip's misses at `s` with `Φ` built by `inverse`: `‖shape(r, m) − n‖`, through the reconstructed
/// positions, and `‖shape_vec(ρ̃, λ̃) − n‖`, through the inverse's Jacobi vectors.
pub fn miss(s: &Sample, inverse: InverseKind) -> (f64, f64) {
    let m = mass::decode::<Defaults, f64>(s.z_mass);
    let built = match inverse {
        InverseKind::ClosedForm => s.n,
        InverseKind::NegativeCross => [s.n[0], s.n[1], -s.n[2]],
    };
    let c = place(built, s.inertia, s.phase, &m);
    (
        dist(shape(c.r, m.m), s.n),
        dist(shape_vec(c.rho_t, c.lambda_t), s.n),
    )
}

/// The verdict on a worst miss against the threshold: a pass at or below it.
pub fn verdict(worst: f64, threshold: f64) -> Verdict {
    if worst <= threshold {
        Verdict::Pass
    } else {
        Verdict::Fail
    }
}

/// The `shape-roundtrip` gate.
pub struct ShapeRoundtripGate;

impl ShapeRoundtripGate {
    /// Measures `input` and judges it against `config`'s threshold, which must be the register's
    /// `shape_roundtrip_bound` (R-427), or refuses it.
    pub fn judge(config: &GateConfig, name: &str, input: &Input) -> Result<GateReport, String> {
        let threshold = config.threshold.value;
        let register = SHAPE_ROUNDTRIP_BOUND.number();
        if threshold != register {
            return Err(format!(
                "refused: gate.json's threshold {threshold:e} is not the register's shape_roundtrip_bound \
                 {register:e} (R-427)"
            ));
        }
        if input.samples == 0 {
            return Err("refused: no samples".to_owned());
        }
        let all = samples(input);
        let pairs: Vec<(f64, f64)> = all.iter().map(|s| miss(s, input.inverse)).collect();
        let misses: Vec<f64> = pairs.iter().map(|&(a, b)| a.max(b)).collect();
        let (by_positions, by_jacobi) = pairs
            .iter()
            .fold((0.0f64, 0.0f64), |(x, y), &(a, b)| (x.max(a), y.max(b)));
        let worst = misses.iter().copied().fold(0.0, f64::max);
        let at = misses.iter().position(|&x| x == worst).unwrap_or(0);
        let random = input.samples as usize;
        let near = 2 * input.near_pole_samples as usize;
        let near_max = misses[random..random + near]
            .iter()
            .copied()
            .fold(0.0, f64::max);
        let poles = &misses[random + near..];
        let lower = all[..random].iter().filter(|s| s.n[2] < 0.0).count();
        let q = |x| quantile(&misses, x).unwrap_or(f64::NAN);
        let mut scatter = vec![
            format!(
                "‖shape(Φ(n)) − n‖: max {worst:.3e} at sample {at} (n {:?}); median {:.3e}, p99 {:.3e}, over {} samples",
                all[at].n,
                q(0.5),
                q(0.99),
                misses.len()
            ),
            format!(
                "max by path: ‖shape(r, m) − n‖ on the reconstructed positions {by_positions:.3e}, \
                 ‖shape_vec(ρ̃, λ̃) − n‖ on the inverse's Jacobi vectors {by_jacobi:.3e}"
            ),
            format!(
                "{random} random samples, {lower} on the lower hemisphere (w < 0); {near} near n₀ = ±1 at polar radii \
                 10⁻² to 10⁻¹⁵, their max {near_max:.3e}; {} at the eight poles (±1, ±0, ±0), their max {:.3e}",
                poles.len(),
                poles.iter().copied().fold(0.0, f64::max)
            ),
        ];
        let over = misses.iter().filter(|&&x| x > threshold).count();
        let mut negative = Vec::new();
        let pass = verdict(worst, threshold) == Verdict::Pass;
        if pass {
            scatter.push(format!(
                "margin {:.1}× (threshold {threshold:.3e} over max {worst:.3e})",
                threshold / worst
            ));
            negative.push(format!(
                "0 of {} samples beyond the threshold {threshold:.3e}",
                misses.len()
            ));
        } else {
            negative.push(format!(
                "{over} of {} samples beyond the threshold {threshold:.3e}; the worst, {worst:.3e} at sample {at}, fails",
                misses.len()
            ));
        }
        if input.inverse == InverseKind::NegativeCross {
            negative.push(
                "measured with ψ = φ_f + atan2(−q, p), the superseded negative-cross inverse (PIT-3: it must fail)"
                    .to_owned(),
            );
        }
        Ok(GateReport {
            gate: NAME.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: Some(Regions {
                count: RegionCount::Recorded(
                    u32::try_from(misses.len()).map_err(|e| e.to_string())?,
                ),
                minimum: config.min_regions.clone(),
            }),
            negative_results: negative,
        })
    }
}

impl Gate for ShapeRoundtripGate {
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
            .map_err(|e| format!("refused: {name} is not a shape-roundtrip input: {e}"))?;
        Self::judge(config, name, &input)
    }
}
