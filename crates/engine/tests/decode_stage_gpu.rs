//! The kernel's decode stage on the GPU (`kernel::stage::decode`, `engine::decode_dispatch`; TASK-M2-06), its f32 build,
//! `target/spirv/kernel.wgsl`, dispatched natively through the compute entry point, read back and checked against the
//! same source at f64 (`decode_only`). CI runs this file in the `gpu-kernel` job, after `cargo xtask build-kernel`.
//! - `decode_stage_identities`: dd_decoder §5 tests 1–4 on the kernel's actual output, the `(m, r, p)` the integrator
//!   reads, over the golden IC, fuzzed interiors and saturated controls (REQ-DEC-005).
//! - `decode_uniforms_no_recompile`: editing `z₀`, `q₁`, `q₂` and a slice value changes the decode and creates no
//!   pipeline (REQ-DEC-007).
//! - `decode_stage_descriptor`: the GPU `ICDescriptor` against the f64 decode's, within REQ-DEC-043's factor; at the
//!   poles, the GPU stores the bits the f64 decode does (REQ-TOOL-030, R-436).
//! - `decode_stage_mode`: `DECODE_MODE` linearised writes nothing in the kernel and is refused by the host until M6
//!   (REQ-DEC-007).
//! - `decode_only_sim_key`: `decode_only` is the stage's source at f64, from the chart's parameters widened, and only
//!   the physics kernel decodes.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.

use std::path::{Path, PathBuf};

