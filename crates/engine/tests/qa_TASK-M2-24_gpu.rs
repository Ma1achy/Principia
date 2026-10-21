//! QA tests for TASK-M2-24 on the GPU, written from the requirements, not from the implementation: the decode stage,
//! `target/spirv/kernel.wgsl` (`cargo xtask build-kernel`), dispatched natively, against the same source at f64
//! (`decode_only_with`, parity_contract §1). They need the built kernel, so they belong to the `gpu-kernel` profile
//! with `decode_cross_backend`.
//! - REQ-VAL-017 (dd_decoder §5 test 12: "the degenerate/feasibility branch tags are bit-identical across all
//!   backends"; chart_reference §2.2): the feasibility tag is reached on the GPU, not only its absence. On the invariant
//!   charts, at points past the warp's domain, where `K* < L_z²/2` (chart_reference §5.2: inside `[0,1]²` the warp keeps
//!   every pixel feasible), and on the `(ν, K)` chart, the GPU writes `INVARIANT_INFEASIBLE` exactly where the f64
//!   decode does, on identical f32 `(u, v)` and parameters, and the momenta of each infeasible pixel are finite, kept,
//!   never dropped (`qa_m224_gpu_feasibility_tag`).
//! - dd_decoder §3.7, "It must be disabled" on the invariant-momentum charts, and R-25 (`None` is off, `Some(0)` a real
//!   target): on the GPU, a forbidding chart's decode under every `Some(E*)` is its decode with `η_E` off, word for word,
//!   and on the latent chart `Some(0)` rescales (`qa_m224_gpu_eta_switch`).
//!
//! Tier B (parity_contract §2): the tags are compared bitwise, given the same inputs, at once, never accumulated (§3).
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use std::path::Path;

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{decode_only_with, DecodeStage};
use kernel::chart::invariant::Invariant;
use kernel::chart::{Burrau, Chart, InvariantLE, InvariantLK, Latent};
use kernel::decode::degenerate::Degenerate;
use kernel::stage::decode::{
    BurrauDefaults, DecodeMode, DecodeOut, DecodeVariant, InvariantLEDefaults, InvariantLKDefaults,
    LatentDefaults, UniformChart,
};
use validation::decode_f32_eps::physics_key;
use validation::gpu::GpuHarness;
use validation::{negative_control, GAMMA_K, K_MAX};

fn wgsl() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/spirv/kernel.wgsl");
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

/// A grid on `[−0.75, 1.75]²`, inside the warp's domain and well past it on every side.
fn grid() -> Vec<[f32; 2]> {
    let mut p = Vec::new();
    for i in 0..=20 {
        for j in 0..=20 {
            p.push([-0.75 + 0.125 * i as f32, -0.75 + 0.125 * j as f32]);
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
    let nb = feasibility::<BurrauDefaults>(&h, &wgsl, &b, &uv, drop);
    eprintln!(
        "qa_m224_gpu_feasibility_tag: INVARIANT_INFEASIBLE on {} (L_z, E), {} (L_z, K), {nb} (ν, K) of {} points each \
         on {}",
        found[0],
        found[1],
        3 * uv.len(),
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
