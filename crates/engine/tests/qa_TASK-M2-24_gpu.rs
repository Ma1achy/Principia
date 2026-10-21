//! QA tests for TASK-M2-24 on the GPU, written from the requirements, not from the implementation: the decode stage,
//! `target/spirv/kernel.wgsl` (`cargo xtask build-kernel`), dispatched natively, against the same source at f64
//! (`decode_only_with`, parity_contract §1). They need the built kernel, so they belong to the `gpu-kernel` profile
//! with `decode_cross_backend`.
//! - REQ-VAL-017 (dd_decoder §5 test 12: "the degenerate/feasibility branch tags are bit-identical across all
//!   backends"; chart_reference §2.2): the feasibility tag is reached on the GPU, not only its absence. On the invariant
//!   charts, at points past the warp's square in `s`, where `K* < L_z²/2` (chart_reference §5.2: inside `[0,1]²` the
//!   warp keeps every pixel feasible), and on the `(ν, K)` chart, the GPU writes `INVARIANT_INFEASIBLE` exactly where
//!   the f64 decode does, on identical f32 `(u, v)` and parameters, and the momenta of each infeasible pixel are finite,
//!   kept, never dropped (`qa_m224_gpu_feasibility_tag`). The grid stays in the chart's domain, `t ≥ 0`: below `t = 0`
//!   the warp's `K(t) = K_max·t^γ_K` has no real value (chart_reference §2.1, §5.4: bounded there, blank, never
//!   decoded), and dd_decoder §2 claims finite outputs only inside the domain.
//! - REQ-VAL-017 on the charts the stage gained in the fix round (`BurrauThetaK`, `BurrauThetaLz`, `BurrauThetaDeltaM`,
//!   `ShapeSphere` under both maps), each with parameters other than the defaults, so a misplaced uniform slot decodes
//!   another chart: on `[0,1]²`'s interior, off the shape sphere's `n₀ = ±1` poles, the GPU's 27 continuous values
//!   within REQ-DEC-043's factor (`decode-f32-eps`'s gate.json) of the f64 decode and its tag bitwise equal; on
//!   `Φ_θ,L_z` past its affine `L_z` edges (chart_reference §5.4: the formula continues, `L_z² > 2IK` infeasible), the
//!   GPU and the f64 decode both write `INVARIANT_INFEASIBLE` at the same pixels, the outputs finite
//!   (`qa_m224_gpu_new_charts`).
//! - dd_decoder §3.7, "It must be disabled" on the invariant-momentum charts, and R-25 (`None` is off, `Some(0)` a real
//!   target): on the GPU, a forbidding chart's decode under every `Some(E*)` is its decode with `η_E` off, word for word,
//!   the `(θ, K)` and `(θ, L_z)` strips among them; on the latent chart and the shape sphere `Some(0)` rescales, and on
//!   the δm strip, a rest start, `Some(0)` above its `V₀ < 0` is `ENERGY_UNSCALABLE` (`qa_m224_gpu_eta_switch`).
//!
//! - REQ-VAL-017 on `C`, the canonicalisation the decode ends in (and encode starts from), alone on the GPU: generic
//!   states with one light body far out, the case that once put a body's `r_y` 273.88 eps from f64 on lavapipe when
//!   step 2 turned by `sin` and `cos` of a rounded angle, at every orientation and scale: each of the GPU's canonical
//!   `m`, `r` and `p` within REQ-DEC-043's factor of `C` at f64 on the same widened inputs, `lookup_mirrored` and
//!   `lookup_clamped` bitwise equal, and the canonical `ρ̃` on `+x` (REQ-DEC-002) (`qa_m224_gpu_canon_turn`).
//!
//! Tier B (parity_contract §2): the tags are compared bitwise, given the same inputs, at once, never accumulated (§3).
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use std::path::{Path, PathBuf};

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{canonicalise_gpu, decode_only_with, DecodeStage};
use kernel::canon::{canonicalise, Canonical};
use kernel::chart::invariant::Invariant;
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, ChartDecode, InvariantLE,
    InvariantLK, Latent, ShapeMap, ShapeSphere,
};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::Ic;
use kernel::stage::decode::{
    BurrauDefaults, BurrauThetaDeltaMDefaults, BurrauThetaKDefaults, BurrauThetaLzDefaults,
    DecodeMode, DecodeOut, DecodeVariant, InvariantLEDefaults, InvariantLKDefaults, LatentDefaults,
    ShapeSphereDefaults, UniformChart,
};
use validation::decode_f32_eps::{error, physics_key, values, QUANTITIES};
use validation::gpu::GpuHarness;
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, GAMMA_K, K_MAX,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn wgsl() -> String {
    let p = root().join("target/spirv/kernel.wgsl");
    std::fs::read_to_string(p)
        .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

struct Rng(u64);

impl Rng {
    fn uniform(&mut self, lo: f32, hi: f32) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        lo + (hi - lo) * ((z >> 11) as f64 / (1u64 << 53) as f64) as f32
    }
}

