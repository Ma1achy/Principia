//! Decoder test 12 across backends (dd_decoder §2, §5 test 12; parity_contract Tier N; R-34, R-84, R-85; TASK-M2-24):
//! the decode stage's GPU-f32 instantiation, `target/spirv/kernel.wgsl` dispatched natively (`DecodeStage::run_with`),
//! against the same source at f64 on the CPU (`decode_only_with`), over every compute variant the stage is built for,
//! on identical inputs: the same f32 `(u, v)`, chart parameters and `E*`, the CPU's widened, never narrowed (PIT-10).
//! CI runs this file in the `gpu-kernel` job (lavapipe) and on Metal (`gpu-metal`'s `decode_` filter).
//! - `decode_cross_backend_registry`: the charts this file decodes on the GPU are every chart of the registry
//!   (`kernel::chart::REGISTRY`), through every decode entry point the kernel's WGSL holds.
//! - `decode_cross_backend_continuous`: the 27 continuous values within REQ-DEC-043's factor (`decode-f32-eps`'s
//!   threshold), `η_E` off and on, inside its domain; outside it (saturated `stick_breaking`, the invariant charts' edge
//!   columns and the points past their square, the shape sphere's `n₀ = ±1` poles) measured and reported, as
//!   REQ-DEC-043's proposal records (`validation::decode_f32_eps`).
//! - `decode_cross_backend_tags`: on each backend each tag is what dd_decoder §2's branch inputs, read from that
//!   backend's own emitted values, call for; across backends the tags are bitwise equal wherever those inputs are, and
//!   the construction's tag (`INVARIANT_INFEASIBLE`), decided on the shared `(u, v)` and parameters, everywhere, the
//!   invariant charts' edge columns `s ∈ {0, 1}` included (the GPU's `sqrt` there, TASK-M2-12's note), and past the
//!   square in `s`, where every pixel is `INVARIANT_INFEASIBLE`, on both backends.
//! - `decode_cross_backend_energy`: `η_E` on, as a uniform: rest starts at `E* > V₀` (`ENERGY_UNSCALABLE`), targets
//!   `E* < V₀` (`ENERGY_INFEASIBLE`) and `E* = V₀` (untouched, untagged), each on each backend, its tag dd_decoder
//!   §3.7's from that backend's own `V₀`, `E*`, `s` and `r_min_pair_0`; a disagreement between backends only where those
//!   differ (precision, as §3.1 reads `M₀₁`); and a chart that forbids `η_E` ignoring the target on the GPU as on the
//!   CPU.
//! - `decode_parity_m01_crossings`: test 12 at `M01_TINY`'s onset crossings (TASK-M2-02's 40 000 points, one
//!   dispatch; TASK-M2-48, REQ-DEC-011, REQ-VAL-017): the GPU's tag is the CPU form's (R-547) on the GPU's own emitted
//!   masses at every point, and the native f32 decode's wherever the two emitted the same masses bit for bit.
//! - `decode_parity_superseded`: REQ-VAL-017's control on those inputs: the superseded `fma(m₁, 1, m₀)`, its `×1`
//!   folded and its add contracted, as a backend may evaluate it, gives a different tag from the block's on the CPU at
//!   f32 at some crossing; unfused, it is the one rounded sum and flips none (RQ-621).
//!
//! The controls contaminate one branch input (PIT-9): the GPU's emitted `V₀` one float up, or its `M₀₁` read from a
//! perturbed mass, so the bitwise comparison of a tag against its recomputation fails.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed, on this machine's GPU backend; no trajectory.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{decode_only_with, DecodeStage, DESCRIBE_ENTRY};
use kernel::chart::invariant::{Invariant, InvariantChart};
use kernel::chart::mixed::{Alpha, ZMu1};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, EuclidPlane, InvariantLE,
    InvariantLK, Latent, MassSimplex, Mixed, ShapeMap, ShapeSphere, REGISTRY,
};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::energy_norm::largest;
use kernel::decode::invariant::infeasible;
use kernel::decode::mass::{self, from_masses};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Link, LinkReal, Literals, StickBreaking};
use kernel::payload::PayloadReal;
use kernel::stage::decode::{
    BurrauDefaults, BurrauThetaDeltaMDefaults, BurrauThetaKDefaults, BurrauThetaLzDefaults,
    DecodeMode, DecodeOut, DecodeVariant, EuclidPlaneDefaults, InvariantLEDefaults,
    InvariantLKDefaults, LatentDefaults, LatentStickBreaking, MassSimplexDefaults,
    MixedMu1AlphaDefaults, ShapeSphereDefaults, UniformChart,
};
use validation::decode_f32_eps::{
    error, interior, physics_key, values, Fuzz, SplitMix, QUANTITIES,
};
use validation::gpu::GpuHarness;
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The WGSL `cargo xtask build-kernel` wrote.
fn wgsl() -> String {
    std::fs::read_to_string(root().join("target/spirv/kernel.wgsl"))
        .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

/// REQ-DEC-043's factor, in f32 eps: `decode-f32-eps`'s threshold (`fixtures/gates/decode-f32-eps/gate.json`).
fn factor() -> f64 {
    validation::gate::config(&root(), "decode-f32-eps")
        .expect("decode-f32-eps's gate.json")
        .threshold
        .value
}

// ── The branch inputs, per backend ──────────────────────────────────────────────────────────────────────────────

/// A precision a backend decodes at.
trait Q: LinkReal + PayloadReal + Into<f64> {}
impl Q for f32 {}
impl Q for f64 {}

/// `M01_TINY` as the mass block forms it from the emitted masses, the CPU form at `R` (`kernel::decode::mass`; R-547):
/// `ε_M01 > M₀₁`, `M₀₁` the one rounded sum `m₀ + m₁` in `Wide` arithmetic. Applied to a backend's own emitted masses,
/// so a tag is compared on identical inputs (REQ-VAL-079; dd_decoder §3.1, "Its premise").
fn m01_tag<R: Q>(m: [R; 3]) -> Option<Degenerate> {
    from_masses(m).degenerate
}

/// `η_E`'s tag (dd_decoder §3.7) from the emitted `V₀` and `r_min_pair_0`, the momenta's `s` before the rescale and the
/// target, each a comparison of emitted values.
fn energy_tag<R: Q>(v0: R, r_min: R, s: R, e_star: R) -> Option<Degenerate> {
    let zero = R::zero();
    if r_min == zero {
        Some(Degenerate::EnergyUnscalable)
    } else if e_star < v0 {
        Some(Degenerate::EnergyInfeasible)
    } else if s == zero && e_star != v0 {
        Some(Degenerate::EnergyUnscalable)
    } else {
        None
    }
}

/// A chart the stage decodes, and the construction's tag at `(u, v)` at the precision `R`, recomputed on the CPU from
/// the chart's own targets by the exact test (chart_reference §2.2): none for a chart with no construction.
trait Case: UniformChart + Chart<Param = f32> + Copy {
    /// The chart with one parameter moved, the reference a control decodes on the CPU: the chart itself, but for the
    /// latent chart, whose `z₀`'s first component moves by `10⁻⁴`, as `decode-f32-eps`'s perturbed input does.
    fn perturbed(&self) -> Self {
        *self
    }

    fn construction<R: Q + kernel::chart::invariant::WarpReal + From<f32>>(
        &self,
        _u: R,
        _v: R,
    ) -> Option<Degenerate> {
        None
    }
}

impl Case for Latent<f32> {
    fn perturbed(&self) -> Self {
        let mut z0 = self.z0;
        z0[0] += 1e-4;
        Latent { z0, ..*self }
    }
}
impl Case for MassSimplex<f32> {}
impl Case for EuclidPlane<f32> {}
impl Case for Mixed<ZMu1, Alpha, f32> {}
impl Case for BurrauThetaDeltaM<f32> {}
impl Case for ShapeSphere<f32> {}

fn short<R: Q + kernel::chart::invariant::WarpReal>(k: R, lz: R) -> Option<Degenerate> {
    infeasible(R::one(), k, lz).then_some(Degenerate::InvariantInfeasible)
}

impl Case for Burrau<f32> {
    fn construction<R: Q + kernel::chart::invariant::WarpReal + From<f32>>(
        &self,
        _u: R,
        v: R,
    ) -> Option<Degenerate> {
        short(self.kinetic(v), R::zero())
    }
}

impl Case for BurrauThetaK<f32> {
    fn construction<R: Q + kernel::chart::invariant::WarpReal + From<f32>>(
        &self,
        _u: R,
        v: R,
    ) -> Option<Degenerate> {
        short(self.kinetic(v), R::zero())
    }
}

impl Case for BurrauThetaLz<f32> {
    fn construction<R: Q + kernel::chart::invariant::WarpReal + From<f32>>(
        &self,
        _u: R,
        v: R,
    ) -> Option<Degenerate> {
        short(self.kinetic(), self.lz(v))
    }
}

impl Case for InvariantLE<f32> {
    fn construction<R: Q + kernel::chart::invariant::WarpReal + From<f32>>(
        &self,
        u: R,
        v: R,
    ) -> Option<Degenerate> {
        let t = self.invariant().targets(u, v);
        short(t.k, t.lz)
    }
}

impl Case for InvariantLK<f32> {
    fn construction<R: Q + kernel::chart::invariant::WarpReal + From<f32>>(
        &self,
        u: R,
        v: R,
    ) -> Option<Degenerate> {
        let t = self.invariant().targets(u, v);
        short(t.k, t.lz)
    }
}

/// The tag a backend's decode must carry: the mass block's from its emitted masses, then the construction's, then
/// `η_E`'s from its emitted `V₀`, `r_min_pair_0` and the `s` of its decode with `η_E` off (`off`).
fn expected<R: Q>(
    out: &DecodeOut<R>,
    off: &DecodeOut<R>,
    construction: Option<Degenerate>,
    e_star: Option<R>,
) -> Option<Degenerate> {
    let energy = e_star.and_then(|e| {
        energy_tag(
            out.descriptor.V_0,
            out.descriptor.r_min_pair_0,
            largest(&off.ic.p),
            e,
        )
    });
    m01_tag(out.ic.m).or(construction).or(energy)
}

// ── The cases ───────────────────────────────────────────────────────────────────────────────────────────────────

/// One chart and its points.
type Batch<C> = (C, Vec<[f32; 2]>);

/// A latent chart with `z₀` on `[−1, 1]`, `q₁`, `q₂` on `[−spread, spread]`, its momentum block zero where `rest`.
fn latent(rng: &mut SplitMix, spread: f32, rest: bool) -> Latent<f32> {
    let mut draw = |h: f32| -> [f32; 8] { core::array::from_fn(|_| rng.uniform(-h, h)) };
    let (mut z0, mut q1, mut q2) = (draw(1.0), draw(spread), draw(spread));
    if rest {
        for k in 2..6 {
            (z0[k], q1[k], q2[k]) = (0.0, 0.0, 0.0);
        }
    }
    Latent { z0, q1, q2 }
}

/// `n` points on `[0, 1]²` from `rng`, then the corners and the edge midpoints.
fn points(rng: &mut SplitMix, n: usize) -> Vec<[f32; 2]> {
    let mut out: Vec<[f32; 2]> = (0..n)
        .map(|_| [rng.uniform(0.0, 1.0), rng.uniform(0.0, 1.0)])
        .collect();
    for u in [0.0, 0.5, 1.0] {
        for v in [0.0, 0.5, 1.0] {
            out.push([u, v]);
        }
    }
    out
}

/// The defaults' latent batches: the golden IC, REQ-DEC-043's interior, and a rest start.
fn latent_defaults() -> Vec<Batch<Latent<f32>>> {
    let mut b = validation::decode_f32_eps::golden();
    b.extend(interior(Fuzz {
        seed: 24,
        charts: 8,
        per_chart: 64,
    }));
    let mut rng = SplitMix::new(124);
    b.push((latent(&mut rng, 0.5, true), points(&mut rng, 64)));
    b
}

/// `stick_breaking`'s batches: the interior, where `saturated` does not hold, else saturated planes, whose corner
/// reaches `M01_TINY`'s onset.
fn latent_stick(saturated: bool) -> Vec<Batch<Latent<f32>>> {
    let mut rng = SplitMix::new(224 + u64::from(saturated));
    let (spread, n) = if saturated { (20.0, 128) } else { (0.5, 64) };
    (0..4)
        .map(|_| (latent(&mut rng, spread, false), points(&mut rng, n)))
        .collect()
}

/// An invariant chart's frozen part from `rng`, under the register's warp.
fn frozen(rng: &mut SplitMix) -> Invariant<f32> {
    Invariant {
        z_cfg: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        z_mass: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    }
}

/// The invariant charts' edge columns `s ∈ {0, 1}`, at 33 values of `t` each and 16 drawn from `rng`.
fn edges(rng: &mut SplitMix) -> Vec<[f32; 2]> {
    let mut p = Vec::new();
    for s in [0.0, 1.0] {
        p.extend((0..=32).map(|j| [s, j as f32 / 32.0]));
        p.extend((0..16).map(|_| [s, rng.uniform(0.0, 1.0)]));
    }
    p
}

/// Points past the invariant charts' square in `s`, where `K* < K_min` and every pixel is `INVARIANT_INFEASIBLE`
/// (chart_reference §2.1, §5.4): `s ∈ {−0.5, −0.25, 1.25, 1.5}` at 17 values of `t`, and 32 more with `s` drawn from
/// `[−0.75, 0)` or `(1, 1.75]`.
fn past(rng: &mut SplitMix) -> Vec<[f32; 2]> {
    let mut p = Vec::new();
    for s in [-0.5, -0.25, 1.25, 1.5] {
        p.extend((0..=16).map(|j| [s, j as f32 / 16.0]));
    }
    for k in 0..32 {
        let s = if k % 2 == 0 {
            rng.uniform(-0.75, -0.01)
        } else {
            rng.uniform(1.01, 1.75)
        };
        p.push([s, rng.uniform(0.0, 1.0)]);
    }
    p
}

/// Each of `charts` with the points `draw` gives.
fn with<C: Copy>(
    charts: &[C],
    rng: &mut SplitMix,
    draw: fn(&mut SplitMix) -> Vec<[f32; 2]>,
) -> Vec<Batch<C>> {
    charts.iter().map(|&c| (c, draw(rng))).collect()
}

/// `n` points inside `(0, 1)²` from `rng`, off the edges.
fn inside(rng: &mut SplitMix, n: usize) -> Vec<[f32; 2]> {
    (0..n)
        .map(|_| [rng.uniform(0.01, 0.99), rng.uniform(0.0, 1.0)])
        .collect()
}

fn simplex() -> MassSimplex<f32> {
    MassSimplex {
        nu0: 0.5,
        eps_m: validation::MASS_SIMPLEX_EPS_M.number() as f32,
    }
}

fn burrau() -> Burrau<f32> {
    Burrau {
        nu_range: [0.05, 0.95],
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    }
}

fn plane() -> EuclidPlane<f32> {
    EuclidPlane {
        m_range: [EUCLID_M_MIN.number() as f32, EUCLID_M_MAX.number() as f32],
        nu_range: [EUCLID_NU_MIN.number() as f32, EUCLID_NU_MAX.number() as f32],
    }
}

fn theta_range() -> [f32; 2] {
    [
        BURRAU_THETA_MIN.number() as f32,
        BURRAU_THETA_MAX.number() as f32,
    ]
}

fn strip_k() -> BurrauThetaK<f32> {
    BurrauThetaK {
        theta_range: theta_range(),
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    }
}

fn strip_lz() -> BurrauThetaLz<f32> {
    BurrauThetaLz {
        theta_range: theta_range(),
        k: BURRAU_THETA_LZ_K.number() as f32,
        lz_fraction: BURRAU_THETA_LZ_FRACTION.number() as f32,
    }
}

/// The `(θ, Δm)` strip toward a target whose sum is 1 exactly at f32 and f64.
fn strip_m() -> BurrauThetaDeltaM<f32> {
    BurrauThetaDeltaM {
        theta_range: theta_range(),
        m_target: [0.25, 0.375, 0.375],
    }
}

/// The shape sphere under `map`, its held blocks from `rng`, the exponential map about the `+w` pole.
fn sphere(rng: &mut SplitMix, map: ShapeMap) -> ShapeSphere<f32> {
    ShapeSphere {
        map,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.2,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [rng.uniform(-1.0, 1.0), rng.uniform(-1.0, 1.0)],
        z_mom: core::array::from_fn(|_| rng.uniform(-1.0, 1.0)),
    }
}

fn mixed(rng: &mut SplitMix) -> Mixed<ZMu1, Alpha, f32> {
    Mixed::new(
        core::array::from_fn(|_| rng.uniform(-2.0, 2.0)),
        [-2.0, 2.0],
        [0.1, 1.4],
    )
}

// ── The runner ──────────────────────────────────────────────────────────────────────────────────────────────────

/// What a run checks: the decoders as they are, or with a control's contamination (PIT-9).
#[derive(Clone, Copy)]
struct Taint {
    /// Applied to the GPU's decode before its tag is recomputed from it: a contaminated branch input.
    gpu: fn(&mut DecodeOut<f32>),
    /// The f64 reference decodes [`Case::perturbed`]: a decode that is not the kernel's.
    reference: bool,
}

/// The GPU's decode as it is.
fn clean(_: &mut DecodeOut<f32>) {}

const CLEAN: Taint = Taint {
    gpu: clean,
    reference: false,
};

/// What the runs found, over every variant.
struct Report {
    taint: Taint,
    /// The worst continuous error in f32 eps, its quantity and where, by the runs' domain ([`Report::domain`]).
    worst: BTreeMap<String, (f64, usize, String)>,
    /// The worst difference of the GPU's from the native f32 decode, in f32 eps, by domain: the backend's own share.
    backend: BTreeMap<String, (f64, usize, String)>,
    /// The domain the samples now compared belong to: a variant and a batch kind.
    domain: String,
    /// Samples whose tags differ across backends, by precision (the deciding inputs differ).
    precision: usize,
    /// Each backend's tags met, by name, `None` as "none".
    gpu_tags: BTreeSet<String>,
    cpu_tags: BTreeSet<String>,
    /// The energy cases met on each backend: a rest start tagged `ENERGY_UNSCALABLE`, `ENERGY_INFEASIBLE`, and a rest
    /// start at `E* = V₀` left untouched and untagged.
    unscalable_rest: [usize; 2],
    infeasible: [usize; 2],
    at_target: [usize; 2],
    samples: usize,
    /// The entry points run and the charts they decode, by name.
    entries: BTreeSet<String>,
    charts: BTreeSet<String>,
}

impl Report {
    fn new(taint: Taint) -> Self {
        Report {
            taint,
            worst: BTreeMap::new(),
            backend: BTreeMap::new(),
            domain: String::new(),
            precision: 0,
            gpu_tags: BTreeSet::new(),
            cpu_tags: BTreeSet::new(),
            unscalable_rest: [0; 2],
            infeasible: [0; 2],
            at_target: [0; 2],
            samples: 0,
            entries: BTreeSet::new(),
            charts: BTreeSet::new(),
        }
    }
}

fn name(t: Option<Degenerate>) -> String {
    t.map_or("none".to_owned(), |d| d.name().to_owned())
}

/// One sample on both backends: each decode toward its target and with `η_E` off, the targets (the CPU's the GPU's
/// widened, unless the case is each backend's own `E* = V₀`), whether the two decoded the same input, and where.
struct Sample<'a> {
    uv: [f32; 2],
    gpu: (&'a DecodeOut<f32>, &'a DecodeOut<f32>),
    /// The same source's f32 decode, natively (rustc), toward the GPU's target: f32 without the backend.
    native: DecodeOut<f32>,
    cpu: (&'a DecodeOut<f64>, &'a DecodeOut<f64>),
    targets: (Option<f32>, Option<f64>),
    shared: bool,
    at: String,
}

