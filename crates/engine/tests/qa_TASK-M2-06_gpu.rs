//! QA tests for TASK-M2-06 on the GPU, written from the requirements, not from the implementation: the kernel's decode
//! stage, `target/spirv/kernel.wgsl` (`cargo xtask build-kernel`), dispatched natively and read back, against the same
//! source at f64 (`decode_only`, parity_contract §1). They need the built kernel, so they belong to the `gpu-kernel`
//! profile with `decode_stage_gpu`.
//! - REQ-DEC-047 (the verify detail: "the decode stage's range and identities holding on the GPU at saturated
//!   controls"; Metal's `tanh` fails from 43.671875, 0 and then NaN): every latent component at once at ±43.671875,
//!   ±44, ±44.5, ±10³, ±10⁴, ±10³⁰ and ±f32::MAX, in random sign patterns and one component at a time, through both compiled
//!   variants (the defaults, whose mass link `softmax_tanh` reads `tanh`, and `stick_breaking`, which reads it too):
//!   every `(m, r, p)` word finite, no NaN anywhere, dd_decoder §5 test 1's identities within REQ-DEC-044's and
//!   REQ-DEC-045's proposals, the masses on the simplex (each positive, at most 1: dd_generation_root §5 test 8(a)), the
//!   descriptor's fields in dd_decoder §3.6's ranges (test 3's `β ∈ [0, π]` as the GPU can show it, `rho_angle`
//!   within `(−π̂, π̂]`), test 4's tag (`M01_TINY` exactly where the GPU's own `m₀ + m₁ < ε_M01`), and test 12's tag
//!   clause, the GPU's tag the same source's natively at f32 (`qa_m206_gpu_saturated_controls`). A descriptor field is
//!   non-finite only at its pole, as R-436 selects it. No `(m, r, p)` value is held to an eps factor there: test 12
//!   scopes its tolerance to "the golden IC and fuzzed interiors", REQ-DEC-043's domain is the defaults at `|zₖ| ≤ 2`
//!   (`validation::decode_f32_eps`), and the corpus gives no factor for saturated controls, so the differences from
//!   the native f32 and the f64 decodes, and the tags that differ from f64's, are printed, not asserted.
//! - REQ-DEC-047's bound on the GPU: §3.9's `tanh` operator as the kernel evaluates it (`stage_decode_link_tanh`), past
//!   `tanh_sat`, is in `[−1, 1]`, of its argument's sign, and the operator's own bits at `±tanh_sat`, its clamp's
//!   definition (`qa_m206_gpu_saturated_tanh_bound`).
//! - REQ-DEC-047's window on this backend: the GPU's own `tanh`, the compute entry point's path, measured on a sweep;
//!   `tanh_sat` lies below the first argument where it fails as Metal's does, non-finite or not of its argument's sign
//!   (0 included), and it has not failed at `±tanh_sat` (`qa_m206_gpu_tanh_window`).
//! - REQ-TOOL-030 / R-436: the descriptor writer on the GPU at each pole, each pair's collision and all three bodies,
//!   `‖λ̃‖ = 0`, and one step off each: `V_0` `−∞`'s bits, `virial_ratio` `+0`'s, `rho_ratio` `+∞`'s exactly where the
//!   GPU's own `‖λ̃‖` is 0 and the quotient elsewhere, the f64 descriptor's bits at the poles
//!   (`qa_m206_gpu_pole_bits`).
//! - REQ-DEC-007: a sequence of navigation edits (pan, zoom, tilt, slice, a component to the domain's edge) creates no
//!   pipeline, and after each the GPU's decode is the f64 decode of the edited chart within REQ-DEC-043's factor, every
//!   decoded `|zₖ| ≤ 2`, its domain, so each edit reached the kernel through the uniform
//!   (`qa_m206_gpu_edits_reach_the_kernel`); `DECODE_MODE` is bit 0 of the flags word on the GPU, the other 31
//!   bits ignored (`qa_m206_gpu_decode_mode_bit0`).
//! - REQ-DEC-007 / REQ-CHART-001: the link set is baked per variant: the same chart and points through the two entry
//!   points decode differently, each sample nearer its own selection's f64 decode than the other's, and the defaults'
//!   within REQ-DEC-043's factor of theirs (its domain is the defaults; the corpus gives `stick_breaking` none)
//!   (`qa_m206_gpu_variants_baked`).
//! - REQ-DEC-043: the gate fails on its checked-in perturbed input, its verdict turns at its threshold, and refuses the
//!   inputs that measure nothing (`qa_m206_gate_*`).
//!
//! Tier N (parity_contract §2): each comparison feeds both pipelines the same f32 inputs, widened for f64, and compares
//! at once, never accumulating (§3).
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use std::path::{Path, PathBuf};

