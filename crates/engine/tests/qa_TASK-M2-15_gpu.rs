//! QA tests for TASK-M2-15 on the GPU, written from the requirements: encode's selects and notices at the poles and on
//! the `−x` half-axis, GPU-f32 against the same source on CPU-f32 and CPU-f64, on identical inputs. The CPU side is
//! `kernel`'s `qa_TASK-M2-15`.
//! - REQ-DEC-014 (`encode_qa_m215_gpu_poles`): `‖λ̃‖ = 0` and `‖ρ̃‖ = 0` exactly, every sign of the zero components: no
//!   NaN, two dispatches bit-identical, the notices identical to CPU-f32's and CPU-f64's, `lookup_clamped` set, `β`
//!   R-437's `+0` at `λ̃ = 0`, and `α`, `β` within f32 rounding of CPU-f64's.
//! - REQ-ENC-007 (`encode_qa_m215_gpu_half_axis`): `β` on given operands (R-437, R-438): `(x < 0, ±0)` gives π̂ (f32's
//!   image of π, rounded toward zero, R-417) with CPU-f32's bits, `(±0, ±0)` gives `+0`, `β(x, −y)` has `β(x, y)`'s
//!   bits; and collinear states with body 2 on the `−x` side, `λ̃_y = ±0`, encode on the GPU with `β = π̂`.
//!
//! The test names carry `encode_` so CI's Metal and lavapipe steps select them. The binary reads
//! `target/spirv/kernel.wgsl`, which `cargo xtask build-kernel` writes. Operands are normal or zero: a subnormal may be
//! flushed on the GPU, which the requirements do not cover.
//!
//! **Tolerances** (applied per R-369, test decisions): an angle `atan2` computes compares within WGSL's stated accuracy
//! for `atan2`, 4096 ULP of an f32 result ([`ATAN2_TOL`]), so a backend that meets the WGSL spec passes, Metal and
//! lavapipe alike; the selects (`+0`, `π̂`) and the notices compare exactly.
//!
//! **Domain** (pitfalls §10): encode alone, on states held fixed; no trajectory. **PIT-10:** each state is one
//! `Ic<f32>` handed to every backend.
//!
//! Each test has a registered negative control (R-176).

use std::f64::consts::{FRAC_PI_2, PI};
use std::path::Path;

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{beta_gpu, encode_gpu};
use kernel::decode::Ic;
use kernel::encode::{blocks, encode};
use kernel::generated::links::Defaults;
use kernel::stage::encode::EncodeOut;
use validation::gpu::GpuHarness;
use validation::negative_control;

fn wgsl() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/spirv/kernel.wgsl"),
    )
    .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

fn gpu_encode(h: &GpuHarness, ics: &[Ic<f32>]) -> Vec<EncodeOut> {
    encode_gpu(h.device(), h.queue(), &wgsl(), ics, FastMath::default())
        .expect("the encode dispatch")
}

fn gpu_beta(h: &GpuHarness, pairs: &[[f32; 2]]) -> Vec<f32> {
    beta_gpu(h.device(), h.queue(), &wgsl(), pairs, FastMath::default()).expect("the β dispatch")
}