/// One sample's comparison (the module's note).
fn compare<C: Case>(r: &mut Report, chart: &C, s: Sample) {
    let Sample {
        uv,
        gpu: (gpu, gpu_off),
        native,
        cpu: (cpu, cpu_off),
        targets: (gpu_e, cpu_e),
        shared,
        at,
    } = s;
    let at = at.as_str();
    let allows = !chart.forbids_energy_normalisation();
    let gpu_e = gpu_e.filter(|_| allows);
    let cpu_e = cpu_e.filter(|_| allows);
    let mut g = *gpu;
    (r.taint.gpu)(&mut g);
    let want_g = expected(&g, gpu_off, chart.construction(uv[0], uv[1]), gpu_e);
    let wide = uv.map(f64::from);
    let want_c = expected(cpu, cpu_off, chart.construction(wide[0], wide[1]), cpu_e);
    assert_eq!(
        gpu.degenerate, want_g,
        "the GPU's tag at {at}: not its own branch inputs'"
    );
    assert_eq!(
        cpu.degenerate, want_c,
        "the f64 decode's tag at {at}: not its own branch inputs'"
    );
    r.gpu_tags.insert(name(gpu.degenerate));
    r.cpu_tags.insert(name(cpu.degenerate));
    let w = |x: f32| f64::from(x);
    if gpu.degenerate != cpu.degenerate {
        let construction = |t: Option<Degenerate>| {
            matches!(
                t,
                Some(Degenerate::InvariantInfeasible | Degenerate::NoQualifyingSeed)
            )
        };
        assert!(
            !(construction(gpu.degenerate) || construction(cpu.degenerate)),
            "a fork at {at}: the construction's tag, decided on the shared (u, v) and parameters, is {} on the GPU and \
             {} on the CPU",
            name(gpu.degenerate),
            name(cpu.degenerate)
        );
        let masses = w(gpu.ic.m[0]) == cpu.ic.m[0] && w(gpu.ic.m[1]) == cpu.ic.m[1];
        let energy = w(gpu.descriptor.V_0) == cpu.descriptor.V_0
            && w(gpu.descriptor.r_min_pair_0) == cpu.descriptor.r_min_pair_0
            && w(largest(&gpu_off.ic.p)) == largest(&cpu_off.ic.p)
            && gpu_e.map(w) == cpu_e;
        assert!(
            !(masses && energy),
            "a fork at {at}: the tags are {} on the GPU and {} on the CPU on bit-identical branch inputs",
            name(gpu.degenerate),
            name(cpu.degenerate)
        );
        r.precision += 1;
    } else if shared {
        for (k, (a, b)) in values(gpu).into_iter().zip(values(cpu)).enumerate() {
            let e = error(a, b);
            let w = r
                .worst
                .entry(r.domain.clone())
                .or_insert((0.0, 0, String::new()));
            if e > w.0 || e.is_nan() {
                *w = (e, k, at.to_owned());
            }
        }
        let wide = |o: &DecodeOut<f32>| values(o);
        for (k, (a, b)) in wide(gpu).into_iter().zip(wide(&native)).enumerate() {
            let e = error(a, b);
            let w = r
                .backend
                .entry(r.domain.clone())
                .or_insert((0.0, 0, String::new()));
            if e > w.0 || e.is_nan() {
                *w = (e, k, at.to_owned());
            }
        }
    }
    let rest = [largest(&gpu_off.ic.p) == 0.0, largest(&cpu_off.ic.p) == 0.0];
    for (k, (tag, on)) in [
        (gpu.degenerate, gpu_e.is_some()),
        (cpu.degenerate, cpu_e.is_some()),
    ]
    .into_iter()
    .enumerate()
    {
        match tag {
            Some(Degenerate::EnergyUnscalable) if rest[k] => r.unscalable_rest[k] += 1,
            Some(Degenerate::EnergyInfeasible) => r.infeasible[k] += 1,
            None if on && rest[k] => r.at_target[k] += 1,
            _ => {}
        }
    }
    if rest[0] && gpu.degenerate.is_none() && gpu_e.is_some() {
        assert_eq!(
            gpu.ic, gpu_off.ic,
            "the GPU's untagged rest start at {at} moved"
        );
    }
    r.samples += 1;
}