use engine::compute::{pipeline, pipelines_created, ComputeShader};
use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{decode_only, describe_gpu, DecodeStage, UNWRITTEN};
use kernel::chart::Latent;
use kernel::decode::descriptor::describe;
use kernel::decode::{Degenerate, Ic};
use kernel::generated::constants::tanh_sat;
use kernel::stage::decode::{
    sample, DecodeMode, DecodeOut, DecodeVariant, LatentDefaults, LatentStickBreaking,
};
use validation::decode_f32_eps::{
    error, interior, physics_key, Batch, DecodeF32EpsGate, Fuzz, Input, Perturb, Points,
};
use validation::gate::{GateConfig, Verdict};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn wgsl() -> String {
    std::fs::read_to_string(root().join("target/spirv/kernel.wgsl"))
        .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

fn harness() -> GpuHarness {
    GpuHarness::new().expect("a GPU device")
}

const EPS: f64 = f32::EPSILON as f64;

/// The gate's configuration as checked in: REQ-DEC-043's proposed factor (provisional until the M2 gate).
fn config() -> GateConfig {
    validation::gate::config(&root(), "decode-f32-eps").expect("gate.json")
}

/// REQ-DEC-043's factor, in f32 eps.
fn factor() -> f64 {
    config().threshold.value
}

/// REQ-DEC-044's proposals for test 1 (`crates/kernel/tests/decode_blocks.rs`' header): `Σmᵢ − 1` 4 eps, the CoM 4
/// eps of `Σmᵢ‖rᵢ‖`, `I − 1` 8 eps; REQ-DEC-045's for `Σpᵢ`: 4 eps of `Σ‖pᵢ‖`.
const SUM_M_EPS: f64 = 4.0;
const COM_EPS: f64 = 4.0;
const INERTIA_EPS: f64 = 8.0;
const SUM_P_EPS: f64 = 4.0;

/// The GPU decodes of `batches` through variant `V`'s entry.
fn gpu<V: DecodeVariant<Chart = Latent<f32>>>(
    h: &GpuHarness,
    batches: &[Batch],
) -> Vec<DecodeOut<f32>> {
    let stage = DecodeStage::<V>::new(h.device(), &wgsl(), FastMath::default()).expect("the stage");
    batches
        .iter()
        .flat_map(|(c, uv)| {
            stage
                .run(h.device(), h.queue(), c, DecodeMode::Full, uv)
                .expect("the dispatch")
        })
        .collect()
}

/// The f64 decodes of `batches` under `V`'s selection.
fn f64s<V: DecodeVariant<Chart = Latent<f32>>>(batches: &[Batch]) -> Vec<DecodeOut<f64>> {
    let key = physics_key();
    batches
        .iter()
        .flat_map(|(c, uv)| {
            uv.iter()
                .map(|p| {
                    decode_only::<_, V::Links>(c, p.map(f64::from), &key).expect("decode_only")
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// `(m, r, p)` as 15 values, widened.
fn mrp<R: Copy + Into<f64>>(ic: &Ic<R>) -> [f64; 15] {
    let mut v = [0.0; 15];
    let flat =
        ic.m.iter()
            .chain(ic.r.iter().flatten())
            .chain(ic.p.iter().flatten());
    for (s, x) in v.iter_mut().zip(flat) {
        *s = (*x).into();
    }
    v
}

// ── Saturated controls (REQ-DEC-047, REQ-DEC-005) ───────────────────────────────────────────────────────────────

/// The magnitudes: from Metal's failure (43.671875, where its `tanh` gives 0, then 44 and 44.5), past f32's and f64's
/// `exp` overflow, and to f32's largest.
const MAGNITUDES: [f32; 7] = [43.671_875, 44.0, 44.5, 1e3, 1e4, 1e30, f32::MAX];

/// Charts at saturation: every component at `±s` in all-plus, all-minus and 16 random sign patterns per magnitude;
/// and each component alone at `±s`, the others drawn on `[−2, 2]`. Each decoded at the chart's centre and corners
/// (`q₁`, `q₂` 0 or 1, so the corners move the controls further).
fn saturated() -> Vec<Batch> {
    let mut rng = validation::decode_f32_eps::SplitMix::new(2026);
    let mut out = Vec::new();
    let uv = vec![[0.5, 0.5], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for s in MAGNITUDES {
        let mut patterns: Vec<[f32; 8]> = vec![[s; 8], [-s; 8]];
        for _ in 0..16 {
            patterns.push(core::array::from_fn(|_| {
                if rng.next_u64() & 1 == 0 {
                    s
                } else {
                    -s
                }
            }));
        }
        for z in patterns {
            out.push((
                Latent {
                    z0: z,
                    q1: [0.0; 8],
                    q2: [0.0; 8],
                },
                vec![[0.5, 0.5]],
            ));
        }
        for k in 0..8 {
            for sign in [1.0, -1.0] {
                let mut z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
                z[k] = sign * s;
                let mut q1 = [0.0; 8];
                q1[k] = sign;
                out.push((
                    Latent {
                        z0: z,
                        q1,
                        q2: [0.0; 8],
                    },
                    uv.clone(),
                ));
            }
        }
    }
    out
}

/// The worst `(m, r, p)` differences seen: the GPU against the same source at f32 natively (the backend's share), and
/// the GPU against f64 (f32's own conditioning included), and the tags the f32 and f64 decodes disagree on.
#[derive(Default)]
struct Report {
    backend: f64,
    backend_at: String,
    precision: f64,
    precision_at: String,
    tag_differs: usize,
}

/// The f32 image of π the descriptor's `rho_angle` stays within, `fl↓(π)` (dd_decoder §3.6; R-417).
const PI_HAT: f32 = f32::from_bits(0x4049_0FDA);

/// One GPU output `g` at saturation, against the same source at f32 natively, `n`, and at f64, `c`: the `(m, r, p)`
/// words finite; no NaN in the descriptor, and a non-finite descriptor field only at its pole (R-436); test 1 within
/// REQ-DEC-044/045; the masses on the simplex, each positive and at most 1 (dd_generation_root §5 test 8(a)); the
/// descriptor's fields in dd_decoder §3.6's ranges (`q_mass` in `(0, 1]`, `rho_angle` in `(−π̂, π̂]`, the magnitudes,
/// `r_min_pair_0`, `K_0` and `virial_ratio` at least 0, `V_0` at most 0); `M01_TINY` exactly where the GPU's own
/// `m₀ + m₁ < ε_M01` (dd_decoder §5 test 4); and the GPU's tag the native f32 decode's (test 12's tag clause).
/// The `(m, r, p)` differences from the native f32 and the f64 decodes are recorded, not asserted: REQ-DEC-043's factor
/// holds on its own domain, the defaults at `|zₖ| ≤ 2`, and the corpus gives none at saturation, where f32's own
/// rounding dominates (the bodies of a tiny pair sit at `|r| ≈ 10³`, where `sin β` at β's f32 image of π is `10⁻⁷` of
/// it).
fn check_saturated(
    g: &DecodeOut<f32>,
    n: &DecodeOut<f32>,
    c: &DecodeOut<f64>,
    at: &str,
    report: &mut Report,
) {
    let v = mrp(&g.ic);
    assert!(
        v.iter().all(|x| x.is_finite()),
        "a non-finite (m, r, p) on the GPU at {at}: {v:?}"
    );
    let d = &g.descriptor;
    let fields = [
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
    ];
    assert!(
        fields.iter().all(|x| !x.is_nan()),
        "a NaN in the GPU's descriptor at {at}: {d:?}"
    );
    assert!(
        d.rho_ratio.is_finite() || (d.rho_ratio.to_bits() == 0x7F80_0000 && d.lambda_mag == 0.0),
        "rho_ratio = {} with ‖λ̃‖ = {:e} at {at}",
        d.rho_ratio,
        d.lambda_mag
    );
    assert!(
        d.V_0.is_finite() || (d.V_0.to_bits() == 0xFF80_0000 && d.r_min_pair_0 == 0.0),
        "V_0 = {} with r_min = {:e} at {at}",
        d.V_0,
        d.r_min_pair_0
    );
    for (what, x) in [
        ("q_mass", d.q_mass),
        ("rho_mag", d.rho_mag),
        ("lambda_mag", d.lambda_mag),
        ("rho_angle", d.rho_angle),
        ("K_0", d.K_0),
        ("virial_ratio", d.virial_ratio),
        ("r_min_pair_0", d.r_min_pair_0),
    ] {
        assert!(x.is_finite(), "{what} = {x} on the GPU at {at}");
    }
    // dd_decoder §3.6's ranges.
    for (what, ok) in [
        ("q_mass in (0, 1]", d.q_mass > 0.0 && d.q_mass <= 1.0),
        (
            "rho_angle in (−π̂, π̂]",
            d.rho_angle > -PI_HAT && d.rho_angle <= PI_HAT,
        ),
        ("rho_mag ≥ 0", d.rho_mag >= 0.0),
        ("lambda_mag ≥ 0", d.lambda_mag >= 0.0),
        ("rho_ratio ≥ 0", d.rho_ratio >= 0.0),
        ("r_min_pair_0 ≥ 0", d.r_min_pair_0 >= 0.0),
        ("K_0 ≥ 0", d.K_0 >= 0.0),
        ("virial_ratio ≥ 0", d.virial_ratio >= 0.0),
        ("V_0 ≤ 0", d.V_0 <= 0.0),
    ] {
        assert!(ok, "the GPU's descriptor breaks {what} at {at}: {d:?}");
    }
    // Test 1 (dd_decoder §5) on the GPU's output.
    let m = [v[0], v[1], v[2]];
    let r = [[v[3], v[4]], [v[5], v[6]], [v[7], v[8]]];
    let p = [[v[9], v[10]], [v[11], v[12]], [v[13], v[14]]];
    let sum = (m[0] + m[1] + m[2] - 1.0).abs() / EPS;
    assert!(sum <= SUM_M_EPS, "Σm − 1 = {sum} eps at {at}");
    assert!(
        m.iter().all(|&x| x > 0.0 && x <= 1.0),
        "the masses {m:?} leave the simplex at {at}"
    );
    let size: f64 = (0..3).map(|i| m[i] * r[i][0].hypot(r[i][1])).sum();
    let com = [0, 1].map(|k| (0..3).map(|i| m[i] * r[i][k]).sum::<f64>());
    let e = com[0].hypot(com[1]) / (size * EPS);
    assert!(e <= COM_EPS, "the CoM is {e} eps of Σm‖r‖ at {at}");
    let inertia: f64 = (0..3)
        .map(|i| m[i] * (r[i][0].powi(2) + r[i][1].powi(2)))
        .sum();
    let e = (inertia - 1.0).abs() / EPS;
    assert!(e <= INERTIA_EPS, "I − 1 = {e} eps at {at}");
    let scale: f64 = p.iter().map(|q| q[0].hypot(q[1])).sum();
    for (k, s) in [0, 1].map(|k| (k, (p[0][k] + p[1][k] + p[2][k]).abs())) {
        let e = if scale == 0.0 {
            s / EPS
        } else {
            s / (scale * EPS)
        };
        assert!(e <= SUM_P_EPS, "Σp_{k} is {e} eps of Σ‖p‖ at {at}");
    }
    let m01 = g.ic.m[0] + g.ic.m[1];
    assert_eq!(
        g.degenerate == Some(Degenerate::M01Tiny),
        m01 < kernel::decode::mass::m01_epsilon::<f32>(),
        "M01_TINY is not exactly where M₀₁ = {m01:e} < ε_M01 at {at}: the tag is {:?}",
        g.degenerate
    );
    assert_eq!(
        g.degenerate, n.degenerate,
        "the GPU's tag is not the f32 decode's at {at}"
    );
    if g.degenerate != c.degenerate {
        report.tag_differs += 1;
    }
    for (k, ((a, b), w)) in v.iter().zip(mrp(&n.ic)).zip(mrp(&c.ic)).enumerate() {
        let e = error(*a, b);
        if e > report.backend {
            report.backend = e;
            report.backend_at = format!("value {k} at {at}");
        }
        let e64 = error(*a, w);
        if e64 > report.precision {
            report.precision = e64;
            report.precision_at = format!("value {k} at {at}");
        }
    }
}

fn run_saturated<V: DecodeVariant<Chart = Latent<f32>>>(h: &GpuHarness) -> Report {
    let batches = saturated();
    let (g, c) = (gpu::<V>(h, &batches), f64s::<V>(&batches));
    let n: Vec<DecodeOut<f32>> = batches
        .iter()
        .flat_map(|(chart, uv)| {
            uv.iter()
                .map(|p| sample::<_, V::Links, f32>(chart, p[0], p[1]))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!((g.len(), n.len()), (c.len(), c.len()));
    let mut report = Report::default();
    for (i, ((g, n), c)) in g.iter().zip(&n).zip(&c).enumerate() {
        check_saturated(g, n, c, &format!("{} sample {i}", V::ENTRY), &mut report);
    }
    println!(
        "{}: {} saturated samples; GPU against native f32: worst {:.2} eps ({}); GPU against f64: worst {:.2} eps \
         ({}); f32 and f64 tags differ at {}",
        V::ENTRY,
        g.len(),
        report.backend,
        report.backend_at,
        report.precision,
        report.precision_at,
        report.tag_differs
    );
    report
}

#[test]
fn qa_m206_gpu_saturated_controls() {
    let h = harness();
    println!("adapter: {}", h.adapter_info());
    run_saturated::<LatentDefaults>(&h);
    run_saturated::<LatentStickBreaking>(&h);
}

negative_control!(
    qa_m206_gpu_saturated_controls,
    "a GPU output with a NaN in its momenta, as Metal's unclamped tanh gives, must fail",
    expected = "a non-finite (m, r, p) on the GPU",
    {
        let h = harness();
        let batches = saturated();
        let mut g = gpu::<LatentDefaults>(&h, &batches[..1]);
        let c = f64s::<LatentDefaults>(&batches[..1]);
        let n = g[0];
        g[0].ic.p[1][0] = f32::NAN;
        check_saturated(&g[0], &n, &c[0], "the NaN'd sample", &mut Report::default())
    }
);

// ── The operator's bound on the GPU (REQ-DEC-047) ──────────────────────────────────────────────────────────────

/// Past `tanh_sat`: Metal's onset 43.671875, the saturated magnitudes, and the bound itself, each signed.
fn bound_arguments(t: f32) -> Vec<f32> {
    let past = [t, 20.5].into_iter().chain(MAGNITUDES);
    past.flat_map(|x| [x, -x]).collect()
}

/// Each of `ys`, §3.9's `tanh` operator at `xs`, all at or past `±t`: not NaN, in `[−1, 1]`, of its argument's sign
/// and not 0 (`tanh` is 0 only at 0), and the operator's own bits at `±t`, since the clamp gives every argument past
/// the bound the bound itself (dd_generation_root §3.9's definition).
fn check_tanh_bound(t: f32, xs: &[f32], ys: &[f32]) {
    assert_eq!(xs.len(), ys.len(), "the operator's outputs");
    let at = |s: f32| {
        xs.iter()
            .zip(ys)
            .find(|(x, _)| **x == s)
            .map(|(_, y)| *y)
            .expect("the arguments hold ±tanh_sat")
    };
    let (hi, lo) = (at(t), at(-t));
    for (&x, &y) in xs.iter().zip(ys) {
        assert!(
            !y.is_nan() && y.abs() <= 1.0 && y * x.signum() > 0.0,
            "the GPU's tanh operator at {x:e} is {y:e} (bits {:#x}): not in [-1, 1] with its argument's sign",
            y.to_bits()
        );
        let want = if x > 0.0 { hi } else { lo };
        assert_eq!(
            y.to_bits(),
            want.to_bits(),
            "the GPU's tanh operator at {x:e} is not its value at the bound"
        );
    }
}

#[test]
fn qa_m206_gpu_saturated_tanh_bound() {
    let h = harness();
    let t = tanh_sat::<f32>();
    let xs = bound_arguments(t);
    let ys = gpu_tanh(&h, &wgsl(), "stage_decode_link_tanh", &xs);
    println!(
        "adapter: {}; the operator at ±tanh_sat: {:?}",
        h.adapter_info(),
        &ys[..2]
    );
    check_tanh_bound(t, &xs, &ys);
}

negative_control!(
    qa_m206_gpu_saturated_tanh_bound,
    "the GPU's own tanh, unclamped, which Metal gives 0 at 43.671875, must fail",
    expected = "not in [-1, 1] with its argument's sign",
    {
        let h = harness();
        let t = tanh_sat::<f32>();
        let xs = bound_arguments(t);
        let mut ys = gpu_tanh(&h, &wgsl(), "stage_decode_link_tanh", &xs);
        ys[4] = 0.0;
        check_tanh_bound(t, &xs, &ys)
    }
);

// ── The GPU's own tanh against tanh_sat (REQ-DEC-047) ───────────────────────────────────────────────────────────

const TANH_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> xs: array<f32>;
@group(0) @binding(1) var<storage, read_write> ys: array<f32>;

@compute @workgroup_size(64)
fn qa_tanh(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i < arrayLength(&xs)) {
        ys[i] = tanh(xs[i]);
    }
}
";

/// The GPU's `entry` in `wgsl`, one f32 in and one out per word, at each of `xs`, through the compute entry point's
/// path (fast-math off, as the stage is built).
fn gpu_tanh(h: &GpuHarness, wgsl: &str, entry: &str, xs: &[f32]) -> Vec<f32> {
    use wgpu::util::DeviceExt;
    let device = h.device();
    let kernel = pipeline(
        device,
        &ComputeShader {
            label: "qa tanh",
            wgsl,
            entry,
        },
        FastMath::default(),
    )
    .expect("the tanh probe's pipeline");
    let bytes = |w: &[f32]| w.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("qa tanh in"),
        contents: &bytes(xs),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("qa tanh out"),
        contents: &bytes(&vec![f32::NAN; xs.len()]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let size = output.size();
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("qa tanh readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bindings = kernel.bind(device, &[&[&input, &output]]).expect("bind");
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        kernel.set(&mut pass, &bindings);
        pass.dispatch_workgroups((xs.len() as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, size);
    h.queue().submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let view = slice.get_mapped_range().expect("range");
    let out = view
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    drop(view);
    readback.unmap();
    out
}

/// The sweep: 0 to 200 in steps of 1/64, then 10³ … 10³⁸ and f32's largest, both signs.
fn sweep() -> Vec<f32> {
    let mut xs: Vec<f32> = (0..=12_800).map(|i| i as f32 / 64.0).collect();
    xs.extend((3..=38).map(|e| 10f32.powi(e)));
    xs.push(f32::MAX);
    let neg: Vec<f32> = xs.iter().map(|x| -x).collect();
    xs.extend(neg);
    xs
}

/// Where `ys = tanh(xs)` on the GPU: `t` lies below the least `|x|` at which the GPU's `tanh` fails as Metal's was
/// measured to (REQ-DEC-047: 0, then NaN), a non-finite value or one not of its argument's sign, 0 included (`tanh`
/// is odd and 0 only at 0), and at `±t` it has not failed. No accuracy factor is asserted: REQ-DEC-043's is the
/// decode's, on its own domain.
fn check_tanh_window(t: f32, xs: &[f32], ys: &[f32]) {
    let fails = |x: f32, y: f32| x != 0.0 && (!y.is_finite() || y * x.signum() <= 0.0);
    let first_fail = xs
        .iter()
        .zip(ys)
        .filter(|&(&x, &y)| fails(x, y))
        .map(|(x, _)| x.abs())
        .fold(f32::INFINITY, f32::min);
    println!("tanh_sat = {t}; the GPU's tanh first fails at |x| = {first_fail}");
    assert!(
        t < first_fail,
        "tanh_sat = {t} is not below where this GPU's tanh fails, |x| = {first_fail}"
    );
    let at: Vec<_> = xs.iter().zip(ys).filter(|(x, _)| x.abs() == t).collect();
    assert!(
        at.iter().any(|(x, _)| **x > 0.0) && at.iter().any(|(x, _)| **x < 0.0),
        "the sweep holds ±tanh_sat"
    );
    for (&x, &y) in at {
        assert!(!fails(x, y), "the GPU's tanh({x}) = {y} fails");
    }
}

#[test]
fn qa_m206_gpu_tanh_window() {
    let h = harness();
    println!("adapter: {}", h.adapter_info());
    let t = tanh_sat::<f32>();
    let mut xs = sweep();
    xs.extend([t, -t]);
    let ys = gpu_tanh(&h, TANH_WGSL, "qa_tanh", &xs);
    check_tanh_window(t, &xs, &ys);
}

negative_control!(
    qa_m206_gpu_tanh_window,
    "a GPU tanh that is 0 from 43.671875, as Metal's measured, against a bound of 50, must fail",
    expected = "is not below where this GPU's tanh fails",
    {
        let mut xs = sweep();
        xs.extend([50.0, -50.0]);
        let ys: Vec<f32> = xs
            .iter()
            .map(|&x| if x.abs() >= 43.671_875 { 0.0 } else { x.tanh() })
            .collect();
        check_tanh_window(50.0, &xs, &ys)
    }
);

// ── The poles on the GPU (REQ-TOOL-030, R-436) ─────────────────────────────────────────────────────────────────

/// Equal masses at `r`, momenta summing to 0.
fn ic32(r: [[f32; 2]; 3]) -> Ic<f32> {
    let t = 1.0f32 / 3.0;
    Ic {
        m: [t, t, t],
        r,
        p: [[0.5, 0.0], [-0.25, 0.1], [-0.25, -0.1]],
    }
}

fn widen(ic: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: ic.m.map(f64::from),
        r: ic.r.map(|v| v.map(f64::from)),
        p: ic.p.map(|v| v.map(f64::from)),
    }
}

/// The poles and a step off each: each pair coincident, all three, `‖λ̃‖ = 0` with `ρ̃ ≠ 0`, and with both 0.
const COLLIDE: [[[f32; 2]; 3]; 4] = [
    [[0.25, 0.0], [0.25, 0.0], [0.0, 1.0]],
    [[0.25, 0.0], [-0.5, 0.0], [0.25, 0.0]],
    [[0.0, 1.0], [0.25, 0.5], [0.25, 0.5]],
    [[0.1, 0.2], [0.1, 0.2], [0.1, 0.2]],
];
const NEAR: [[[f32; 2]; 3]; 3] = [
    [[0.25, 0.0], [0.25 + 1e-3, 0.0], [0.0, 1.0]],
    [[-0.5, 0.0], [0.5, 0.0], [0.0, 1e-3]],
    [[-0.5, 0.0], [0.5, 0.0], [0.0, 1e-15]],
];
const LAMBDA0: [[[f32; 2]; 3]; 2] = [
    [[-0.5, 0.0], [0.5, 0.0], [0.0, 0.0]],
    [[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]],
];

/// The GPU's descriptors `gpu` of `COLLIDE`, `LAMBDA0`, `NEAR`, in that order, against the f64 descriptors of the same
/// `(m, r, p)`.
fn check_pole_bits(gpu: &[kernel::payload::ICDescriptor], factor: f64) {
    let all: Vec<[[f32; 2]; 3]> = COLLIDE
        .iter()
        .chain(&LAMBDA0)
        .chain(&NEAR)
        .copied()
        .collect();
    let cpu: Vec<_> = all
        .iter()
        .map(|r| describe::<f64>(&widen(&ic32(*r))))
        .collect();
    for (k, g) in gpu.iter().enumerate() {
        // The select follows the GPU's own divisor, on both sides.
        assert_eq!(
            g.rho_ratio.to_bits() == 0x7F80_0000,
            g.lambda_mag == 0.0,
            "IC {k}: rho_ratio = {} where the GPU's ‖λ̃‖ = {:e}",
            g.rho_ratio,
            g.lambda_mag
        );
        assert_eq!(
            g.V_0.to_bits() == 0xFF80_0000,
            g.r_min_pair_0 == 0.0,
            "IC {k}: V_0 = {} where the GPU's r_min = {:e}",
            g.V_0,
            g.r_min_pair_0
        );
        if g.V_0.to_bits() == 0xFF80_0000 {
            assert_eq!(
                g.virial_ratio.to_bits(),
                0,
                "IC {k}: virial_ratio is not +0's bits"
            );
        }
    }
    for k in 0..COLLIDE.len() {
        assert_eq!(
            (gpu[k].V_0.to_bits(), cpu[k].V_0.to_bits()),
            (0xFF80_0000, 0xFFF0_0000_0000_0000),
            "collision {k}: V_0 is not −∞'s bits at both widths"
        );
        assert_eq!(
            (gpu[k].virial_ratio.to_bits(), cpu[k].virial_ratio.to_bits()),
            (0, 0),
            "collision {k}: virial_ratio"
        );
    }
    for k in COLLIDE.len()..COLLIDE.len() + LAMBDA0.len() {
        assert_eq!(
            (gpu[k].rho_ratio.to_bits(), cpu[k].rho_ratio.to_bits()),
            (0x7F80_0000, 0x7FF0_0000_0000_0000),
            "‖λ̃‖ = 0 at IC {k}: rho_ratio is not +∞'s bits at both widths"
        );
    }
    for k in COLLIDE.len() + LAMBDA0.len()..all.len() {
        let (g, c) = (&gpu[k], &cpu[k]);
        for (what, a, b) in [
            ("V_0", g.V_0, c.V_0),
            ("rho_ratio", g.rho_ratio, c.rho_ratio),
            ("virial_ratio", g.virial_ratio, c.virial_ratio),
        ] {
            let e = error(f64::from(a), b);
            assert!(
                a.is_finite() && e <= factor,
                "IC {k} off its pole: {what} = {a} on the GPU, {b} at f64 ({e:.1} eps)"
            );
        }
    }
}

fn pole_ics() -> Vec<Ic<f32>> {
    COLLIDE
        .iter()
        .chain(&LAMBDA0)
        .chain(&NEAR)
        .map(|r| ic32(*r))
        .collect()
}

#[test]
fn qa_m206_gpu_pole_bits() {
    let h = harness();
    let gpu = describe_gpu(
        h.device(),
        h.queue(),
        &wgsl(),
        &pole_ics(),
        FastMath::default(),
    )
    .expect("describe");
    check_pole_bits(&gpu, factor());
}

negative_control!(
    qa_m206_gpu_pole_bits,
    "a descriptor whose V_0 at a collision is the most negative finite value, not −∞, must fail",
    expected = "V_0 = ",
    {
        let h = harness();
        let mut gpu = describe_gpu(
            h.device(),
            h.queue(),
            &wgsl(),
            &pole_ics(),
            FastMath::default(),
        )
        .expect("describe");
        gpu[0].V_0 = -f32::MAX;
        check_pole_bits(&gpu, factor())
    }
);

// ── Edits reach the kernel through the uniform, and create no pipeline (REQ-DEC-007) ───────────────────────────

type Edit = (&'static str, fn(&mut Latent<f32>));

const EDITS: [Edit; 8] = [
    ("pan", |c| c.z0[1] -= 0.3),
    ("zoom in", |c| {
        c.q1 = c.q1.map(|x| x * 0.25);
        c.q2 = c.q2.map(|x| x * 0.25)
    }),
    ("tilt", |c| c.q2[4] += 0.5),
    ("slice", |c| c.z0[7] += 0.6),
    ("slice back", |c| c.z0[7] -= 0.6),
    ("to the edge", |c| c.z0[6] = 2.0),
    ("swap plane", |c| core::mem::swap(&mut c.q1, &mut c.q2)),
    ("pan far", |c| c.z0[0] = -1.8),
];

const POINTS: [[f32; 2]; 5] = [[0.5, 0.5], [0.1, 0.2], [0.9, 0.3], [0.2, 0.8], [0.7, 0.7]];

/// After each edit, `run`'s decode of the chart is the f64 decode of the edited chart, value by value within
/// `factor`, every decoded `|zₖ| ≤ 2` (REQ-DEC-043's domain), and no pipeline was created since the first decode.
fn check_edits(run: &mut dyn FnMut(&Latent<f32>) -> Vec<DecodeOut<f32>>, factor: f64) {
    let mut chart = Latent {
        z0: [0.2, 0.1, -0.3, 0.4, 0.0, 0.3, -0.2, 0.1],
        q1: [0.7, 0.0, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0],
        q2: [0.0, 0.7, 0.0, 0.0, -0.2, 0.0, 0.0, 0.0],
    };
    let _ = run(&chart);
    let created = pipelines_created();
    for (what, edit) in EDITS {
        edit(&mut chart);
        let got = run(&chart);
        assert_eq!(
            pipelines_created(),
            created,
            "the {what} edit created a pipeline"
        );
        for [u, v] in POINTS {
            for k in 0..8 {
                let z = chart.z0[k] + (2.0 * u - 1.0) * chart.q1[k] + (2.0 * v - 1.0) * chart.q2[k];
                assert!(
                    z.abs() <= 2.0,
                    "after the {what} edit, z_{k} = {z} leaves the factor's domain"
                );
            }
        }
        let want = f64s::<LatentDefaults>(&[(chart, POINTS.to_vec())]);
        for (i, (g, c)) in got.iter().zip(&want).enumerate() {
            for (k, (a, b)) in mrp(&g.ic).iter().zip(mrp(&c.ic)).enumerate() {
                let e = error(*a, b);
                assert!(
                    e <= factor,
                    "after the {what} edit, point {i} value {k} is {e:.1} eps from the edited chart's f64 decode"
                );
            }
        }
    }
}

#[test]
fn qa_m206_gpu_edits_reach_the_kernel() {
    let h = harness();
    let stage = DecodeStage::<LatentDefaults>::new(h.device(), &wgsl(), FastMath::default())
        .expect("the stage");
    check_edits(
        &mut |c| {
            stage
                .run(h.device(), h.queue(), c, DecodeMode::Full, &POINTS)
                .expect("the dispatch")
        },
        factor(),
    );
}

negative_control!(
    qa_m206_gpu_edits_reach_the_kernel,
    "a stage that keeps the first chart's uniform, ignoring the edits, must fail",
    expected = "from the edited chart's f64 decode",
    {
        let h = harness();
        let stage = DecodeStage::<LatentDefaults>::new(h.device(), &wgsl(), FastMath::default())
            .expect("the stage");
        let mut first: Option<Latent<f32>> = None;
        check_edits(
            &mut |c| {
                let kept = *first.get_or_insert(*c);
                stage
                    .run(h.device(), h.queue(), &kept, DecodeMode::Full, &POINTS)
                    .expect("the dispatch")
            },
            factor(),
        )
    }
);

// ── DECODE_MODE on the GPU: bit 0 of the flags word ─────────────────────────────────────────────────────────────

/// Under each flags word, the kernel writes every sample, bit for bit the decode under 0, where the `DECODE_MODE` bit
/// `mode` (bit 0) is 0, and writes nothing where it is 1 (the linearised path lands in M6), whatever the other 31 bits
/// are.
fn check_flags(mode: u32, words: &[u32]) {
    let h = harness();
    let stage = DecodeStage::<LatentDefaults>::new(h.device(), &wgsl(), FastMath::default())
        .expect("the stage");
    let chart = Latent {
        z0: [0.3, -0.1, 0.2, 0.0, -0.4, 0.1, 0.2, -0.2],
        q1: [0.5; 8],
        q2: [-0.25; 8],
    };
    let full = stage
        .dispatch(h.device(), h.queue(), &chart, 0, &POINTS)
        .expect("the dispatch");
    assert!(full.tags.iter().all(|&t| t != UNWRITTEN));
    let nan = f32::NAN.to_bits();
    for &w in words {
        let got = stage
            .dispatch(h.device(), h.queue(), &chart, w, &POINTS)
            .expect("the dispatch");
        if w & mode == 0 {
            assert_eq!(
                got, full,
                "flags {w:#010x}, bit 0 clear: not the full decode"
            );
        } else {
            assert!(
                got.tags.iter().all(|&t| t == UNWRITTEN)
                    && got.ic.iter().chain(&got.descriptors).all(|&x| x == nan),
                "flags {w:#010x}, bit 0 set: the kernel wrote"
            );
        }
    }
}

#[test]
fn qa_m206_gpu_decode_mode_bit0() {
    check_flags(
        1,
        &[
            0,
            1,
            2,
            3,
            0xFFFF_FFFE,
            0xFFFF_FFFF,
            0x8000_0000,
            0x8000_0001,
            0x0000_0100,
            0xDEAD_BEEE,
            0xDEAD_BEEF,
        ],
    );
}

negative_control!(
    qa_m206_gpu_decode_mode_bit0,
    "a check told that bit 1 is DECODE_MODE must fail on the real kernel",
    expected = "bit 0 set: the kernel wrote",
    check_flags(2, &[0, 2])
);

// ── The link set is baked per variant (REQ-DEC-007, REQ-CHART-001) ─────────────────────────────────────────────

/// The worst of `g`'s `(m, r, p)` values' differences from `c`'s, in f32 eps (the gate's measure).
fn worst_eps(g: &DecodeOut<f32>, c: &DecodeOut<f64>) -> f64 {
    mrp(&g.ic)
        .iter()
        .zip(mrp(&c.ic))
        .map(|(x, y)| error(*x, y))
        .fold(0.0, f64::max)
}

/// The same chart and points through the two entries: they differ; each sample is nearer its own selection's f64
/// decode than the other selection's; and the defaults' is within `factor` of theirs (REQ-DEC-043's domain is the
/// defaults at `|zₖ| ≤ 2`, which `interior` stays in; the corpus gives `stick_breaking` no factor).
fn check_variants(a: &[DecodeOut<f32>], b: &[DecodeOut<f32>], batches: &[Batch], factor: f64) {
    let (fa, fb) = (
        f64s::<LatentDefaults>(batches),
        f64s::<LatentStickBreaking>(batches),
    );
    assert_ne!(
        a.iter().map(|o| o.ic).collect::<Vec<_>>(),
        b.iter().map(|o| o.ic).collect::<Vec<_>>(),
        "the two variants decode alike: the link set is not baked"
    );
    for (name, g, own, other) in [("defaults", a, &fa, &fb), ("stick_breaking", b, &fb, &fa)] {
        for (i, ((g, c), o)) in g.iter().zip(own.iter()).zip(other.iter()).enumerate() {
            let (e, f) = (worst_eps(g, c), worst_eps(g, o));
            assert!(
                e < f,
                "the {name} entry at sample {i} is {e:.1} eps from its selection's f64 decode, {f:.1} from the other's"
            );
        }
    }
    for (i, (g, c)) in a.iter().zip(&fa).enumerate() {
        for (k, (x, y)) in mrp(&g.ic).iter().zip(mrp(&c.ic)).enumerate() {
            let e = error(*x, y);
            assert!(
                e <= factor,
                "the defaults entry at sample {i}, value {k}: {e:.1} eps from its selection's f64 decode"
            );
        }
    }
}

fn variant_batches() -> Vec<Batch> {
    interior(Fuzz {
        seed: 606,
        charts: 4,
        per_chart: 32,
    })
}

#[test]
fn qa_m206_gpu_variants_baked() {
    let h = harness();
    let batches = variant_batches();
    let a = gpu::<LatentDefaults>(&h, &batches);
    let b = gpu::<LatentStickBreaking>(&h, &batches);
    check_variants(&a, &b, &batches, factor());
}

negative_control!(
    qa_m206_gpu_variants_baked,
    "the defaults' output passed off as stick_breaking's must fail",
    expected = "the two variants decode alike",
    {
        let h = harness();
        let batches = variant_batches();
        let a = gpu::<LatentDefaults>(&h, &batches);
        check_variants(&a, &a, &batches, factor())
    }
);

// ── The gate (REQ-DEC-043) ─────────────────────────────────────────────────────────────────────────────────────

/// The checked-in input `name`, without its `"expected"`, which must be `want`: the runner's own outcome for it.
fn fixture(name: &str, want: &str) -> Input {
    let text = std::fs::read_to_string(root().join("fixtures/gates/decode-f32-eps").join(name))
        .expect(name);
    let mut v: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let expected = v
        .as_object_mut()
        .and_then(|o| o.remove("expected"))
        .expect("an expected outcome");
    assert_eq!(expected, want, "{name} does not expect {want}");
    serde_json::from_value(v).expect("a decode-f32-eps input")
}

/// The checked-in perturbed input fails the gate; the interior passes it.
fn check_fixtures(perturbed: &Input, interior: &Input) {
    let (c, w) = (config(), wgsl());
    let bad = DecodeF32EpsGate::judge(&c, "perturbed.json", perturbed, &w).expect("judged");
    assert_eq!(
        bad.verdict,
        Verdict::Fail,
        "the perturbed input passed the gate"
    );
    let good = DecodeF32EpsGate::judge(&c, "interior.json", interior, &w).expect("judged");
    assert_eq!(good.verdict, Verdict::Pass, "the interior failed the gate");
}

#[test]
fn qa_m206_gate_perturbed_fails() {
    check_fixtures(
        &fixture("perturbed.json", "fail"),
        &fixture("interior.json", "pass"),
    );
}

negative_control!(
    qa_m206_gate_perturbed_fails,
    "the perturbed input without its perturbation must fail the check",
    expected = "the perturbed input passed the gate",
    {
        let mut p = fixture("perturbed.json", "fail");
        p.perturb = None;
        check_fixtures(&p, &fixture("interior.json", "pass"))
    }
);

/// A smaller interior.
fn small() -> Input {
    Input {
        description: "qa".to_owned(),
        points: Points::Interior(Fuzz {
            seed: 11,
            charts: 8,
            per_chart: 64,
        }),
        perturb: None,
        recorded: None,
    }
}

/// The worst eps the gate reports for `input` (its first scatter line after the adapter's).
fn worst(input: &Input, threshold: f64) -> (Verdict, f64) {
    let mut c = config();
    c.threshold.value = threshold;
    let r = DecodeF32EpsGate::judge(&c, "qa", input, &wgsl()).expect("judged");
    let line = r
        .scatter
        .iter()
        .find(|l| l.starts_with("worst "))
        .expect("the worst line")
        .clone();
    let eps: f64 = line["worst ".len()..]
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .expect("a number");
    (r.verdict, eps)
}

/// The verdict turns at the threshold: at the measured worst, scaled by `at`, it passes; just below it, fails; and a
/// perturbation of any one latent component by 10⁻² fails at the proposed factor.
fn check_turn(at: f64) {
    let (_, w) = worst(&small(), factor());
    assert!(w > 0.0, "the measurement is 0");
    // The report prints two decimals: allow for the rounding.
    let (pass, _) = worst(&small(), (w + 0.005) * at);
    assert_eq!(
        pass,
        Verdict::Pass,
        "the gate fails at its measured worst {w}"
    );
    let (fail, _) = worst(&small(), (w - 0.005) * 0.99);
    assert_eq!(
        fail,
        Verdict::Fail,
        "the gate passes below its measured worst {w}"
    );
    for component in 0..8 {
        let mut p = small();
        p.perturb = Some(Perturb {
            component,
            by: 1e-2,
        });
        let (v, e) = worst(&p, factor());
        assert_eq!(
            v,
            Verdict::Fail,
            "z₀[{component}] moved by 10⁻² passes the gate ({e} eps)"
        );
    }
}

#[test]
fn qa_m206_gate_turns_at_threshold() {
    check_turn(1.0);
}

negative_control!(
    qa_m206_gate_turns_at_threshold,
    "a threshold 2 % under the measured worst must fail the pass side",
    expected = "the gate fails at its measured worst",
    check_turn(0.98)
);