fn frozen(rng: &mut Rng) -> Invariant<f32> {
    Invariant {
        z_cfg: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        z_mass: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    }
}

/// A grid of `s ∈ [−0.75, 1.75]` by `t ∈ [0, 1.75]`: inside the warp's square, past it in `s` on both sides and past
/// `t = 1`, where the warp continues (R-508). Never below `t = 0`, outside the chart's domain: there `t^γ_K` has no real
/// value (chart_reference §2.1, §5.4), the pixel is blank and never decoded, and a backend's `pow` of a negative base
/// is undefined (WGSL), NaN on lavapipe.
fn grid() -> Vec<[f32; 2]> {
    let mut p = Vec::new();
    for i in 0..=20 {
        for j in 0..=14 {
            p.push([-0.75 + 0.125 * i as f32, 0.125 * j as f32]);
        }
    }
    p
}

fn gpu<V: DecodeVariant>(
    h: &GpuHarness,
    wgsl: &str,
    chart: &V::Chart,
    uv: &[[f32; 2]],
    e: Option<f32>,
) -> Vec<DecodeOut<f32>> {
    DecodeStage::<V>::new(h.device(), wgsl, FastMath::default())
        .expect("the decode stage's pipeline")
        .run_with(h.device(), h.queue(), chart, DecodeMode::Full, e, uv)
        .expect("the decode dispatch")
}

// ── REQ-VAL-017: the feasibility tag on the GPU ─────────────────────────────────────────────────────────────────

/// The GPU's and the f64 decode's tags at each point, compared bitwise; returns how many points each tagged
/// `INVARIANT_INFEASIBLE`. `drop` is a control's defect: the GPU's feasibility tag read as none.
fn feasibility<V>(
    h: &GpuHarness,
    wgsl: &str,
    chart: &V::Chart,
    uv: &[[f32; 2]],
    drop: bool,
) -> usize
where
    V: DecodeVariant,
    V::Chart: Chart<Param = f32> + Copy,
{
    let key = physics_key();
    let g = gpu::<V>(h, wgsl, chart, uv, None);
    let mut infeasible = 0;
    for (i, p) in uv.iter().enumerate() {
        let c = decode_only_with::<_, V::Links>(chart, p.map(f64::from), &key, None)
            .expect("decode_only");
        let mut tag = g[i].degenerate;
        if drop && tag == Some(Degenerate::InvariantInfeasible) {
            tag = None;
        }
        // The feasibility tag is decided on the shared (u, v) and parameters by an exact test (chart_reference §2.2;
        // dd_decoder §2's branch inputs), so it is compared bitwise, with no precision exemption.
        let fz = |t: Option<Degenerate>| t == Some(Degenerate::InvariantInfeasible);
        assert_eq!(
            fz(tag),
            fz(c.degenerate),
            "{} at {p:?}: the feasibility tag is {:?} on the GPU and {:?} at f64",
            V::ENTRY,
            tag,
            c.degenerate
        );
        if fz(tag) {
            infeasible += 1;
            let ic = &g[i].ic;
            assert!(
                ic.m.iter()
                    .chain(ic.r.iter().flatten())
                    .chain(ic.p.iter().flatten())
                    .all(|x| x.is_finite()),
                "{} at {p:?}: an infeasible pixel's (m, r, p) is not finite",
                V::ENTRY
            );
        }
    }
    infeasible
}

