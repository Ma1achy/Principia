//! The `decode-f32-eps` gate (TASK-M2-06; REQ-DEC-043): dd_decoder §5 test 12's continuous values, the kernel's decode
//! stage at f32 on the GPU against the same source at f64 on the CPU (`engine::decode_dispatch::decode_only`), on the
//! golden IC and on fuzzed interiors, and the multiple of f32's `eps` that covers their differences. The calibration
//! (R-71) proposes that multiple; it is provisional in `fixtures/gates/decode-f32-eps/gate.json` until the human
//! confirms it at the M2 gate.
//!
//! **The measure.** For each of a sample's 27 continuous values, its `(m, r, p)` (15) and its `ICDescriptor`'s twelve
//! fields, the difference in units of f32's `eps` ([`f32::EPSILON`], `2⁻²³`) scaled by the f64 value's magnitude where
//! that is above 1 and by 1 below it: `|x₃₂ − x₆₄| / (eps · max(|x₆₄|, 1))` ([`error`]). The decode's output is in
//! `G = M = I = 1`, so its values are of order 1 and the measure is relative where a value is large (`V_0`, `rho_ratio`)
//! and absolute where it is near 0 (a component of `r` or `p`, `rho_angle` near the axis), where a relative measure
//! would divide by the value's own rounding.
//!
//! **The inputs.** Each sample's `(u, v)` and the chart's parameters are f32, and the f64 decode reads them widened,
//! never narrowed, so both decode exactly the same input (pitfalls §10: the inputs held fixed). The golden IC is the
//! latent chart at `z₀ = q₁ = q₂ = 0` and `(u, v) = (½, ½)`, `z = 0` (dd_decoder §5). A fuzzed interior is drawn from a
//! fixed seed ([`Fuzz`]): `charts` latent charts, `z₀`'s components on `[−1, 1]` and `q₁`'s and `q₂`'s on `[−½, ½]`, each
//! decoded at `per_chart` points `(u, v)` on `[0, 1]²`, so every `|zₖ| ≤ 2`, the interior the decoder's tests use
//! (`kernel/tests/ic_descriptor.rs`). The defaults link set ([`LatentDefaults`]).
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed, on this machine's GPU backend; it says nothing
//! about a trajectory, and another backend is measured by running the gate there (R-85: native wgpu sets the Tier-N
//! tolerances; this is decoder test 12's own factor, not those).
//!
//! **The proposal** (REQ-DEC-043, R-71): **256 eps** (≈ 3.05e-5), `gate.json`'s threshold, provisional until the human
//! confirms it at the M2 gate. **Its domain:** the defaults link set, every `|zₖ| ≤ 2`, Metal with fast-math off (R-84),
//! the decode alone (no trajectory). Measured on an Apple M3 Pro by this gate's measure:
//!
//! | input | samples | worst (eps) | at | median | p99 |
//! |---|---|---|---|---|---|
//! | `golden.json` | 1 | 0.87 | `r0x` | — | — |
//! | `interior.json` (seed 1, 64 × 256) | 16 384 | 42.55 | `r2x` | 3.21 | 12.18 |
//! | sweep: seeds 1 to 16, 256 charts × 256 points each | 1 048 576 | 175.21 | `virial_ratio` (seed 7) | ≈ 3.1 | ≈ 12 |
//! | the same sweep, f32 on the CPU (native), not the GPU | 1 048 576 | 188.27 | | | |
//! | outside the domain: the defaults at saturated controls | | 188.62 | | | |
//! | outside the domain: `stick_breaking` at saturated controls | | 2.53e6 | light bodies at `‖r‖` of 10³ to 2·10⁶ | | |
//!
//! The two saturated rows are physics review 5539636118's measurement, against f64. They lie outside the proposal's
//! domain. At `stick_breaking`'s saturation the gap is f32's own: the GPU is 16.5 and 7 eps from the native f32 decode,
//! not 2.53e6, so the backend adds nothing there; a factor for saturated controls, or for another link set, would be
//! measured on its own domain.
//!
//! **Its use across the chart registry** (decoder test 12, `engine/tests/decode_cross_backend.rs`; TASK-M2-24, physics
//! review 5593488673 F3). The test holds the same factor, by the same measure, on every chart's own fuzzed interior,
//! each decoded on the GPU through its stage variant from f32 `(u, v)` and parameters, `η_E` off and toward targets
//! `0, −0.3, −40, 1.5` where the chart allows it. That widens the domain above, the defaults' latent interior, to these
//! domains, and the confirmation at the M2 gate covers them. Worst over the 27 values, against the f64 decode and
//! against the native f32 decode (rustc's, the backend's own share), in eps:
//!
//! | domain | Metal (M3 Pro), against f64 | Metal, against native f32 | lavapipe, against f64 | lavapipe, against native f32 |
//! |---|---|---|---|---|
//! | latent, defaults links (golden, interior seed 24, rest starts) | 12.43 | 19.22 | 14.51 | 19.11 |
//! | latent, `stick_breaking`, interior (`q` on `[−½, ½]`) | 21.57 | 15.50 | 25.66 | 20.00 |
//! | `(L_z, E)` / `(L_z, K)`, `s ∈ [0.01, 0.99]` | 4.67 / 4.53 | 5.21 / 7.63 | 11.07 / 9.00 | 11.65 / 9.60 |
//! | ternary mass plot | 9.91 | 16.00 | 25.91 | 7.19 |
//! | `(ν, K)` Burrau / Euclid plane | 4.43 / 3.64 | 6.61 / 2.72 | 7.61 / 7.44 | 7.41 / 7.23 |
//! | `(θ, K)` / `(θ, L_z)` / `(θ, Δm)` strips | 10.57 / 9.70 / 22.12 | 9.56 / 8.50 / 15.44 | 7.68 / 9.34 / 8.73 | 7.17 / 7.40 / 7.38 |
//! | mixed `(z_μ1, α)` | 9.76 | 12.82 | 17.50 | 24.10 |
//! | shape sphere, both maps, off its poles | 10.89 | 12.64 | 45.07 | 44.57 |
//!
//! Outside the domain, measured and reported, not held to the factor:
//!
//! | domain | Metal, against f64 | Metal, against native f32 | lavapipe, against f64 | lavapipe, against native f32 | why |
//! |---|---|---|---|---|---|
//! | `stick_breaking` at saturated controls (`q` on `[−20, 20]`) | 1.58e10 | 7.40e6 | 1.58e10 | 4.16e6 | f32's own conditioning, as above |
//! | `(L_z, E)` / `(L_z, K)` edge columns `s ∈ {0, 1}` | 1119.6 / 885.3 | 686.1 / 523.1 | 1276.3 / 795.3 | 978.6 / 746.3 | `a = √(2(K* − L_z²/2))` of a rounding-sized radicand amplifies a one-ulp difference in a backend's `sqrt` by about `1/√eps` (TASK-M2-12's stepped `L_max`); the feasibility tag there is held bitwise |
//! | `(L_z, E)` / `(L_z, K)` past the square in `s` | 4.34 / 4.52 | 5.71 / 5.84 | 10.53 / 9.00 | 11.52 / 9.60 | every pixel `INVARIANT_INFEASIBLE`, the rigid rotation; outside the chart's view |
//! | shape sphere's `n₀ = ±1` poles, `(s, t) = (½, ½), (0, ½), (1, ½)` | 4.12e6 | 3.94e6 | 1.61e6 | 2.88e6 | `BC₀₁` (`ρ̃ = 0`) and `λ̃ = 0`, reached only to each precision's rounding of `sin π` and `cos(π/2)` |
//!
//! The lavapipe columns are the `gpu-kernel` job's (Mesa's lavapipe, llvmpipe on LLVM 20.1.2, Vulkan; PR #262, CI run
//! 39568757113, on the code of commit 7fbf8653), which shows the test's report on success (`.config/nextest.toml`); the
//! Metal columns the same code's run on the build machine. Each row is measured at the chart's default parameters, the
//! shape sphere's at its default frame (`n₀ = +w`), scale 1.2, `I = 1` and `φ_f = 0`, only its held blocks drawn.
//! There the worst is 45.07 (the shape sphere's `virial_ratio` on lavapipe), 5.7 below 256.
//!
//! **Off the defaults** (qa's `qa_m224_gpu_new_charts`, `engine/tests/qa_TASK-M2-24_gpu.rs`): the `(θ, K)`, `(θ, L_z)`
//! and `(θ, Δm)` strips with drawn ranges, warps and targets, and the shape sphere with `n₀ = (0, 0.6, 0.8)` and its
//! scale, `I`, `φ_f` and held blocks drawn, which reach masses as light as `m₁ ≈ 1.2·10⁻³`, off its poles; `η_E` off and
//! toward 0 and 1.5. Worst against f64, in eps:
//!
//! | domain | Metal (M3 Pro) | lavapipe |
//! |---|---|---|
//! | `(θ, K)` / `(θ, L_z)` / `(θ, Δm)` strips | 10.68 / 5.62 / 3.03 | 7.54 / 7.52 / 7.52 |
//! | shape sphere, spherical / exponential map | 13.76 / 27.11 | 72.81 / 58.62 |
//!
//! Off the defaults the lavapipe worst is 72.81 (`r2x`, the spherical map, `z_mass = (−0.490, −0.724)`, at
//! `(0.484, 0.249)`, where the native f32 decode is 3.06), 3.5 below 256 (CI run 39572746056's `gpu-kernel` job, a
//! diagnostic on the code of 7fbf8653). Before 7fbf8653 the lavapipe decode exceeded the factor there: `r1y` 273.88
//! eps from f64 at `(0.9118395, 0.6456125)` (masses about `(0.0514, 0.00124, 0.947)`, native f32 1.12), the light
//! body's `‖r₁‖ ≈ 27` multiplying the error of `C`'s turn by `sin` and `cos` of a rounded `atan2`. `C` now turns by
//! `ρ/‖ρ‖` (`kernel::canon::rotation`), and that value is 12.12 (native f32 2.62).
//!
//! **`C` alone** (qa's `qa_m224_gpu_canon_turn`, `engine/tests/qa_TASK-M2-24_gpu.rs`; physics review 5599301723 F1).
//! The test holds the same factor, by the same measure, on `C`'s `(m, r, p)` (`kernel::canon`, the decode's last step)
//! from f32 states, against f64 `C` on the same states widened: 4096 states at every orientation (a turn drawn on
//! `[−3.2, 3.2]` rad); scale `2^k` with `k` on `[−19, 19]`, `k = 0` drawn twice as often as each other value (`k` is a
//! draw on `[−20, 20]` truncated toward zero); a light body of mass `10^−3.5` to `10^−2` at 4 to 40 from the CoM of the
//! heavy two; canonical momenta, left unnormalised, up to about 10³. Worst against f64, in eps:
//!
//! | domain | Metal (M3 Pro) | CPU, f32 (native) | lavapipe |
//! |---|---|---|---|
//! | `C` alone, the domain above | 81.29 (in `p`) | 81.29 | 162.71 |
//!
//! The lavapipe value is the `gpu-kernel` job's (llvmpipe on LLVM 20.1.2; PR #262, CI run 39599090423, job
//! 119219350820, on commit 43164ef3, whose code is 8666c7bd's), which shows the test's report on success
//! (`.config/nextest.toml`); the Metal and CPU values are the same code's run on the build machine, the Metal value's
//! place, in `p`, physics review 5599301723's reading. The test reports its worst over `(m, r, p)`, not where it lies
//! nor its distance from the native f32 `C` on the same state; lavapipe's worst is twice the CPU's f32 worst over the
//! same states. Over the domains TASK-M2-24 adds (the chart registry, off the defaults, `C` alone) the worst is this
//! 162.71, 1.57 below 256; over every domain this record holds the factor on, the proposal sweep's 175.21
//! (`virial_ratio`, seed 7, Metal) remains the worst, 1.46 below 256.
//!
//! Each seed's worst but seed 7's lay between 38 and 129. The tail is f32's conditioning, not the backend: the CPU's own
//! f32 decode is as far from f64 as the GPU's; the worst values are a small component of a large vector (`r2x`) and a
//! small `K_0` over cancelling momenta (`virial_ratio`). 256 covers the sweep's worst by a factor of 1.46; the perturbed
//! input (`perturbed.json`, 10⁻⁴ on the first latent component) measures ≈ 2235 eps and fails.
//!
//! An input may perturb one latent component of the f64 reference by `perturb` before measuring, so the gate shows that
//! a decode that is not the kernel's fails it (pitfalls §3: check the measurement can fire); and record a measurement
//! made on that input (`recorded`), set beside the one made here in the report.