/// Runs `batches` through `V` on `h` and on the CPU, `η_E` off and toward each of `targets`, and compares every sample.
/// Where `own` holds, each sample is also decoded toward each backend's own `V₀`, one dispatch a sample: the GPU's
/// rest start at `E* = V₀_GPU` and the CPU's at `E* = V₀_CPU`.
fn run<V>(
    h: &GpuHarness,
    wgsl: &str,
    r: &mut Report,
    (label, batches): (&str, &[Batch<V::Chart>]),
    targets: &[f32],
    own: bool,
) where
    V: DecodeVariant,
    V::Chart: Case,
{
    let stage = DecodeStage::<V>::new(h.device(), wgsl, FastMath::default())
        .expect("the decode stage's pipeline");
    let key = physics_key();
    let gpu_run = |chart: &V::Chart, uv: &[[f32; 2]], e: Option<f32>| {
        stage
            .run_with(h.device(), h.queue(), chart, DecodeMode::Full, e, uv)
            .expect("the decode dispatch")
    };
    let taint = r.taint;
    let cpu_run = |chart: &V::Chart, p: [f32; 2], e: Option<f64>| {
        let reference = if taint.reference {
            chart.perturbed()
        } else {
            *chart
        };
        decode_only_with::<_, V::Links>(&reference, p.map(f64::from), &key, e).expect("decode_only")
    };
    let native = |chart: &V::Chart, p: [f32; 2], e: Option<f32>| {
        kernel::stage::decode::sample_with::<V::Chart, V::Links, f32>(chart, p[0], p[1], e)
    };
    r.entries.insert(V::ENTRY.to_owned());
    for (n, (chart, uv)) in batches.iter().enumerate() {
        r.charts.insert(chart.name().to_owned());
        let gpu_off = gpu_run(chart, uv, None);
        let cpu_off: Vec<_> = uv.iter().map(|&p| cpu_run(chart, p, None)).collect();
        r.domain = format!("{} {label}, η_E off", V::ENTRY);
        for (i, &p) in uv.iter().enumerate() {
            let sample = Sample {
                uv: p,
                gpu: (&gpu_off[i], &gpu_off[i]),
                native: native(chart, p, None),
                cpu: (&cpu_off[i], &cpu_off[i]),
                targets: (None, None),
                shared: true,
                at: format!(
                    "{} batch {n} sample {i} ({}, {}) η_E off",
                    V::ENTRY,
                    p[0],
                    p[1]
                ),
            };
            compare(r, chart, sample);
        }
        r.domain = format!("{} {label}, η_E on", V::ENTRY);
        for &e in targets {
            let gpu = gpu_run(chart, uv, Some(e));
            for (i, &p) in uv.iter().enumerate() {
                let cpu = cpu_run(chart, p, Some(f64::from(e)));
                let sample = Sample {
                    uv: p,
                    gpu: (&gpu[i], &gpu_off[i]),
                    native: native(chart, p, Some(e)),
                    cpu: (&cpu, &cpu_off[i]),
                    targets: (Some(e), Some(f64::from(e))),
                    shared: true,
                    at: format!(
                        "{} batch {n} sample {i} ({}, {}) E* {e}",
                        V::ENTRY,
                        p[0],
                        p[1]
                    ),
                };
                compare(r, chart, sample);
            }
        }
        if own {
            for (i, &p) in uv.iter().enumerate().step_by(8) {
                let (g, c) = (gpu_off[i].descriptor.V_0, cpu_off[i].descriptor.V_0);
                if !(g.is_finite() && c.is_finite()) {
                    continue;
                }
                let gpu = gpu_run(chart, &[p], Some(g));
                let cpu = cpu_run(chart, p, Some(c));
                let sample = Sample {
                    uv: p,
                    gpu: (&gpu[0], &gpu_off[i]),
                    native: native(chart, p, Some(g)),
                    cpu: (&cpu, &cpu_off[i]),
                    targets: (Some(g), Some(c)),
                    shared: false,
                    at: format!(
                        "{} batch {n} sample {i} ({}, {}) E* = own V₀",
                        V::ENTRY,
                        p[0],
                        p[1]
                    ),
                };
                compare(r, chart, sample);
            }
        }
    }
}