fn check_feasibility(drop: bool) {
    let h = GpuHarness::new().expect("a GPU device");
    let wgsl = wgsl();
    let uv = grid();
    let mut rng = Rng(2424);
    let mut found = [0usize; 2];
    for _ in 0..3 {
        found[0] += feasibility::<InvariantLEDefaults>(
            &h,
            &wgsl,
            &InvariantLE(frozen(&mut rng)),
            &uv,
            drop,
        );
        found[1] += feasibility::<InvariantLKDefaults>(
            &h,
            &wgsl,
            &InvariantLK(frozen(&mut rng)),
            &uv,
            drop,
        );
    }
    let b = Burrau {
        nu_range: [0.1, 0.9],
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    };
    // `ν` is bounded past its range (chart_reference §5.4: past `(0, 1)` a zero or negative mass), so the `(ν, K)`
    // chart keeps `s ∈ [0, 1]`.
    let inside: Vec<[f32; 2]> = uv
        .iter()
        .copied()
        .filter(|p| (0.0..=1.0).contains(&p[0]))
        .collect();
    let nb = feasibility::<BurrauDefaults>(&h, &wgsl, &b, &inside, drop);
    eprintln!(
        "qa_m224_gpu_feasibility_tag: INVARIANT_INFEASIBLE on {} (L_z, E) and {} (L_z, K) of {} points each, {nb} \
         (ν, K) of {}, on {}",
        found[0],
        found[1],
        3 * uv.len(),
        inside.len(),
        h.adapter_info().name
    );
    assert!(
        found[0] > 0 && found[1] > 0,
        "the GPU never wrote INVARIANT_INFEASIBLE on an invariant chart: {found:?}"
    );
}

#[test]
fn qa_m224_gpu_feasibility_tag() {
    check_feasibility(false);
}

negative_control!(
    qa_m224_gpu_feasibility_tag,
    "a GPU decode whose feasibility tag is dropped must fail the bitwise comparison",
    expected = "the feasibility tag is",
    check_feasibility(true)
);

// ── REQ-VAL-017 on the fix round's variants ─────────────────────────────────────────────────────────────────────

/// REQ-DEC-043's factor, `decode-f32-eps`'s threshold (provisional until the M2 gate).
fn factor() -> f64 {
    validation::gate::config(&root(), "decode-f32-eps")
        .expect("decode-f32-eps's gate.json")
        .threshold
        .value
}

/// The GPU's decodes of `chart` at `uv` toward `e` against the f64 decodes of `reference` at the same widened point
/// and target: the tag bitwise equal, each of the 27 continuous values within `f` eps (REQ-DEC-043's measure,
/// `validation::decode_f32_eps::error`). Returns the worst, in eps.
fn agree<V>(
    h: &GpuHarness,
    wgsl: &str,
    chart: &V::Chart,
    reference: &V::Chart,
    uv: &[[f32; 2]],
    e: Option<f32>,
    f: f64,
) -> f64
where
    V: DecodeVariant,
    V::Chart: ChartDecode + Chart<Param = f32>,
{
    let key = physics_key();
    let g = gpu::<V>(h, wgsl, chart, uv, e);
    let mut worst: f64 = 0.0;
    for (i, p) in uv.iter().enumerate() {
        let c =
            decode_only_with::<_, V::Links>(reference, p.map(f64::from), &key, e.map(f64::from))
                .expect("decode_only");
        assert_eq!(
            g[i].degenerate,
            c.degenerate,
            "{} at {p:?}, E* {e:?}: the tag is {:?} on the GPU and {:?} at f64",
            V::ENTRY,
            g[i].degenerate,
            c.degenerate
        );
        for (k, (a, b)) in values(&g[i]).into_iter().zip(values(&c)).enumerate() {
            let x = error(a, b);
            worst = worst.max(x);
            assert!(
                x <= f,
                "{} at {p:?}, E* {e:?}: {} is {x:.2} eps from the f64 decode, over REQ-DEC-043's {f}",
                V::ENTRY,
                QUANTITIES[k]
            );
        }
    }
    worst
}

/// `n` points of `[0.02, 0.98]²`, the interior.
fn interior(rng: &mut Rng, n: usize) -> Vec<[f32; 2]> {
    (0..n)
        .map(|_| [rng.uniform(0.02, 0.98), rng.uniform(0.02, 0.98)])
        .collect()
}