use std::path::Path;

use serde::Deserialize;

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{decode_only, DecodeStage};
use kernel::chart::Latent;
use kernel::payload::ICDescriptorOf;
use kernel::stage::decode::{sample, DecodeMode, DecodeOut, DecodeVariant, LatentDefaults};

use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::gpu::GpuHarness;

/// The gate's name, its directory under `fixtures/gates/`.
pub const NAME: &str = "decode-f32-eps";

/// The names of a sample's 27 continuous values, in [`values`]' order.
pub const QUANTITIES: [&str; 27] = [
    "m0",
    "m1",
    "m2",
    "r0x",
    "r0y",
    "r1x",
    "r1y",
    "r2x",
    "r2y",
    "p0x",
    "p0y",
    "p1x",
    "p1y",
    "p2x",
    "p2y",
    "d.m0",
    "d.m1",
    "d.m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
];

/// A descriptor's twelve fields, in the generated struct's order.
fn fields<R: kernel::payload::PayloadReal>(d: &ICDescriptorOf<R>) -> [R; 12] {
    [
        d.m0,
        d.m1,
        d.m2,
        d.q_mass,
        d.rho_mag,
        d.lambda_mag,
        d.rho_ratio,
        d.rho_angle,
        d.K_0,
        d.V_0,
        d.virial_ratio,
        d.r_min_pair_0,
    ]
}