/// The targets of the `η_E` runs: a real zero, a bound target above most interior `V₀`, one below them all, and an
/// unbound one.
const TARGETS: [f32; 4] = [0.0, -0.3, -40.0, 1.5];

/// Every variant, `η_E` off; and, where `energy` holds, toward [`TARGETS`] and each backend's own `V₀` on the charts that
/// allow it, and toward the targets on two that forbid it, which must ignore them.
fn run_all(taint: Taint, energy: bool) -> Report {
    let h = GpuHarness::new().expect("a GPU device");
    let wgsl = wgsl();
    let mut r = Report::new(taint);
    let t: &[f32] = if energy { &TARGETS } else { &[] };
    let mut rng = SplitMix::new(324);
    run::<LatentDefaults>(
        &h,
        &wgsl,
        &mut r,
        ("defaults", &latent_defaults()),
        t,
        energy,
    );
    run::<LatentStickBreaking>(
        &h,
        &wgsl,
        &mut r,
        ("interior", &latent_stick(false)),
        t,
        false,
    );
    run::<LatentStickBreaking>(
        &h,
        &wgsl,
        &mut r,
        (SATURATED, &latent_stick(true)),
        t,
        false,
    );
    let le: Vec<_> = (0..3).map(|_| InvariantLE(frozen(&mut rng))).collect();
    let lk: Vec<_> = (0..3).map(|_| InvariantLK(frozen(&mut rng))).collect();
    run::<InvariantLEDefaults>(
        &h,
        &wgsl,
        &mut r,
        ("fuzz", &with(&le, &mut rng, |g| inside(g, 64))),
        t,
        false,
    );
    run::<InvariantLKDefaults>(
        &h,
        &wgsl,
        &mut r,
        ("fuzz", &with(&lk, &mut rng, |g| inside(g, 64))),
        t,
        false,
    );
    run::<InvariantLEDefaults>(
        &h,
        &wgsl,
        &mut r,
        (EDGES, &with(&le, &mut rng, edges)),
        t,
        false,
    );
    run::<InvariantLKDefaults>(
        &h,
        &wgsl,
        &mut r,
        (EDGES, &with(&lk, &mut rng, edges)),
        t,
        false,
    );
    run::<InvariantLEDefaults>(
        &h,
        &wgsl,
        &mut r,
        (PAST, &with(&le, &mut rng, past)),
        t,
        false,
    );
    run::<InvariantLKDefaults>(
        &h,
        &wgsl,
        &mut r,
        (PAST, &with(&lk, &mut rng, past)),
        t,
        false,
    );
    let one = |c, n| vec![(c, points(&mut SplitMix::new(n), 128))];
    run::<MassSimplexDefaults>(&h, &wgsl, &mut r, ("fuzz", &one(simplex(), 1)), t, energy);
    run::<BurrauDefaults>(
        &h,
        &wgsl,
        &mut r,
        ("fuzz", &[(burrau(), points(&mut rng, 128))]),
        t,
        false,
    );
    run::<EuclidPlaneDefaults>(
        &h,
        &wgsl,
        &mut r,
        ("fuzz", &[(plane(), points(&mut rng, 128))]),
        t,
        energy,
    );
    let mx: Vec<_> = (0..3)
        .map(|_| (mixed(&mut rng), points(&mut rng, 64)))
        .collect();
    run::<MixedMu1AlphaDefaults>(&h, &wgsl, &mut r, ("fuzz", &mx), t, false);
    run::<BurrauThetaKDefaults>(
        &h,
        &wgsl,
        &mut r,
        ("fuzz", &[(strip_k(), points(&mut rng, 128))]),
        t,
        false,
    );
    run::<BurrauThetaLzDefaults>(
        &h,
        &wgsl,
        &mut r,
        ("fuzz", &[(strip_lz(), points(&mut rng, 128))]),
        t,
        false,
    );
    run::<BurrauThetaDeltaMDefaults>(
        &h,
        &wgsl,
        &mut r,
        ("fuzz", &[(strip_m(), points(&mut rng, 128))]),
        t,
        energy,
    );
    let spheres: Vec<_> = [ShapeMap::Spherical, ShapeMap::Exponential]
        .into_iter()
        .flat_map(|map| [0, 1].map(|_| map))
        .map(|map| {
            let uv = points(&mut rng, 64)
                .into_iter()
                .filter(|p| !POLES.contains(p));
            (sphere(&mut rng, map), uv.collect())
        })
        .collect();
    run::<ShapeSphereDefaults>(&h, &wgsl, &mut r, ("fuzz", &spheres), t, energy);
    let pole = [(sphere(&mut rng, ShapeMap::Spherical), POLES.to_vec())];
    run::<ShapeSphereDefaults>(&h, &wgsl, &mut r, (POLE, &pole), t, energy);
    for (domain, (e, k, at)) in &r.worst {
        let (b, bk, _) = &r.backend[domain];
        eprintln!(
            "decode_cross_backend: {domain}: worst {e:.2} eps against f64, {} at {at}; {b:.2} eps against native f32 ({})",
            QUANTITIES[*k],
            QUANTITIES[*bk]
        );
    }
    eprintln!(
        "decode_cross_backend: {} samples on {}; {} precision disagreements; GPU tags {:?}, CPU tags {:?}; rest \
         unscalable {:?}, infeasible {:?}, at target {:?}",
        r.samples,
        h.adapter_info().name,
        r.precision,
        r.gpu_tags,
        r.cpu_tags,
        r.unscalable_rest,
        r.infeasible,
        r.at_target
    );
    r
}