/// A `θ` range inside the default one, `(0, π/4]`, other than the default.
fn theta_range(rng: &mut Rng) -> [f32; 2] {
    let lo = BURRAU_THETA_MIN.number() as f32;
    [rng.uniform(lo, 0.3), rng.uniform(0.45, 0.78)]
}

/// The shape sphere under `map`: the exponential frame `(n₀, e₁, e₂)` orthonormal and off the default `+w` pole, its
/// extent at most 1 so every point of `[0,1]²` lies within `r < π/2` of `n₀` and off `n₀ = ±1`; the held blocks drawn.
fn sphere(rng: &mut Rng, map: ShapeMap) -> ShapeSphere<f32> {
    ShapeSphere {
        map,
        n0: [0.0, 0.6, 0.8],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, -0.8, 0.6],
        scale: rng.uniform(0.5, 1.0),
        inertia: rng.uniform(0.5, 2.0),
        phase: rng.uniform(-3.0, 3.0),
        z_mass: [rng.uniform(-1.0, 1.0), rng.uniform(-1.0, 1.0)],
        z_mom: core::array::from_fn(|_| rng.uniform(-1.0, 1.0)),
    }
}

/// The points of `uv` at least `10⁻²` from the shape sphere's `n₀ = ±1` poles, where `ρ̃ = 0` (`BC₀₁`) or `λ̃ = 0`
/// amplify a rounding of the point (chart_reference §3.2; R-436, R-437), outside REQ-DEC-043's domain.
fn off_poles(c: &ShapeSphere<f32>, uv: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    uv.into_iter()
        .filter(|p| 1.0 - c.point::<f64>(f64::from(p[0]), f64::from(p[1]))[0].abs() > 1e-2)
        .collect()
}

/// Mass targets for the δm strip, each summing to 1 exactly at f32 and f64.
const TARGETS: [[f32; 3]; 3] = [
    [0.25, 0.375, 0.375],
    [0.5, 0.25, 0.25],
    [0.125, 0.625, 0.25],
];

/// `drift` is a control's defect: the f64 reference of the `(θ, K)` strip reads `γ_K + 10⁻³`, a uniform slot read
/// wrong.
fn check_new_charts(drift: bool) {
    let h = GpuHarness::new().expect("a GPU device");
    let wgsl = wgsl();
    let f = factor();
    let mut rng = Rng(9024);
    let mut worst = [0.0f64; 5];
    for target in TARGETS {
        let k = BurrauThetaK {
            theta_range: theta_range(&mut rng),
            k_max: rng.uniform(0.25, 2.0),
            gamma_k: rng.uniform(1.0, 3.0),
        };
        let mut k_ref = k;
        if drift {
            k_ref.gamma_k += 1e-3;
        }
        let uv = interior(&mut rng, 96);
        worst[0] = worst[0].max(agree::<BurrauThetaKDefaults>(
            &h, &wgsl, &k, &k_ref, &uv, None, f,
        ));
        let lz = BurrauThetaLz {
            theta_range: theta_range(&mut rng),
            k: rng.uniform(0.05, 1.0),
            lz_fraction: rng.uniform(0.3, 0.95),
        };
        let uv = interior(&mut rng, 96);
        worst[1] = worst[1].max(agree::<BurrauThetaLzDefaults>(
            &h, &wgsl, &lz, &lz, &uv, None, f,
        ));
        let m = BurrauThetaDeltaM {
            theta_range: theta_range(&mut rng),
            m_target: target,
        };
        let uv = interior(&mut rng, 96);
        for e in [None, Some(0.0), Some(-40.0)] {
            worst[2] = worst[2].max(agree::<BurrauThetaDeltaMDefaults>(
                &h, &wgsl, &m, &m, &uv, e, f,
            ));
        }
        for (n, map) in [ShapeMap::Spherical, ShapeMap::Exponential]
            .into_iter()
            .enumerate()
        {
            let c = sphere(&mut rng, map);
            let uv = off_poles(&c, interior(&mut rng, 96));
            assert!(uv.len() > 48, "the pole filter left {} points", uv.len());
            for e in [None, Some(0.0), Some(1.5)] {
                worst[3 + n] =
                    worst[3 + n].max(agree::<ShapeSphereDefaults>(&h, &wgsl, &c, &c, &uv, e, f));
            }
        }
    }
    eprintln!(
        "qa_m224_gpu_new_charts: worst against f64, in eps (REQ-DEC-043's {f}): (θ, K) {:.2}, (θ, L_z) {:.2}, (θ, Δm) \
         {:.2}, shape sphere spherical {:.2}, exponential {:.2}, on {}",
        worst[0],
        worst[1],
        worst[2],
        worst[3],
        worst[4],
        h.adapter_info().name
    );
}