/// A sample's 27 continuous values, widened to f64, named by [`QUANTITIES`].
pub fn values<R>(out: &DecodeOut<R>) -> [f64; 27]
where
    R: kernel::payload::PayloadReal + Into<f64>,
{
    let ic = &out.ic;
    let d = fields(&out.descriptor);
    let mut v = [0.0; 27];
    let flat =
        ic.m.iter()
            .chain(ic.r.iter().flatten())
            .chain(ic.p.iter().flatten())
            .chain(d.iter())
            .map(|&x| x.into());
    for (slot, x) in v.iter_mut().zip(flat) {
        *slot = x;
    }
    v
}

/// The difference of the f32 value `x32` from the f64 value `x64` in f32 `eps`, scaled by `max(|x64|, 1)` (the module's
/// note). Two values with the same non-finite bits, a pole's `±∞` stored by both, differ by 0; any other non-finite
/// value differs by `+∞`, and fails.
pub fn error(x32: f64, x64: f64) -> f64 {
    if !x32.is_finite() || !x64.is_finite() {
        return if x32.to_bits() == x64.to_bits() {
            0.0
        } else {
            f64::INFINITY
        };
    }
    (x32 - x64).abs() / (f64::from(f32::EPSILON) * x64.abs().max(1.0))
}

/// The worst error of one quantity: its value in eps and the sample it was reached at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Worst {
    /// The error, in f32 eps.
    pub eps: f64,
    /// The sample's index, in decode order.
    pub at: usize,
}