use engine::compute::pipelines_created;
use engine::contract::fast_math::FastMath;
use engine::contract::sim_config::KernelVariant;
use engine::decode_dispatch::{decode_only, describe_gpu, DecodeStage, UNWRITTEN};
use kernel::chart::Latent;
use kernel::decode::descriptor::{describe, pi_image};
use kernel::decode::{Degenerate, Ic};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{
    flags, sample, DecodeMode, DecodeOut, LatentDefaults, LatentStickBreaking, StickBreakingLinks,
};
use validation::decode_f32_eps::{interior, measure, physics_key, values, Batch, Fuzz, QUANTITIES};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The WGSL `cargo xtask build-kernel` wrote.
fn wgsl() -> String {
    std::fs::read_to_string(root().join("target/spirv/kernel.wgsl"))
        .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

fn harness() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

/// The latent chart at the point `z`: `z₀ = z`, `q₁ = q₂ = 0`.
fn at(z: [f32; 8]) -> Latent<f32> {
    Latent {
        z0: z,
        q1: [0.0; 8],
        q2: [0.0; 8],
    }
}

/// The GPU's decodes of `batches` under the variant's stage.
fn gpu_decodes<V>(h: &GpuHarness, batches: &[Batch]) -> Vec<DecodeOut<f32>>
where
    V: kernel::stage::decode::DecodeVariant<Chart = Latent<f32>>,
{
    let stage = DecodeStage::<V>::new(h.device(), &wgsl(), FastMath::default())
        .expect("the decode stage's pipeline");
    batches
        .iter()
        .flat_map(|(chart, uv)| {
            stage
                .run(h.device(), h.queue(), chart, DecodeMode::Full, uv)
                .expect("the decode dispatch")
        })
        .collect()
}

/// The f64 decodes of `batches` under `Defaults`.
fn f64_decodes(batches: &[Batch]) -> Vec<DecodeOut<f64>> {
    let key = physics_key();
    batches
        .iter()
        .flat_map(|(chart, uv)| {
            let key = &key;
            uv.iter().map(move |p| {
                decode_only::<_, Defaults>(chart, p.map(f64::from), key).expect("decode_only")
            })
        })
        .collect()
}

/// The golden IC, `z = 0`.
fn golden() -> Batch {
    (at([0.0; 8]), vec![[0.5, 0.5]])
}

/// Saturated controls: each component at `±30` or `±1000` (past f32's and f64's `exp` overflow), the other components
/// fuzzed, at the chart's centre.
fn saturated() -> Vec<Batch> {
    let mut rng = validation::decode_f32_eps::SplitMix::new(7);
    let mut out = Vec::new();
    for k in 0..8 {
        for s in [-1000.0, -30.0, 30.0, 1000.0] {
            let mut z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
            z[k] = s;
            out.push((at(z), vec![[0.5, 0.5]]));
        }
    }
    out
}

/// Every batch the identity suite decodes: the golden IC, a fuzzed interior and the saturated controls.
fn identity_batches() -> Vec<Batch> {
    let mut b = vec![golden()];
    b.extend(interior(Fuzz {
        seed: 3,
        charts: 16,
        per_chart: 64,
    }));
    b.extend(saturated());
    b
}

// ── decode_stage_identities (REQ-DEC-005) ───────────────────────────────────────────────────────────────────────

/// REQ-DEC-044's and REQ-DEC-045's proposals for test 1, in f32 eps (`kernel/tests/decode_blocks.rs`,
/// `decode_join.rs`): `Σmᵢ − 1`; the CoM relative to `Σmᵢ‖rᵢ‖`; `Σpᵢ` relative to `Σ‖pᵢ‖`; `I − 1`.
const SUM_EPS: f64 = 4.0;
const COM_EPS: f64 = 4.0;
const SUM_P_EPS: f64 = 4.0;
const INERTIA_EPS: f64 = 8.0;

const EPS: f64 = f32::EPSILON as f64;

/// Test 1 on one output: `Σmᵢ = 1`, CoM `= 0`, `Σpᵢ = 0`, `I = 1`, each within its proposal; and no NaN anywhere.
fn check_identities(ic: &Ic<f32>, at: &str) {
    let m = ic.m.map(f64::from);
    let r = ic.r.map(|v| v.map(f64::from));
    let p = ic.p.map(|v| v.map(f64::from));
    assert!(
        m.iter()
            .chain(r.iter().flatten())
            .chain(p.iter().flatten())
            .all(|x| x.is_finite()),
        "a non-finite (m, r, p) at {at}"
    );
    let sum = (m[0] + m[1] + m[2] - 1.0).abs() / EPS;
    assert!(sum <= SUM_EPS, "Σm − 1 = {sum} eps at {at}");
    let com = [0, 1].map(|k| (0..3).map(|i| m[i] * r[i][k]).sum::<f64>());
    let size: f64 = (0..3).map(|i| m[i] * r[i][0].hypot(r[i][1])).sum();
    let err = com[0].hypot(com[1]) / (size * EPS);
    assert!(err <= COM_EPS, "the CoM is {err} eps of Σm‖r‖ at {at}");
    let scale: f64 = p.iter().map(|v| v[0].hypot(v[1])).sum();
    for s in [0, 1].map(|k| (p[0][k] + p[1][k] + p[2][k]).abs()) {
        let err = if scale == 0.0 { s } else { s / (scale * EPS) };
        assert!(err <= SUM_P_EPS, "Σp = {s} ({err} eps of Σ‖p‖) at {at}");
    }
    let inertia: f64 = (0..3)
        .map(|i| m[i] * (r[i][0].powi(2) + r[i][1].powi(2)))
        .sum();
    let err = (inertia - 1.0).abs() / EPS;
    assert!(err <= INERTIA_EPS, "I − 1 = {err} eps at {at}");
}

/// Test 3 on one output, in the canonical frame: `ρ̃` on `+x` (`r₁ − r₀` with `y` exactly 0, `x ≥ 0`), `λ̃_y ≥ 0`;
/// `α ∈ [0, π/2]` as `rho_mag = cos α` and `lambda_mag = sin α` on `[0, 1]`, each read back from the emitted `(m, r)`, so
/// to the reconstruction's rounding, the `I = 1` bound [`INERTIA_EPS`]; `β ∈ [0, π]` as `rho_angle` on `[0, π̂]`, exactly,
/// its value selected (dd_decoder §3.6); and `|p_λ| = |(q₂, q₃)| ≤ q_max`.
fn check_ranges(out: &DecodeOut<f32>, at: &str) {
    let [r0, r1, r2] = out.ic.r;
    assert_eq!(
        r1[1].to_bits(),
        r0[1].to_bits(),
        "ρ̃ is not on the x axis at {at}"
    );
    assert!(r1[0] >= r0[0], "ρ̃ points along −x at {at}");
    assert!(r2[1] >= r0[1], "λ̃_y < 0 at {at}");
    let d = &out.descriptor;
    for (what, x) in [("rho_mag", d.rho_mag), ("lambda_mag", d.lambda_mag)] {
        let top = 1.0 + INERTIA_EPS * EPS;
        assert!(
            (0.0..=top).contains(&f64::from(x)),
            "{what} = {x} is outside [0, 1] beyond the reconstruction's rounding at {at}"
        );
    }
    let pi = pi_image::<f32>();
    assert!(
        (0.0..=pi).contains(&d.rho_angle),
        "β = rho_angle = {} is outside [0, π̂] at {at}",
        d.rho_angle
    );
    let q_max = <f32 as ChartConstants>::Q_MAX;
    assert!(
        out.ic.p[2].iter().all(|q| q.abs() <= q_max),
        "|p_λ| = {:?} passes q_max at {at}",
        out.ic.p[2]
    );
}

/// Test 4 on one output: tagged iff `M₀₁ < ε_M01`, `M₀₁` the emitted masses' one-rounding sum, as the kernel forms it.
fn check_tag(out: &DecodeOut<f32>, at: &str) -> bool {
    let m01 = out.ic.m[1].mul_add(1.0, out.ic.m[0]);
    let tiny = m01 < <f32 as ChartConstants>::EPS_M01;
    assert_eq!(
        out.degenerate,
        tiny.then_some(Degenerate::M01Tiny),
        "M₀₁ = {m01:e}: the tag is not M₀₁ < ε at {at}"
    );
    tiny
}

/// Test 2 on the output: along a line through `z` in one block's controls, the outputs another block alone decides
/// are bit-identical: config moved, `m` and `p` fixed; momentum moved, `m` and `r` fixed; mass moved, `p₂ = p_λ =
/// (q₂, q₃)` fixed.
fn check_independence(outs: &[(usize, Vec<DecodeOut<f32>>)]) {
    for (block, line) in outs {
        let first = &line[0].ic;
        for (j, o) in line.iter().enumerate().skip(1) {
            let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            let (m, r, p) = (&o.ic.m[..], o.ic.r.as_flattened(), o.ic.p.as_flattened());
            let (m0, r0, p0) = (&first.m[..], first.r.as_flattened(), first.p.as_flattened());
            match block {
                0 => assert!(
                    bits(m) == bits(m0) && bits(p) == bits(p0),
                    "moving z_cfg moved m or p at point {j}"
                ),
                2 => assert!(
                    bits(m) == bits(m0) && bits(r) == bits(r0),
                    "moving z_mom moved m or r at point {j}"
                ),
                _ => assert!(
                    bits(&o.ic.p[2]) == bits(&first.p[2]),
                    "moving z_μ moved p_λ at point {j}"
                ),
            }
            assert_ne!(
                o.ic, *first,
                "the line in block {block} moved nothing at point {j}"
            );
        }
    }
}

/// Lines through a fuzzed `z` along each block's first control (config `z[0]`, momentum `z[2]`, mass `z[6]`), at
/// eight points.
fn lines() -> Vec<(usize, Batch)> {
    let base = [0.4f32, -0.7, 0.3, -0.2, 0.5, 0.1, 0.6, -0.3];
    [0usize, 2, 6]
        .map(|k| {
            let mut q1 = [0.0; 8];
            q1[k] = 0.9;
            let chart = Latent {
                z0: base,
                q1,
                q2: [0.0; 8],
            };
            let uv = (0..8).map(|j| [j as f32 / 8.0, 0.5]).collect();
            (k, (chart, uv))
        })
        .to_vec()
}

#[test]
fn decode_stage_identities() {
    let h = harness();
    let batches = identity_batches();
    let outs = gpu_decodes::<LatentDefaults>(&h, &batches);
    let mut reached = false;
    for (i, o) in outs.iter().enumerate() {
        let at = format!("sample {i}");
        check_identities(&o.ic, &at);
        check_ranges(o, &at);
        assert!(!check_tag(o, &at), "the defaults tagged M01_TINY at {at}");
        reached |= o.descriptor.rho_angle == 0.0;
    }
    assert!(reached, "β = 0 is not reached at saturation (R-417)");
    let lines: Vec<(usize, Vec<DecodeOut<f32>>)> = lines()
        .into_iter()
        .map(|(k, b)| (k, gpu_decodes::<LatentDefaults>(&h, &[b])))
        .collect();
    check_independence(&lines);
    // Test 4 where it fires: stick_breaking along the mass corner, z_μ from 0 to 30.
    let corner = (
        at([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.0, 0.0]),
        (0..64).map(|j| [0.5, j as f32 / 63.0]).collect::<Vec<_>>(),
    );
    let mut corner = corner;
    corner.0.q2[6] = 15.0;
    corner.0.q2[7] = 15.0;
    corner.0.z0[6] = 15.0;
    corner.0.z0[7] = 15.0;
    let outs = gpu_decodes::<LatentStickBreaking>(&h, &[corner]);
    let tagged: Vec<bool> = outs
        .iter()
        .enumerate()
        .map(|(i, o)| {
            check_identities(&o.ic, &format!("stick_breaking sample {i}"));
            check_tag(o, &format!("stick_breaking sample {i}"))
        })
        .collect();
    assert!(
        tagged.contains(&true) && tagged.contains(&false),
        "the corner sweep does not cross the M01_TINY onset: {tagged:?}"
    );
}

negative_control!(
    decode_stage_identities,
    "an output whose p₀ is shifted, so Σp ≠ 0, must fail test 1",
    expected = "eps of Σ‖p‖",
    {
        let h = harness();
        let mut o = gpu_decodes::<LatentDefaults>(&h, &[golden()])[0];
        o.ic.p[0][0] += 0.25;
        check_identities(&o.ic, "the shifted golden IC")
    }
);

// ── decode_uniforms_no_recompile (REQ-DEC-007) ──────────────────────────────────────────────────────────────────

/// A navigation step: the chart before it and the chart after.
type Edit = fn(&mut Latent<f32>);

/// The edits: pan `z₀` in the plane, zoom `q₁`, `q₂`, tilt `q₁` into a third control, and change a slice value, a
/// component of `z₀` off the plane.
const EDITS: [(&str, Edit); 4] = [
    ("pan", |c| c.z0[0] += 0.25),
    ("zoom", |c| {
        c.q1[0] *= 0.5;
        c.q2[1] *= 0.5
    }),
    ("tilt", |c| c.q1[3] += 0.3),
    ("slice", |c| c.z0[5] -= 0.4),
];

/// Runs every edit in turn through `run`, a decode of the chart at four points, and checks that each edit moves the
/// decode and that no edit creates a pipeline: `pipelines_created` stays where it was after the stage was made.
fn check_no_recompile(run: &mut dyn FnMut(&Latent<f32>) -> Vec<DecodeOut<f32>>) {
    let mut chart = Latent {
        z0: [0.1, -0.2, 0.3, 0.0, 0.2, -0.1, 0.4, -0.5],
        q1: [0.6, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        q2: [0.0, 0.6, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    let mut before = run(&chart);
    let created = pipelines_created();
    for (what, edit) in EDITS {
        edit(&mut chart);
        let after = run(&chart);
        assert_eq!(
            pipelines_created(),
            created,
            "the {what} edit created a pipeline"
        );
        assert_ne!(after, before, "the {what} edit did not change the decode");
        before = after;
    }
}

const POINTS: [[f32; 2]; 4] = [[0.25, 0.25], [0.75, 0.25], [0.25, 0.75], [0.75, 0.75]];

#[test]
fn decode_uniforms_no_recompile() {
    let h = harness();
    let wgsl = wgsl();
    let start = pipelines_created();
    let stage = DecodeStage::<LatentDefaults>::new(h.device(), &wgsl, FastMath::default())
        .expect("the pipeline");
    assert_eq!(
        pipelines_created(),
        start + 1,
        "making the stage is one pipeline creation"
    );
    check_no_recompile(&mut |c| {
        stage
            .run(h.device(), h.queue(), c, DecodeMode::Full, &POINTS)
            .expect("the dispatch")
    });
}

negative_control!(
    decode_uniforms_no_recompile,
    "a dispatch that builds its pipeline per edit must fail",
    expected = "the pan edit created a pipeline",
    {
        let h = harness();
        let wgsl = wgsl();
        check_no_recompile(&mut |c| {
            DecodeStage::<LatentDefaults>::new(h.device(), &wgsl, FastMath::default())
                .expect("the pipeline")
                .run(h.device(), h.queue(), c, DecodeMode::Full, &POINTS)
                .expect("the dispatch")
        })
    }
);

// ── decode_stage_descriptor (REQ-TOOL-030) ──────────────────────────────────────────────────────────────────────

/// REQ-DEC-043's factor, as `fixtures/gates/decode-f32-eps/gate.json` proposes it (provisional until the M2 gate).
fn factor() -> f64 {
    let text = std::fs::read_to_string(root().join("fixtures/gates/decode-f32-eps/gate.json"))
        .expect("gate.json");
    let json: serde_json::Value = serde_json::from_str(&text).expect("gate.json is JSON");
    json["threshold"]["value"]
        .as_f64()
        .expect("a numeric threshold")
}

/// The GPU's descriptors against the f64 decode's, each of the twelve fields within `factor` f32 eps (the gate's
/// measure, `validation::decode_f32_eps::error`), and its masses the decode's masses, bit for bit.
fn check_descriptors(gpu: &[DecodeOut<f32>], cpu: &[DecodeOut<f64>], factor: f64) {
    let m = measure(gpu, cpu).expect("the measurement");
    for (k, w) in m.worst.iter().enumerate().skip(15) {
        assert!(
            w.eps <= factor,
            "ICDescriptor {} is {:.2} eps from the f64 decode at sample {}, beyond {factor}",
            QUANTITIES[k],
            w.eps,
            w.at
        );
    }
    for (i, o) in gpu.iter().enumerate() {
        let d = &o.descriptor;
        assert_eq!(
            [d.m0, d.m1, d.m2].map(f32::to_bits),
            o.ic.m.map(f32::to_bits),
            "the descriptor's masses are not the decode's at sample {i}"
        );
        assert_eq!(values(o)[15..18], values(o)[0..3], "sample {i}");
    }
}

/// `+∞`, `−∞` and `+0`'s bits at f32 and f64.
const POS32: u32 = 0x7F80_0000;
const NEG32: u32 = 0xFF80_0000;
const POS64: u64 = 0x7FF0_0000_0000_0000;
const NEG64: u64 = 0xFFF0_0000_0000_0000;

/// Equal masses, `r₀ = (−a, 0)`, `r₁ = (a, 0)`, `r₂ = (0, b)`, moving: `b = 0` is `‖λ̃‖ = 0`, `a = 0` a pair separation 0.
fn pole_ic(a: f64, b: f64) -> Ic<f64> {
    let t = 1.0 / 3.0;
    Ic {
        m: [t, t, t],
        r: [[-a, 0.0], [a, 0.0], [0.0, b]],
        p: [[0.5, 0.0], [-0.25, 0.1], [-0.25, -0.1]],
    }
}

fn narrow(ic: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: ic.m.map(|x| x as f32),
        r: ic.r.map(|v| v.map(|x| x as f32)),
        p: ic.p.map(|v| v.map(|x| x as f32)),
    }
}

#[test]
fn decode_stage_descriptor() {
    let h = harness();
    let mut batches = vec![golden()];
    batches.extend(interior(Fuzz {
        seed: 2,
        charts: 32,
        per_chart: 64,
    }));
    check_descriptors(
        &gpu_decodes::<LatentDefaults>(&h, &batches),
        &f64_decodes(&batches),
        factor(),
    );

    // The α = 0 pole through the decode: z_α = −1000 underflows the link at f32 and f64 alike, and with equal masses
    // (z_μ = 0) the f64 decode leaves λ̃ exactly 0, so its rho_ratio is +∞'s bits. The GPU's ‖λ̃‖ there is the rounding
    // its reconstruction leaves, or 0 (dd_decoder §3.6; a backend may contract the inner pair's centre of mass into an
    // fma): its rho_ratio is +∞'s bits where its ‖λ̃‖ is 0, and the quotient elsewhere, never a value in between.
    let pole = vec![(
        at([-1000.0, 0.3, 0.2, -0.1, 0.4, 0.1, 0.0, 0.0]),
        vec![[0.5, 0.5]],
    )];
    let (g, c) = (
        &gpu_decodes::<LatentDefaults>(&h, &pole)[0],
        &f64_decodes(&pole)[0],
    );
    assert_eq!(
        c.descriptor.lambda_mag, 0.0,
        "‖λ̃‖ is not 0 at z_α = −1000 at f64"
    );
    assert_eq!(
        c.descriptor.rho_ratio.to_bits(),
        POS64,
        "the f64 decode's rho_ratio is not +∞'s bits"
    );
    let d = &g.descriptor;
    let want = if d.lambda_mag == 0.0 {
        POS32
    } else {
        (d.rho_mag / d.lambda_mag).to_bits()
    };
    assert_eq!(
        d.rho_ratio.to_bits(),
        want,
        "the GPU's rho_ratio at z_α = −1000, ‖λ̃‖ = {:e}",
        d.lambda_mag
    );

    // The descriptor writer on the GPU at each pole, against the f64 descriptor of the same (m, r, p): a pair
    // separation 0 (r₀ = r₁), where V_0 is −∞ for the whole sum and virial_ratio 0; ‖λ̃‖ = 0; and a step off each.
    let ics = [
        pole_ic(0.0, 1.0),
        pole_ic(0.5, 0.0),
        pole_ic(1e-3, 1.0),
        pole_ic(0.5, 1e-3),
    ];
    let narrowed: Vec<Ic<f32>> = ics.iter().map(narrow).collect();
    let gpu = describe_gpu(
        h.device(),
        h.queue(),
        &wgsl(),
        &narrowed,
        FastMath::default(),
    )
    .expect("describe");
    let cpu: Vec<_> = ics.iter().map(describe::<f64>).collect();
    let [collision, lambda0, near_collision, near_lambda0] = [0, 1, 2, 3];
    assert_eq!(
        (gpu[collision].V_0.to_bits(), cpu[collision].V_0.to_bits()),
        (NEG32, NEG64),
        "V_0 is not −∞'s bits where a pair separation is 0"
    );
    assert_eq!(
        (
            gpu[collision].virial_ratio.to_bits(),
            cpu[collision].virial_ratio.to_bits()
        ),
        (0, 0),
        "virial_ratio is not +0 where V_0 is non-finite"
    );
    assert_eq!(
        gpu[collision].r_min_pair_0.to_bits(),
        0,
        "r_min_pair_0 is not 0 at the collision pole"
    );
    assert_eq!(
        (
            gpu[lambda0].rho_ratio.to_bits(),
            cpu[lambda0].rho_ratio.to_bits()
        ),
        (POS32, POS64),
        "rho_ratio is not +∞'s bits where ‖λ̃‖ = 0"
    );
    for (i, what) in [(near_collision, "V_0"), (near_lambda0, "rho_ratio")] {
        let (g, c) = (&gpu[i], &cpu[i]);
        let (x, y) = if what == "V_0" {
            (g.V_0, c.V_0)
        } else {
            (g.rho_ratio, c.rho_ratio)
        };
        assert!(
            x.is_finite()
                && (f64::from(x) - y).abs()
                    <= factor() * f64::from(f32::EPSILON) * y.abs().max(1.0),
            "{what} one step off its pole is {x}, the f64 descriptor's {y}"
        );
    }
}

negative_control!(
    decode_stage_descriptor,
    "an f64 reference decoded at a z_α 10⁻³ away must fail the descriptor check",
    expected = "eps from the f64 decode",
    {
        let h = harness();
        let batches = interior(Fuzz {
            seed: 2,
            charts: 4,
            per_chart: 64,
        });
        let shifted: Vec<Batch> = batches
            .iter()
            .map(|(c, uv)| {
                let mut c = *c;
                c.z0[0] += 1e-3;
                (c, uv.clone())
            })
            .collect();
        check_descriptors(
            &gpu_decodes::<LatentDefaults>(&h, &batches),
            &f64_decodes(&shifted),
            factor(),
        )
    }
);

// ── decode_stage_mode (REQ-DEC-007) ─────────────────────────────────────────────────────────────────────────────

/// Under `flag`, the kernel's words for three points: every tag still [`UNWRITTEN`] and every `(m, r, p)` word still
/// the NaN fill under the linearised flag; every one written under the full.
fn check_mode(flag_of_linearised: u32) {
    let h = harness();
    let stage = DecodeStage::<LatentDefaults>::new(h.device(), &wgsl(), FastMath::default())
        .expect("the pipeline");
    let chart = at([0.2, -0.1, 0.0, 0.3, -0.2, 0.1, 0.4, 0.0]);
    let uv = [[0.5, 0.5], [0.1, 0.9], [0.9, 0.1]];
    let nan = f32::NAN.to_bits();
    let w = stage
        .dispatch(h.device(), h.queue(), &chart, flag_of_linearised, &uv)
        .expect("the dispatch");
    assert!(
        w.tags.iter().all(|&t| t == UNWRITTEN)
            && w.ic.iter().chain(&w.descriptors).all(|&x| x == nan),
        "the kernel wrote under DECODE_MODE linearised, whose path lands in M6"
    );
    let w = stage
        .dispatch(h.device(), h.queue(), &chart, flags(DecodeMode::Full), &uv)
        .expect("the dispatch");
    assert!(
        w.tags.iter().all(|&t| t != UNWRITTEN) && w.ic.iter().all(|&x| x != nan),
        "the kernel left a sample unwritten under DECODE_MODE full"
    );
    let refused = stage.run(h.device(), h.queue(), &chart, DecodeMode::Linearised, &uv);
    assert!(
        refused.as_ref().is_err_and(|e| e.0.contains("lands in M6")),
        "the host dispatched a linearised quad: {refused:?}"
    );
    assert!(
        stage
            .dispatch(h.device(), h.queue(), &chart, 0, &[])
            .expect("empty")
            .tags
            .is_empty(),
        "an empty dispatch read something back"
    );
}

#[test]
fn decode_stage_mode() {
    check_mode(flags(DecodeMode::Linearised));
}

negative_control!(
    decode_stage_mode,
    "the full flag passed off as linearised must fail",
    expected = "the kernel wrote under DECODE_MODE linearised",
    check_mode(flags(DecodeMode::Full))
);

// ── decode_only_sim_key ─────────────────────────────────────────────────────────────────────────────────────────

/// A `decode_only` on the latent chart: the chart, the point and the sim key, to the f64 decode or none.
type DecodeOnly =
    fn(&Latent<f32>, [f64; 2], &engine::contract::sim_config::SimConfig) -> Option<DecodeOut<f64>>;

/// `decode` is the stage's source at f64 on the chart's f32 parameters widened, under the physics kernel, and
/// refuses the bring-up mode.
fn check_decode_only(decode: DecodeOnly) {
    let chart = Latent {
        z0: [0.3, 0.1, -0.2, 0.4, 0.0, -0.3, 0.5, 0.2],
        q1: [0.1; 8],
        q2: [-0.05; 8],
    };
    let uv = [0.3, 0.6];
    let mut key = physics_key();
    let want = sample::<_, Defaults, f64>(&chart, 0.3, 0.6);
    assert_eq!(
        decode(&chart, uv, &key),
        Some(want),
        "decode_only is not the stage at f64"
    );
    let stick = sample::<_, StickBreakingLinks, f64>(&chart, 0.3, 0.6);
    assert_ne!(want, stick, "the link set does not reach the decode");
    key.kernel_variant = KernelVariant::BringUp;
    assert_eq!(
        decode(&chart, uv, &key),
        None,
        "decode_only decoded under the bring-up mode"
    );
}

#[test]
fn decode_only_sim_key() {
    check_decode_only(|c, uv, k| decode_only::<_, Defaults>(c, uv, k).ok());
}

negative_control!(
    decode_only_sim_key,
    "a decode_only that narrows the chart's parameters to f32's decode must fail",
    expected = "decode_only is not the stage at f64",
    check_decode_only(|c, uv, _| {
        let o = sample::<_, Defaults, f32>(c, uv[0] as f32, uv[1] as f32);
        let w = |x: f32| f64::from(x);
        Some(DecodeOut {
            ic: Ic {
                m: o.ic.m.map(w),
                r: o.ic.r.map(|v| v.map(w)),
                p: o.ic.p.map(|v| v.map(w)),
            },
            descriptor: describe::<f64>(&Ic {
                m: o.ic.m.map(w),
                r: o.ic.r.map(|v| v.map(w)),
                p: o.ic.p.map(|v| v.map(w)),
            }),
            degenerate: o.degenerate,
        })
    })
);