#[test]
fn qa_m224_gpu_new_charts() {
    check_new_charts(false);
}

negative_control!(
    qa_m224_gpu_new_charts,
    "an f64 reference whose (θ, K) strip reads γ_K + 10⁻³ must fail REQ-DEC-043's factor",
    expected = "over REQ-DEC-043's",
    check_new_charts(true)
);

/// `Φ_θ,L_z` on `u ∈ [0, 1]` and `v ∈ [−0.5, 1.5]`, `λ_L` at its default 1 and at 0.75: past the affine `L_z` axis's
/// edges `L_z² > 2IK` (chart_reference §4.5, §5.4), `INVARIANT_INFEASIBLE`. Each tag bitwise equal to the f64 decode's,
/// each output finite, and the tag reached. `drop` is a control's defect: the GPU's feasibility tag
/// read as none.
fn check_theta_lz(drop: bool) {
    let h = GpuHarness::new().expect("a GPU device");
    let wgsl = wgsl();
    let key = physics_key();
    let mut uv = Vec::new();
    for i in 0..=8 {
        for j in 0..=32 {
            uv.push([0.125 * i as f32, -0.5 + 0.0625 * j as f32]);
        }
    }
    let mut found = 0;
    for fraction in [BURRAU_THETA_LZ_FRACTION.number() as f32, 0.75] {
        let c = BurrauThetaLz {
            theta_range: [
                BURRAU_THETA_MIN.number() as f32,
                BURRAU_THETA_MAX.number() as f32,
            ],
            k: BURRAU_THETA_LZ_K.number() as f32,
            lz_fraction: fraction,
        };
        let g = gpu::<BurrauThetaLzDefaults>(&h, &wgsl, &c, &uv, None);
        for (i, p) in uv.iter().enumerate() {
            let cpu = decode_only_with::<_, <BurrauThetaLzDefaults as DecodeVariant>::Links>(
                &c,
                p.map(f64::from),
                &key,
                None,
            )
            .expect("decode_only");
            let mut tag = g[i].degenerate;
            if drop && tag == Some(Degenerate::InvariantInfeasible) {
                tag = None;
            }
            assert_eq!(
                tag, cpu.degenerate,
                "(θ, L_z) at {p:?}, λ_L {fraction}: the tag is {tag:?} on the GPU and {:?} at f64",
                cpu.degenerate
            );
            let ic = &g[i].ic;
            assert!(
                ic.m.iter()
                    .chain(ic.r.iter().flatten())
                    .chain(ic.p.iter().flatten())
                    .all(|x| x.is_finite()),
                "(θ, L_z) at {p:?}, λ_L {fraction}: the GPU's (m, r, p) is not finite"
            );
            // Inside the band the pixel is feasible; past the edge, `|2v − 1|·λ_L > 1`, infeasible: the exact sign
            // decides it, so only points at least 10⁻³ from the boundary are held to the side they lie on.
            let reach = (2.0 * f64::from(p[1]) - 1.0).abs() * f64::from(fraction);
            if reach > 1.001 {
                assert_eq!(
                    tag,
                    Some(Degenerate::InvariantInfeasible),
                    "(θ, L_z) at {p:?}, λ_L {fraction}: past the L_z edge the GPU tags {tag:?}"
                );
            } else if reach < 0.999 {
                assert_eq!(
                    tag, None,
                    "(θ, L_z) at {p:?}, λ_L {fraction}: inside the band the GPU tags {tag:?}"
                );
            }
            if tag == Some(Degenerate::InvariantInfeasible) {
                found += 1;
            }
        }
    }
    eprintln!(
        "qa_m224_gpu_theta_lz_infeasible: INVARIANT_INFEASIBLE on {found} of {} points, on {}",
        2 * uv.len(),
        h.adapter_info().name
    );
    assert!(
        found > 0,
        "the GPU never wrote INVARIANT_INFEASIBLE on (θ, L_z)"
    );
}