// ── decode_cross_backend_registry ───────────────────────────────────────────────────────────────────────────────

/// Every compute entry point of the decode stage in `wgsl`, `stage_decode_…`, but the descriptor writer and the `tanh`
/// operator, which decode nothing.
fn entries(wgsl: &str) -> BTreeSet<String> {
    let lines: Vec<&str> = wgsl.lines().collect();
    lines
        .windows(2)
        .filter(|w| w[0].trim().starts_with("@compute"))
        .filter_map(|w| w[1].trim().strip_prefix("fn "))
        .map(|l| l.split('(').next().unwrap_or("").to_owned())
        .filter(|n| {
            n.starts_with("stage_decode_") && n != DESCRIBE_ENTRY && n != "stage_decode_link_tanh"
        })
        .collect()
}

/// The runs decoded every chart of the registry (`kernel::chart::REGISTRY`) on the GPU, through every decode entry
/// point the kernel's WGSL holds.
fn check_registry(drop: Option<&str>) {
    let mut r = run_all(CLEAN, false);
    if let Some(chart) = drop {
        r.charts.remove(chart);
    }
    let registry: BTreeSet<String> = REGISTRY.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        r.charts, registry,
        "the charts run on the GPU are not every chart of the registry"
    );
    let want = entries(&wgsl());
    assert!(!want.is_empty(), "the WGSL names no decode entry point");
    assert_eq!(
        r.entries, want,
        "the variants run are not every decode entry point"
    );
}