/// f32's π̂: the image of π rounded toward zero (R-417).
fn pi_hat() -> f32 {
    let nearest = PI as f32;
    if f64::from(nearest) > PI {
        f32::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

const ZEROS: [f32; 2] = [0.0, -0.0];

/// WGSL's accuracy for `atan2` at f32, 4096 ULP, taken at the angles' scale, `π/2` (WGSL § "Floating Point
/// Accuracy"): the bound any conforming backend's `atan2` meets.
const ATAN2_TOL: f32 = 4096.0 * f32::EPSILON * std::f32::consts::FRAC_PI_2;
const P: [[f32; 2]; 3] = [[0.1, 0.2], [-0.3, 0.05], [0.2, -0.25]];

/// `‖λ̃‖ = 0` exactly (`m₀ = m₁`, body 2 at the inner pair's centre) and `‖ρ̃‖ = 0` exactly (bodies 0 and 1 together),
/// every sign of the zero components; dyadic masses and symmetric positions keep every product exact.
fn pole_states() -> (Vec<Ic<f32>>, Vec<Ic<f32>>) {
    let (mut lam, mut rho) = (Vec::new(), Vec::new());
    for a in ZEROS {
        for b in ZEROS {
            for c in ZEROS {
                lam.push(Ic {
                    m: [0.25, 0.25, 0.5],
                    r: [[-1.0, a], [1.0, b], [c, a]],
                    p: P,
                });
                rho.push(Ic {
                    m: [0.25, 0.25, 0.5],
                    r: [[-1.0, a], [-1.0, b], [1.0, c]],
                    p: P,
                });
            }
        }
    }
    (lam, rho)
}

fn notices(o: &EncodeOut) -> (bool, bool, u32, bool) {
    (
        o.report.lookup_clamped(),
        o.report.mirrored,
        o.report.clamped_controls,
        o.report.coincident,
    )
}

fn words(o: &EncodeOut) -> Vec<u32> {
    let mut w: Vec<u32> = o.z.iter().map(|v| v.to_bits()).collect();
    w.extend([
        o.alpha.to_bits(),
        o.beta.to_bits(),
        o.report.lambda.to_bits(),
    ]);
    w.push(o.report.clamped_controls);
    w.push(u32::from(o.report.mirrored) | u32::from(o.report.coincident) << 1);
    w
}

/// Checks one pole family's GPU outputs `g` (and `again`, a second dispatch) against the CPU on the same states.
fn check_poles(
    name: &str,
    states: &[Ic<f32>],
    g: &[EncodeOut],
    again: &[EncodeOut],
    alpha: f64,
    lam_pole: bool,
) {
    let tol = f64::from(ATAN2_TOL);
    for (i, x) in states.iter().enumerate() {
        let what = format!("{name} state {i} ({:?})", x.r);
        let o = &g[i];
        assert!(
            o.z.iter().chain([&o.alpha, &o.beta]).all(|v| v.is_finite()),
            "{what}: a NaN or infinity on the GPU: {o:?}"
        );
        assert_eq!(
            words(o),
            words(&again[i]),
            "{what}: the GPU gave two results"
        );
        let c32 = EncodeOut::of(&encode::<Defaults, f32>(x));
        let x64 = Ic {
            m: x.m.map(f64::from),
            r: x.r.map(|v| v.map(f64::from)),
            p: x.p.map(|v| v.map(f64::from)),
        };
        let c64 = encode::<Defaults, f64>(&x64);
        let n64 = (
            c64.report.lookup_clamped(),
            c64.report.mirrored,
            c64.report.clamped_controls,
            c64.report.coincident,
        );
        assert_eq!(
            notices(o),
            notices(&c32),
            "{what}: the notices differ from CPU-f32's"
        );
        assert_eq!(notices(o), n64, "{what}: the notices differ from CPU-f64's");
        assert!(
            o.report.lookup_clamped(),
            "{what}: lookup_clamped not set on the GPU"
        );
        assert!(
            (f64::from(o.alpha) - alpha).abs() <= tol
                && (f64::from(o.alpha) - c64.blocks.alpha).abs() <= tol,
            "{what}: GPU α = {}, CPU-f64 α = {}, the pole's {alpha}",
            o.alpha,
            c64.blocks.alpha
        );
        assert!(
            (f64::from(o.beta) - c64.blocks.beta).abs() <= tol,
            "{what}: GPU β = {}, CPU-f64 β = {}",
            o.beta,
            c64.blocks.beta
        );
        if lam_pole {
            assert!(
                o.beta.to_bits() == 0.0f32.to_bits(),
                "{what}: GPU β = {:?}, not R-437's 0",
                o.beta
            );
        }
    }
}

#[test]
fn encode_qa_m215_gpu_poles() {
    let h = GpuHarness::new().expect("a GPU device");
    let (lam, rho) = pole_states();
    check_poles(
        "λ̃ = 0",
        &lam,
        &gpu_encode(&h, &lam),
        &gpu_encode(&h, &lam),
        0.0,
        true,
    );
    check_poles(
        "ρ̃ = 0",
        &rho,
        &gpu_encode(&h, &rho),
        &gpu_encode(&h, &rho),
        FRAC_PI_2,
        false,
    );
}

negative_control!(
    encode_qa_m215_gpu_poles,
    "a GPU β of −π at the λ̃ pole (atan2(−0, −0)) must fail",
    expected = "not R-437's 0",
    {
        let h = GpuHarness::new().expect("a GPU device");
        let (lam, _) = pole_states();
        let mut g = gpu_encode(&h, &lam);
        // −0: the zero's sign leaking through, which the select pins to +0.
        g[3].beta = -0.0;
        let again = g.clone();
        check_poles("λ̃ = 0", &lam, &g, &again, 0.0, true);
    }
);

/// `β` operands: the `−x` half-axis with `±0`, the pole, and random pairs with their mirror images.
fn pairs() -> Vec<[f32; 2]> {
    let mut v = Vec::new();
    for e in -30..=30 {
        let x = -(2f32).powi(e) * 1.37;
        v.push([x, 0.0]);
        v.push([x, -0.0]);
    }
    for x in ZEROS {
        for y in ZEROS {
            v.push([x, y]);
        }
    }
    let mut s = 0x2150_0005u64;
    for _ in 0..512 {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = ((s >> 40) as f32 / (1u64 << 24) as f32) * 4.0 - 2.0;
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let y = ((s >> 40) as f32 / (1u64 << 24) as f32) * 4.0 - 2.0;
        v.push([x, y]);
        v.push([x, -y]);
    }
    v
}

fn check_half_axis(p: &[[f32; 2]], g: &[f32]) {
    for (k, &[x, y]) in p.iter().enumerate() {
        let b = g[k];
        let cpu = blocks::beta([x, y]);
        assert!(
            (0.0..=pi_hat()).contains(&b),
            "GPU β({x:e}, {y:e}) = {b} ∉ [0, π̂]"
        );
        if y == 0.0 && x < 0.0 {
            assert!(
                b.to_bits() == pi_hat().to_bits() && cpu.to_bits() == b.to_bits(),
                "GPU β({x:e}, {y:?}) = {b:?}, CPU-f32 {cpu:?}: not π̂ by the select on the −x half-axis"
            );
            assert!(
                (f64::from(b) - blocks::beta([f64::from(x), f64::from(y)])).abs()
                    <= f64::from(f32::EPSILON) * PI,
                "GPU β({x:e}, {y:?}) = {b} is not CPU-f64's π to f32 rounding"
            );
        }
        if x == 0.0 && y == 0.0 {
            assert!(
                b.to_bits() == 0.0f32.to_bits(),
                "GPU β({x:?}, {y:?}) = {b:?}, not R-437's 0"
            );
        }
        assert!(
            (b - cpu).abs() <= ATAN2_TOL,
            "GPU β({x:e}, {y:e}) = {b}, CPU-f32 {cpu}"
        );
    }
    for k in (0..p.len()).filter(|&k| p[k][1] != 0.0) {
        if let Some(m) = (0..p.len()).find(|&m| p[m] == [p[k][0], -p[k][1]]) {
            assert!(
                g[k].to_bits() == g[m].to_bits(),
                "GPU β({:e}, {:e}) = {} but {} mirrored: not the mirror's quotient",
                p[k][0],
                p[k][1],
                g[k],
                g[m]
            );
        }
    }
}

/// Collinear states with body 2 on the `−x` side, `λ̃_y = ±0`, through the whole encode on the GPU.
fn collinear() -> Vec<Ic<f32>> {
    let mut v = Vec::new();
    for y in ZEROS {
        // Body 2 left of the pair: λ̃_x < 0 once ρ̃ is on +x.
        v.push(Ic {
            m: [0.25, 0.25, 0.5],
            r: [[0.5, y], [1.5, y], [-1.0, y]],
            p: P,
        });
        v.push(Ic {
            m: [0.25, 0.5, 0.25],
            r: [[-0.5, y], [0.75, -y], [-1.5, y]],
            p: P,
        });
    }
    v
}

#[test]
fn encode_qa_m215_gpu_half_axis() {
    let h = GpuHarness::new().expect("a GPU device");
    let p = pairs();
    check_half_axis(&p, &gpu_beta(&h, &p));
    let states = collinear();
    for (i, o) in gpu_encode(&h, &states).iter().enumerate() {
        let cpu = EncodeOut::of(&encode::<Defaults, f32>(&states[i]));
        assert!(
            o.beta.to_bits() == pi_hat().to_bits() && cpu.beta.to_bits() == o.beta.to_bits(),
            "collinear state {i}: GPU β = {:?}, CPU-f32 β = {:?}, not π̂ (body 2 on the −x side)",
            o.beta,
            cpu.beta
        );
        assert!(
            o.report.lookup_clamped() && notices(o) == notices(&cpu),
            "collinear state {i}: GPU notices {:?}, CPU-f32 {:?}",
            notices(o),
            notices(&cpu)
        );
    }
}

negative_control!(
    encode_qa_m215_gpu_half_axis,
    "a GPU β of atan2's −π on the −x half-axis must fail",
    expected = "∉ [0, π̂]",
    {
        let h = GpuHarness::new().expect("a GPU device");
        let p = pairs();
        let mut g = gpu_beta(&h, &p);
        g[1] = (-0.0f32).atan2(p[1][0]);
        check_half_axis(&p, &g);
    }
);