#[test]
fn qa_m224_gpu_theta_lz_infeasible() {
    check_theta_lz(false);
}

negative_control!(
    qa_m224_gpu_theta_lz_infeasible,
    "a GPU decode whose feasibility tag is dropped must fail the bitwise comparison",
    expected = "the tag is",
    check_theta_lz(true)
);

// ── dd_decoder §3.7 / R-25: the switch on the GPU ───────────────────────────────────────────────────────────────

fn words(o: &[DecodeOut<f32>]) -> String {
    format!(
        "{:?}",
        o.iter()
            .map(|d| (d.ic, d.descriptor, d.degenerate))
            .collect::<Vec<_>>()
    )
}

fn ignores<V>(h: &GpuHarness, wgsl: &str, chart: &V::Chart, uv: &[[f32; 2]])
where
    V: DecodeVariant,
    V::Chart: Chart<Param = f32> + UniformChart,
{
    let off = words(&gpu::<V>(h, wgsl, chart, uv, None));
    for e in [0.0f32, -0.5, 2.0, -30.0] {
        assert_eq!(
            words(&gpu::<V>(h, wgsl, chart, uv, Some(e))),
            off,
            "{}: the GPU did not ignore E* = {e}",
            V::ENTRY
        );
    }
}

fn check_switch(latent_too: bool) {
    let h = GpuHarness::new().expect("a GPU device");
    let wgsl = wgsl();
    let mut rng = Rng(77);
    let uv: Vec<[f32; 2]> = (0..64)
        .map(|_| [rng.uniform(0.0, 1.0), rng.uniform(0.0, 1.0)])
        .collect();
    let le = InvariantLE(frozen(&mut rng));
    let lk = InvariantLK(frozen(&mut rng));
    assert!(le.forbids_energy_normalisation() && lk.forbids_energy_normalisation());
    ignores::<InvariantLEDefaults>(&h, &wgsl, &le, &uv);
    ignores::<InvariantLKDefaults>(&h, &wgsl, &lk, &uv);
    let b = Burrau {
        nu_range: [0.1, 0.9],
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    };
    ignores::<BurrauDefaults>(&h, &wgsl, &b, &uv);
    let range = [
        BURRAU_THETA_MIN.number() as f32,
        BURRAU_THETA_MAX.number() as f32,
    ];
    let strip_k = BurrauThetaK {
        theta_range: range,
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    };
    let strip_lz = BurrauThetaLz {
        theta_range: range,
        k: BURRAU_THETA_LZ_K.number() as f32,
        lz_fraction: BURRAU_THETA_LZ_FRACTION.number() as f32,
    };
    assert!(strip_k.forbids_energy_normalisation() && strip_lz.forbids_energy_normalisation());
    ignores::<BurrauThetaKDefaults>(&h, &wgsl, &strip_k, &uv);
    ignores::<BurrauThetaLzDefaults>(&h, &wgsl, &strip_lz, &uv);
    // The δm strip is a rest start, `K₀ = 0`, with `V₀ < 0`: `Some(0)` lies above `V₀`, so dd_decoder §3.7 tags every
    // pixel `ENERGY_UNSCALABLE`, where `None` tags none.
    let strip_m = BurrauThetaDeltaM {
        theta_range: range,
        m_target: TARGETS[0],
    };
    let m_off = gpu::<BurrauThetaDeltaMDefaults>(&h, &wgsl, &strip_m, &uv, None);
    let m_zero = gpu::<BurrauThetaDeltaMDefaults>(&h, &wgsl, &strip_m, &uv, Some(0.0));
    for (i, p) in uv.iter().enumerate() {
        assert_eq!(m_off[i].degenerate, None, "(θ, Δm) at {p:?}: η_E off tags");
        assert_eq!(
            m_zero[i].degenerate,
            Some(Degenerate::EnergyUnscalable),
            "(θ, Δm) at {p:?}: Some(0) on the GPU is read as off"
        );
    }
    for map in [ShapeMap::Spherical, ShapeMap::Exponential] {
        let c = sphere(&mut rng, map);
        let off = gpu::<ShapeSphereDefaults>(&h, &wgsl, &c, &uv, None);
        let zero = gpu::<ShapeSphereDefaults>(&h, &wgsl, &c, &uv, Some(0.0));
        let moved = off
            .iter()
            .zip(&zero)
            .filter(|(a, b)| a.degenerate.is_none() && b.degenerate.is_none() && a.ic.p != b.ic.p)
            .count();
        assert!(
            moved > 0,
            "Some(0) on the GPU left every {map:?} shape-sphere decode as off: it is read as off"
        );
    }
    let mut draw = |s: f32| -> [f32; 8] { core::array::from_fn(|_| rng.uniform(-s, s)) };
    let lat = Latent {
        z0: draw(1.0),
        q1: draw(0.5),
        q2: draw(0.5),
    };
    let off = gpu::<LatentDefaults>(&h, &wgsl, &lat, &uv, None);
    let zero = gpu::<LatentDefaults>(&h, &wgsl, &lat, &uv, Some(0.0));
    let moved = off
        .iter()
        .zip(&zero)
        .filter(|(a, b)| a.degenerate.is_none() && b.degenerate.is_none() && a.ic.p != b.ic.p)
        .count();
    assert!(
        moved > 0,
        "Some(0) on the GPU left every latent decode as off: it is read as off"
    );
    if latent_too {
        ignores::<LatentDefaults>(&h, &wgsl, &lat, &uv);
    }
}