#[test]
fn decode_cross_backend_registry() {
    check_registry(None);
}

negative_control!(
    decode_cross_backend_registry,
    "a run that leaves out the shape sphere must fail",
    expected = "the charts run on the GPU are not every chart of the registry",
    check_registry(Some("ShapeSphere"))
);

// ── decode_cross_backend_continuous ─────────────────────────────────────────────────────────────────────────────

/// The batch kind outside REQ-DEC-043's domain: `stick_breaking` at saturated controls, f32's own conditioning
/// (`validation::decode_f32_eps`'s note: 2.53·10⁶ eps against f64, 16.5 against the native f32 decode).
const SATURATED: &str = "saturated";

/// The invariant charts' edge columns, outside REQ-DEC-043's domain: there `K* − L_z²/2` is the rounding of the
/// stepped-down `L_max` (TASK-M2-12), so the mixing amplitude `a = √(2(K* − L_z²/2))` is f32's or f64's own rounding,
/// `√eps`-sized and different at each precision, and the momenta differ by `a·w` (f32's conditioning, not the backend).
const EDGES: &str = "edge columns";

/// The invariant charts' points past the square in `s` ([`past`]), outside REQ-DEC-043's domain: every one is
/// `INVARIANT_INFEASIBLE`, its momenta the rigid rotation, and its continuous values are measured and reported.
const PAST: &str = "past the square";

/// The spherical map's points of `n₀ = ±1` in `[0,1]²`, the inverse's two poles (chart_reference §3.2). At `(½, ½)`,
/// `θ = π` and `φ = π/2`, so `n₀ = −1` and `ρ̃ = √(I(1 + n₀)/2) = 0`: the binary collision `BC₀₁`, where `V₀ → −∞`
/// (R-436). At `(0, ½)` and `(1, ½)`, `n₀ = +1` and `λ̃ = 0`, where `rho_angle` is R-437's defined `atan2` pole. Each
/// precision reaches them only to its own rounding of `sin π` and `cos(π/2)`, so the vanishing vector is that rounding,
/// f32's or f64's, and `1/ρ` or the angle of a rounding-sized `λ̃` amplifies it.
const POLES: [[f32; 2]; 3] = [[0.5, 0.5], [0.0, 0.5], [1.0, 0.5]];

/// The shape sphere at [`POLES`], outside REQ-DEC-043's domain: its continuous values are measured and reported.
const POLE: &str = "n₀ = ±1 poles";

/// Whether `domain` lies outside REQ-DEC-043's: [`SATURATED`], [`EDGES`], [`PAST`] or [`POLE`].
fn outside(domain: &str) -> bool {
    [SATURATED, EDGES, PAST, POLE]
        .iter()
        .any(|d| domain.contains(d))
}

/// Inside REQ-DEC-043's domain, every continuous value within its factor of the f64 decode. Outside it, the
/// differences are measured and reported, against f64 and against the native f32 decode, not held to the factor: there
/// a value's rounding at f32 is amplified, so a one-ulp difference in a backend's `sqrt` moves it by more than any
/// fixed multiple of `eps` (REQ-DEC-043's note: "a factor for saturated controls … would be measured on its own
/// domain").
fn check_continuous(taint: Taint) {
    let r = run_all(taint, true);
    let f = factor();
    for (domain, (e, k, at)) in r.worst.iter().filter(|(d, _)| !outside(d)) {
        assert!(
            *e <= f,
            "{domain}: {} is {e:.2} eps from the f64 decode at {at}, over REQ-DEC-043's {f}",
            QUANTITIES[*k]
        );
    }
}