/// The gate's measurement over a set of samples.
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    /// The number of samples.
    pub samples: usize,
    /// Each quantity's worst error, in [`QUANTITIES`]' order.
    pub worst: [Worst; 27],
    /// Every sample's largest error over its quantities, in decode order.
    pub per_sample: Vec<f64>,
}

impl Measurement {
    /// The worst error over every quantity, and the quantity's index.
    pub fn max(&self) -> (f64, usize) {
        self.worst.iter().enumerate().fold(
            (0.0, 0),
            |(e, k), (j, w)| if w.eps > e { (w.eps, j) } else { (e, k) },
        )
    }
}

/// Measures the GPU's decodes `gpu` against the f64 decodes `cpu`, sample by sample. `Err` when the counts differ, or
/// a pair's `DEGENERATE` tags differ: a branch tag is compared exactly, never within a tolerance (dd_decoder §5 test 12).
pub fn measure(gpu: &[DecodeOut<f32>], cpu: &[DecodeOut<f64>]) -> Result<Measurement, String> {
    if gpu.len() != cpu.len() {
        return Err(format!(
            "{} GPU decodes against {} f64 decodes",
            gpu.len(),
            cpu.len()
        ));
    }
    let mut worst = [Worst { eps: 0.0, at: 0 }; 27];
    let mut per_sample = Vec::with_capacity(gpu.len());
    for (i, (g, c)) in gpu.iter().zip(cpu).enumerate() {
        if g.degenerate != c.degenerate {
            return Err(format!(
                "sample {i}: the GPU tags {:?}, the f64 decode {:?}",
                g.degenerate, c.degenerate
            ));
        }
        let mut sample: f64 = 0.0;
        for (k, (a, b)) in values(g).into_iter().zip(values(c)).enumerate() {
            let e = error(a, b);
            if e > worst[k].eps {
                worst[k] = Worst { eps: e, at: i };
            }
            sample = sample.max(e);
        }
        per_sample.push(sample);
    }
    Ok(Measurement {
        samples: gpu.len(),
        worst,
        per_sample,
    })
}