#[test]
fn qa_m224_gpu_eta_switch() {
    check_switch(false);
}

negative_control!(
    qa_m224_gpu_eta_switch,
    "the latent chart, which allows η_E, must not ignore the target on the GPU",
    expected = "the GPU did not ignore",
    check_switch(true)
);

// ── REQ-VAL-017 on `C`: step 2's turn on the GPU ────────────────────────────────────────────────────────────────

/// `n` generic f32 states with a light body far out: the light body (index cycling 0, 1, 2) of mass `10^[−3.5, −2]`
/// at distance `[4, 40]` from the CoM of two heavy ones placed in `[−1, 1]²`, momenta in `[−0.5, 0.5]²`; the whole
/// turned by `θ ∈ [−π, π]` and scaled by `2^[−20, 20]`, so `ρ` lies in every quadrant and at every scale.
fn far_light(n: usize) -> Vec<Ic<f32>> {
    let mut rng = Rng(2402);
    (0..n)
        .map(|j| {
            let k = j % 3;
            let light = 10f64.powf(f64::from(rng.uniform(-3.5, -2.0)));
            let w = [
                f64::from(rng.uniform(0.3, 1.0)),
                f64::from(rng.uniform(0.3, 1.0)),
            ];
            let rest = 1.0 - light;
            let mut m = [0.0; 3];
            let mut r = [[0.0; 2]; 3];
            let mut h = 0;
            for (i, (mi, ri)) in m.iter_mut().zip(r.iter_mut()).enumerate() {
                if i == k {
                    let d = f64::from(rng.uniform(4.0, 40.0));
                    let a = f64::from(rng.uniform(-3.2, 3.2));
                    *mi = light;
                    *ri = [d * a.cos(), d * a.sin()];
                } else {
                    *mi = rest * w[h] / (w[0] + w[1]);
                    *ri = [
                        f64::from(rng.uniform(-1.0, 1.0)),
                        f64::from(rng.uniform(-1.0, 1.0)),
                    ];
                    h += 1;
                }
            }
            let theta = f64::from(rng.uniform(-3.2, 3.2));
            let (s, c) = theta.sin_cos();
            let scale = 2f64.powi(rng.uniform(-20.0, 20.0) as i32);
            Ic {
                m: m.map(|x| x as f32),
                r: r.map(|v| {
                    [
                        ((c * v[0] - s * v[1]) * scale) as f32,
                        ((s * v[0] + c * v[1]) * scale) as f32,
                    ]
                }),
                p: [0; 3].map(|_| [rng.uniform(-0.5, 0.5), rng.uniform(-0.5, 0.5)]),
            }
        })
        .collect()
}

fn widen(ic: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: ic.m.map(f64::from),
        r: ic.r.map(|v| v.map(f64::from)),
        p: ic.p.map(|v| v.map(f64::from)),
    }
}