#[test]
fn decode_cross_backend_continuous() {
    check_continuous(CLEAN);
}

negative_control!(
    decode_cross_backend_continuous,
    "an f64 reference whose z₀ is moved by 10⁻⁴, a decode not the kernel's, must fail the factor",
    expected = "over REQ-DEC-043's",
    check_continuous(Taint {
        reference: true,
        ..CLEAN
    })
);

// ── decode_cross_backend_tags ───────────────────────────────────────────────────────────────────────────────────

fn check_tags(taint: Taint) {
    let r = run_all(taint, false);
    for tag in ["M01_TINY", "INVARIANT_INFEASIBLE"] {
        assert!(
            r.gpu_tags.contains(tag) && r.cpu_tags.contains(tag),
            "the fuzz never reached {tag} on both backends: GPU {:?}, CPU {:?}",
            r.gpu_tags,
            r.cpu_tags
        );
    }
}

#[test]
fn decode_cross_backend_tags() {
    check_tags(CLEAN);
}

negative_control!(
    decode_cross_backend_tags,
    "an M₀₁ recomputed from a contaminated mass must fail the bitwise tag comparison",
    expected = "the GPU's tag at",
    check_tags(Taint {
        gpu: |g| g.ic.m[0] += <f32 as ChartConstants>::EPS_M01,
        ..CLEAN
    })
);

// ── decode_cross_backend_energy ─────────────────────────────────────────────────────────────────────────────────

fn check_energy(taint: Taint) {
    let r = run_all(taint, true);
    for k in 0..2 {
        let backend = ["GPU", "CPU"][k];
        assert!(
            r.unscalable_rest[k] > 0,
            "{backend}: no rest start at E* > V₀ tagged ENERGY_UNSCALABLE"
        );
        assert!(
            r.infeasible[k] > 0,
            "{backend}: no E* < V₀ tagged ENERGY_INFEASIBLE"
        );
        assert!(
            r.at_target[k] > 0,
            "{backend}: no rest start at E* = V₀ left untagged"
        );
    }
}

#[test]
fn decode_cross_backend_energy() {
    check_energy(CLEAN);
}

negative_control!(
    decode_cross_backend_energy,
    "a V₀ one float up must fail the bitwise comparison of the energy tag",
    expected = "the GPU's tag at",
    check_energy(Taint {
        gpu: |g| g.descriptor.V_0 = f32::from_bits(g.descriptor.V_0.to_bits() + 1),
        ..CLEAN
    })
);

// ── decode_parity: M₀₁ at the onset crossings ───────────────────────────────────────────────────────────────────

/// The lines `z₁ ∈ [6.3, 20.3]` the onset crossings are bisected on (TASK-M2-02; `kernel`'s `decode_blocks`).
const LINES: u32 = 20_000;

/// `stick_breaking`'s forward at f32 in the generated op order (`kernel::generated::links`), stopped before its last
/// multiply: `(m₀, t₇, t₁₃)`, with `m₁ = t₇·t₁₃`. Checked against the generated forward bit for bit where it is used.
fn stick_parts(z: [f32; 2]) -> (f32, f32, f32) {
    let sigmoid = |x: f32| 1.0 / (1.0 + (-x).exp());
    let t0 = f32::LIT_3 * z[0];
    let t4 = f32::LIT_1 * f32::EPS_MU * z[0].tanh();
    let t8 = f32::LIT_3 * z[1];
    let t12 = f32::LIT_1 * f32::EPS_MU * z[1].tanh();
    (sigmoid(-t0) + t4, sigmoid(t0) - t4, sigmoid(-t8) + t12)
}

/// `M01_TINY`'s onset crossings at f32 (TASK-M2-02): on each of [`LINES`] lines `z₁ ∈ [6.3, 20.3]`, `z₀ ∈ [0, 30]`
/// bisected to the adjacent pair where the CPU's emitted masses' one rounded sum crosses `ε_M01`, where one rounding
/// decides the tag. Both points of every pair.
fn crossings() -> Vec<[f32; 2]> {
    let eps = mass::m01_epsilon::<f32>();
    let mut out = Vec::with_capacity(2 * LINES as usize);
    for k in 0..LINES {
        let z1 = (6.3 + 14.0 * f64::from(k) / f64::from(LINES)) as f32;
        let sum_tag = |z0: f32| {
            let m = <StickBreaking as Link<2, 3>>::forward([z0, z1]);
            m[0] + m[1] < eps
        };
        let (mut lo, mut hi) = (0.0f32, 30.0f32);
        assert!(
            !sum_tag(lo) && sum_tag(hi),
            "no onset crossing on z₁ = {z1}"
        );
        loop {
            let mid = (lo + hi) * 0.5;
            if mid == lo || mid == hi {
                break;
            }
            if sum_tag(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        out.push([lo, z1]);
        out.push([hi, z1]);
    }
    out
}

/// The latent chart that places `z_μ = (2u, 2v)` exactly, at f32 and f64 and fused or not: `z₀ = e₆ + e₇`, `q₁ = e₆`,
/// `q₂ = e₇`, so `z₆ = 1 + (2u − 1)·1 + (2v − 1)·0`, and for `2u ≥ 2`, `2u − 1` and `1 + (2u − 1)` are exact (the
/// products by 1 and 0 are exact, so a fused one rounds the same); likewise `z₇ = 2v`. The other controls are ±0.
fn crossing_chart() -> Latent<f32> {
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0 } else { 0.0 });
    let mut z0 = [0.0; 8];
    (z0[6], z0[7]) = (1.0, 1.0);
    Latent {
        z0,
        q1: e(6),
        q2: e(7),
    }
}

/// Test 12's batch at the onset crossings: [`crossing_chart`] at `(u, v) = z_μ/2` for every point of [`crossings`],
/// each checked to place its `z_μ` exactly (every crossing lies at `z ≥ 2`).
fn crossing_batch() -> Batch<Latent<f32>> {
    let chart = crossing_chart();
    let uv = crossings()
        .into_iter()
        .map(|z| {
            assert!(
                z[0] >= 2.0 && z[1] >= 2.0,
                "a crossing at {z:?} lies below 2, where the chart does not place it exactly"
            );
            let uv = [z[0] * 0.5, z[1] * 0.5];
            let placed = chart.map::<f32>(uv[0], uv[1]).z.z;
            assert_eq!(
                [placed[6], placed[7]],
                z,
                "the chart does not place the crossing {z:?} exactly"
            );
            uv
        })
        .collect();
    (chart, uv)
}