/// SplitMix64: the fuzz's generator, so a seed names its draws on every platform.
#[derive(Clone, Debug)]
pub struct SplitMix(u64);

impl SplitMix {
    /// The generator seeded with `seed`.
    pub fn new(seed: u64) -> Self {
        SplitMix(seed)
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A value on `[lo, hi]`, rounded to f32: 24 random bits, so the draw is exact in f32 before the affine map.
    pub fn uniform(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32;
        lo + (hi - lo) * t
    }
}

/// A fuzzed interior: `charts` latent charts and `per_chart` points each, from `seed` (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fuzz {
    pub seed: u64,
    pub charts: u32,
    pub per_chart: u32,
}

/// One chart and the points decoded through it.
pub type Batch = (Latent<f32>, Vec<[f32; 2]>);

/// The golden IC's batch: the latent chart at `z₀ = q₁ = q₂ = 0`, at `(u, v) = (½, ½)`, so `z = 0`.
pub fn golden() -> Vec<Batch> {
    let zero = Latent {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    vec![(zero, vec![[0.5, 0.5]])]
}

/// The batches of the fuzzed interior `f`.
pub fn interior(f: Fuzz) -> Vec<Batch> {
    let mut rng = SplitMix::new(f.seed);
    (0..f.charts)
        .map(|_| {
            let mut eight = |h: f32| core::array::from_fn(|_| rng.uniform(-h, h));
            let chart = Latent {
                z0: eight(1.0),
                q1: eight(0.5),
                q2: eight(0.5),
            };
            let points = (0..f.per_chart)
                .map(|_| [rng.uniform(0.0, 1.0), rng.uniform(0.0, 1.0)])
                .collect();
            (chart, points)
        })
        .collect()
}

/// `chart` with `by` added to component `component` of `z₀`, at f64: the perturbed reference (pitfalls §3).
fn perturbed(chart: &Latent<f32>, p: Option<Perturb>) -> Latent<f64> {
    let mut wide = Latent {
        z0: chart.z0.map(f64::from),
        q1: chart.q1.map(f64::from),
        q2: chart.q2.map(f64::from),
    };
    if let Some(p) = p {
        wide.z0[p.component] += p.by;
    }
    wide
}

/// The GPU's decodes of a list of samples and the f64 decodes of the same samples, in order.
pub type Decodes = (Vec<DecodeOut<f32>>, Vec<DecodeOut<f64>>);

/// The GPU's and the f64 decodes of `batches` under the defaults, the f64 reference's `z₀` perturbed by `perturb`.
pub fn decode_both(
    h: &GpuHarness,
    wgsl: &str,
    batches: &[Batch],
    perturb: Option<Perturb>,
) -> Result<Decodes, String> {
    let stage = DecodeStage::<LatentDefaults>::new(h.device(), wgsl, FastMath::default())
        .map_err(|e| e.to_string())?;
    let key = physics_key();
    let mut gpu = Vec::new();
    let mut cpu = Vec::new();
    for (chart, points) in batches {
        gpu.extend(
            stage
                .run(h.device(), h.queue(), chart, DecodeMode::Full, points)
                .map_err(|e| e.to_string())?,
        );
        let reference = perturbed(chart, perturb);
        for uv in points {
            let uv = uv.map(f64::from);
            cpu.push(
                decode_only::<_, <LatentDefaults as DecodeVariant>::Links>(&reference, uv, &key)
                    .map_err(|e| e.to_string())?,
            );
        }
    }
    Ok((gpu, cpu))
}

/// The decodes of `batches` under the defaults by the same source compiled natively at f32 (rustc): the f32
/// instantiation without the GPU, so the report separates f32's own rounding from the backend's.
pub fn decode_native_f32(batches: &[Batch]) -> Vec<DecodeOut<f32>> {
    batches
        .iter()
        .flat_map(|(chart, points)| {
            points.iter().map(move |uv| {
                sample::<_, <LatentDefaults as DecodeVariant>::Links, f32>(chart, uv[0], uv[1])
            })
        })
        .collect()
}

/// The sim key of the physics kernel, every other group empty or zero: what `decode_only` reads of it is the kernel
/// variant.
pub fn physics_key() -> engine::contract::sim_config::SimConfig {
    use engine::contract::sim_config::*;
    SimConfig {
        chart: Chart {},
        plane: Plane {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        slice: Slice {},
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links {},
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 0.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

/// Which points an input decodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Points {
    /// The golden IC, `z = 0`.
    Golden,
    /// A fuzzed interior.
    Interior(Fuzz),
}

/// One latent component of the f64 reference's `z₀` changed by `by`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Perturb {
    pub component: usize,
    pub by: f64,
}

/// A measurement made on an input, and where.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recorded {
    /// The machine and backend measured on.
    pub platform: String,
    /// The worst error, in f32 eps.
    pub max_eps: f64,
}

/// A `decode-f32-eps` input, `fixtures/gates/decode-f32-eps/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// What the input is.
    pub description: String,
    /// The points decoded.
    pub points: Points,
    /// A perturbation of the f64 reference, if any.
    #[serde(default)]
    pub perturb: Option<Perturb>,
    /// A measurement made on this input, if any.
    #[serde(default)]
    pub recorded: Option<Recorded>,
}