/// `v` turned by `a` radians.
fn turn(v: [f64; 2], a: f64) -> [f64; 2] {
    let (s, c) = a.sin_cos();
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

/// `C` on the GPU against `C` at f64 on the same inputs, widened: the notices bitwise equal; each of `m`, `r` and `p`
/// within REQ-DEC-043's factor (`decode_f32_eps::error`, the decode's own measure); the GPU's `ρ̃ = r̃₁ − r̃₀` on `+x`,
/// its `x` positive and its `y` within the factor (against f64's 0). `skew` is a control's defect: the f64 reference
/// turned a further `skew` radians, a step 2 whose angle is off.
fn check_canon_turn(skew: f64) {
    let h = GpuHarness::new().expect("a GPU device");
    let f = factor();
    let states = far_light(4096);
    let g: Vec<Canonical<f32>> =
        canonicalise_gpu(h.device(), h.queue(), &wgsl(), &states, FastMath::default())
            .expect("the canonicalise dispatch");
    assert_eq!(g.len(), states.len());
    let (mut worst, mut cpu32, mut far): (f64, f64, f64) = (0.0, 0.0, 0.0);
    for (j, (x, gj)) in states.iter().zip(&g).enumerate() {
        let c = canonicalise(&widen(x));
        assert_eq!(
            (gj.mirrored, gj.clamped),
            (c.mirrored, c.clamped),
            "state {j}: the GPU's (mirrored, clamped) differ from f64's"
        );
        let r64 = c.ic.r.map(|v| turn(v, skew));
        let p64 = c.ic.p.map(|v| turn(v, skew));
        let g64 = widen(&gj.ic);
        let pairs = c.ic.m.iter().zip(&g64.m).map(|(a, b)| ("m", *a, *b)).chain(
            r64.iter()
                .flatten()
                .zip(g64.r.iter().flatten())
                .map(|(a, b)| ("r", *a, *b)),
        );
        let pairs = pairs.chain(
            p64.iter()
                .flatten()
                .zip(g64.p.iter().flatten())
                .map(|(a, b)| ("p", *a, *b)),
        );
        // CPU-f32 on the same inputs, reported beside the GPU: what f32 alone costs against f64.
        let w32 = widen(&canonicalise(x).ic);
        for (a, b) in
            c.ic.r
                .iter()
                .flatten()
                .chain(c.ic.p.iter().flatten())
                .zip(w32.r.iter().flatten().chain(w32.p.iter().flatten()))
        {
            cpu32 = cpu32.max(error(*b, *a));
        }
        for (what, a, b) in pairs {
            let e = error(b, a);
            worst = worst.max(e);
            assert!(
                e <= f,
                "state {j}: the GPU's canonical {what} {b:e} is {e:.2} eps from f64's {a:e}, over REQ-DEC-043's {f}"
            );
        }
        let rho = [g64.r[1][0] - g64.r[0][0], g64.r[1][1] - g64.r[0][1]];
        assert!(
            rho[0] > 0.0,
            "state {j}: the GPU's ρ̃ = {rho:?} is not on +x"
        );
        let ey = error(rho[1], 0.0);
        assert!(
            ey <= f,
            "state {j}: the GPU's ρ̃_y is {ey:.2} eps off +x, over REQ-DEC-043's {f}"
        );
        far = far.max(g64.r.iter().map(|v| v[0].hypot(v[1])).fold(0.0, f64::max));
    }
    // The domain reached: a light body far out at I = 1, where a turn's error in angle is multiplied most.
    assert!(far > 10.0, "no canonical body beyond ‖r‖ = {far}");
    eprintln!(
        "qa_m224_gpu_canon_turn: {} states, worst {worst:.2} eps against f64 (CPU-f32 {cpu32:.2}; REQ-DEC-043's \
         {f}), farthest body at ‖r‖ = {far:.1}, on {}",
        states.len(),
        h.adapter_info().name
    );
}

#[test]
fn qa_m224_gpu_canon_turn() {
    check_canon_turn(0.0);
}

negative_control!(
    qa_m224_gpu_canon_turn,
    "an f64 reference turned 10⁻⁶ rad further, a step-2 angle off by the order of lavapipe's trig, must fail REQ-DEC-043's factor",
    expected = "over REQ-DEC-043's",
    check_canon_turn(1e-6)
);