/// What the crossings dispatch found.
struct Crossings {
    /// Points whose GPU masses are bit for bit the native f32 decode's, and how many of them each tag.
    identical: usize,
    tiny: usize,
    clear: usize,
}

/// Decoder test 4's cross-backend half and test 12 at the onset crossings (REQ-DEC-011, REQ-VAL-017): the 40 000
/// points of [`crossing_batch`] in one dispatch through `stick_breaking`'s latent entry point. At every point the GPU's
/// tag is the CPU form's applied to the GPU's own emitted masses ([`m01_tag`], after `taint`): the two compared on
/// identical inputs, so a one-ulp difference in the GPU's `tanh` or `exp` reads as precision, not as a fork
/// (REQ-VAL-079; dd_decoder §3.1, "Its premise"). Where the GPU's masses are the native f32 decode's bit for bit, its
/// tag is the native decode's. Both tags are reached.
fn check_crossings(taint: fn(&mut DecodeOut<f32>)) -> Crossings {
    let h = GpuHarness::new().expect("a GPU device");
    let stage = DecodeStage::<LatentStickBreaking>::new(h.device(), &wgsl(), FastMath::default())
        .expect("the decode stage's pipeline");
    let (chart, uv) = crossing_batch();
    let gpu = stage
        .run_with(h.device(), h.queue(), &chart, DecodeMode::Full, None, &uv)
        .expect("the decode dispatch");
    let mut c = Crossings {
        identical: 0,
        tiny: 0,
        clear: 0,
    };
    for (i, (out, p)) in gpu.iter().zip(&uv).enumerate() {
        let mut g = *out;
        taint(&mut g);
        assert_eq!(
            out.degenerate,
            m01_tag(g.ic.m),
            "the GPU's tag at crossing {i}, z_μ = ({}, {}): not the CPU form's on its emitted masses {:?}",
            p[0] * 2.0,
            p[1] * 2.0,
            g.ic.m
        );
        let native = kernel::stage::decode::sample_with::<
            Latent<f32>,
            <LatentStickBreaking as DecodeVariant>::Links,
            f32,
        >(&chart, p[0], p[1], None);
        if out.ic.m.map(f32::to_bits) == native.ic.m.map(f32::to_bits) {
            c.identical += 1;
            assert_eq!(
                out.degenerate, native.degenerate,
                "a fork at crossing {i}: identical masses, the GPU's tag differs from the native f32 decode's"
            );
        }
        match out.degenerate {
            Some(Degenerate::M01Tiny) => c.tiny += 1,
            _ => c.clear += 1,
        }
    }
    assert!(
        c.tiny > 0 && c.clear > 0,
        "the crossings reached one tag only on the GPU: {} M01_TINY, {} untagged",
        c.tiny,
        c.clear
    );
    eprintln!(
        "decode_parity_m01_crossings: {} points on {}; {} tagged M01_TINY, {} untagged; {} with masses bit for bit the \
         native f32 decode's",
        gpu.len(),
        h.adapter_info().name,
        c.tiny,
        c.clear,
        c.identical
    );
    c
}

#[test]
fn decode_parity_m01_crossings() {
    check_crossings(clean);
}

negative_control!(
    decode_parity_m01_crossings,
    "an M₀₁ recomputed from the GPU's m₀ moved down by one ulp of ε must disagree with the GPU's tag at a crossing",
    expected = "the GPU's tag at crossing",
    {
        check_crossings(|g| {
            let eps = <f32 as ChartConstants>::EPS_M01;
            g.ic.m[0] -= f32::from_bits(eps.to_bits() + 1) - eps;
        });
    }
);

/// A superseded form of `M₀₁` under an emulation of a backend: from the masses' last step `(m₀, t₇, t₁₃)`.
type Emulated = fn(f32, f32, f32) -> f32;

/// The explicit `fma(m₁, 1, m₀)` as a backend that folds the `×1` and contracts the add evaluates it:
/// `fma(t₇, t₁₃, m₀)`, fused (dd_decoder §3.1, "Its premise").
fn folded_fused(m0: f32, t7: f32, t13: f32) -> f32 {
    t7.mul_add(t13, m0)
}

/// The explicit `fma(m₁, 1, m₀)` as a backend that does not fuse it evaluates it, a rounded product and an add (CI's
/// lavapipe, RQ-298): `round(round(m₁·1) + m₀)`, the one rounded sum, so it never flips the tag (RQ-621).
#[cfg(feature = "controls")]
fn unfused(m0: f32, t7: f32, t13: f32) -> f32 {
    let m1 = t7 * t13;
    std::hint::black_box(m1 * 1.0) + m0
}

/// REQ-VAL-017's control on test 12's inputs: at the onset crossings ([`crossings`], the batch
/// `decode_parity_m01_crossings` dispatches),
/// the superseded form under `emulation`, evaluated on the CPU at f32, gives a different tag from the block's, the one
/// rounded sum's, on at least one input. Returns how many.
fn check_superseded(emulation: Emulated) -> usize {
    let eps = mass::m01_epsilon::<f32>();
    let mut flips = 0;
    for z in crossings() {
        let (m0, t7, t13) = stick_parts(z);
        let m = <StickBreaking as Link<2, 3>>::forward(z);
        assert_eq!(
            [m[0], m[1]],
            [m0, t7 * t13],
            "the replica is not the generated forward at {z:?}"
        );
        flips += usize::from((emulation(m0, t7, t13) < eps) != m01_tag(m).is_some());
    }
    assert!(
        flips > 0,
        "the superseded form flips no tag at the {} crossings",
        2 * LINES
    );
    flips
}

#[test]
fn decode_parity_superseded() {
    let flips = check_superseded(folded_fused);
    eprintln!(
        "decode_parity_superseded: fma(m₁, 1, m₀), the ×1 folded and the add contracted, flips {flips} of {} crossing \
         tags at f32",
        2 * LINES
    );
}

negative_control!(
    decode_parity_superseded,
    "the superseded fma(m₁, 1, m₀) unfused is the one rounded sum and must flip no tag",
    expected = "the superseded form flips no tag",
    {
        check_superseded(unfused);
    }
);