/// The WGSL `cargo xtask build-kernel` writes under `root`.
pub fn kernel_wgsl(root: &Path) -> Result<String, String> {
    let path = root.join("target/spirv/kernel.wgsl");
    std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "refused: cannot read {} ({e}); run `cargo xtask build-kernel` first",
            path.display()
        )
    })
}

/// The `decode-f32-eps` gate. It reads the kernel from the workspace root the runner is given ([`DecodeF32EpsGate::root`]).
pub struct DecodeF32EpsGate;

impl DecodeF32EpsGate {
    /// The workspace root: the directory above `crates/validation`.
    pub fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .to_path_buf()
    }

    /// Measures `input` on the GPU against the f64 decode, the kernel's WGSL `wgsl`, and judges it against `config`'s
    /// threshold, or refuses it.
    pub fn judge(
        config: &GateConfig,
        name: &str,
        input: &Input,
        wgsl: &str,
    ) -> Result<GateReport, String> {
        if let Some(p) = input.perturb {
            if p.component >= 8 || !p.by.is_finite() || p.by == 0.0 {
                return Err(format!(
                    "refused: the perturbation {p:?} names no latent component (0–7) or changes nothing"
                ));
            }
        }
        let batches = match input.points {
            Points::Golden => golden(),
            Points::Interior(f) if f.charts == 0 || f.per_chart == 0 => {
                return Err(format!("refused: the fuzz {f:?} draws no samples"));
            }
            Points::Interior(f) => interior(f),
        };
        let h = GpuHarness::new().map_err(|e| format!("refused: {e}"))?;
        let (gpu, cpu) = decode_both(&h, wgsl, &batches, input.perturb)?;
        let m = measure(&gpu, &cpu)?;
        let native = measure(&decode_native_f32(&batches), &cpu)?;
        let threshold = config.threshold.value;
        let (max, k) = m.max();
        let (native_max, native_k) = native.max();
        let mut scatter = vec![format!(
            "adapter: {}; domain: the decode alone, its inputs held fixed, no trajectory (pitfalls §10)",
            h.adapter_info()
        )];
        let mut sorted = m.per_sample.clone();
        sorted.sort_by(f64::total_cmp);
        let rank =
            |q: f64| sorted[((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len()) - 1];
        scatter.push(format!(
            "worst {max:.2} eps on {} at sample {}; per-sample worst: median {:.2}, p99 {:.2}, over {} samples",
            QUANTITIES[k],
            m.worst[k].at,
            rank(0.5),
            rank(0.99),
            m.samples
        ));
        scatter.push(format!(
            "the same source at f32 natively (rustc), against the same f64 decodes: worst {native_max:.2} eps on {} at \
             sample {}, f32's own rounding with no GPU in it",
            QUANTITIES[native_k], native.worst[native_k].at
        ));
        for ((q, w), n) in QUANTITIES.iter().zip(&m.worst).zip(&native.worst) {
            scatter.push(format!(
                "{q}: {:.2} eps (sample {}); natively at f32 {:.2} eps",
                w.eps, w.at, n.eps
            ));
        }
        let over = m.per_sample.iter().filter(|&&e| e > threshold).count();
        let pass = max <= threshold;
        let mut negative = vec![if pass {
            format!(
                "0 of {} samples beyond {threshold} eps; margin {:.1}× over the worst",
                m.samples,
                threshold / max.max(f64::MIN_POSITIVE)
            )
        } else {
            format!(
                "{over} of {} samples beyond {threshold} eps; the worst, {max:.2} eps on {}, fails",
                m.samples, QUANTITIES[k]
            )
        }];
        negative.push("every pair's DEGENERATE tag is the same on the GPU and at f64".to_owned());
        if let Some(p) = input.perturb {
            negative.push(format!(
                "measured against an f64 reference with z₀[{}] changed by {:e} (pitfalls §3: a decode not the \
                 kernel's must fail the gate)",
                p.component, p.by
            ));
        }
        if let Some(r) = &input.recorded {
            scatter.push(format!(
                "recorded on {}: worst {} eps; measured here: {max:.2} eps",
                r.platform, r.max_eps
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

impl Gate for DecodeF32EpsGate {
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
            .map_err(|e| format!("refused: {name} is not a decode-f32-eps input: {e}"))?;
        let wgsl = kernel_wgsl(&Self::root())?;
        Self::judge(config, name, &input, &wgsl)
    }
}
