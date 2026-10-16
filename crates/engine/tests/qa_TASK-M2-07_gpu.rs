//! QA tests for TASK-M2-07 on the GPU, written from the requirements: `C`'s branch decisions on CPU-f32 and GPU-f32
//! on identical inputs. The CPU side is `kernel`'s `qa_TASK-M2-07`.
//! - REQ-DEC-017, REQ-ENC-015, REQ-ENC-016; R-34, R-84; dd_decoder §5 test 12 ("the degenerate/feasibility branch tags
//!   are bit-identical across all backends"): for every state fed to both, the mirror (`lookup_mirrored`) and the
//!   `ρ = 0` guard (`lookup_clamped`) are the same on CPU-f32 and GPU-f32, with no condition on the intermediate
//!   values agreeing first (pitfalls §9: a check conditioned on the bits agreeing masks the fork). The states:
//!   collinear configurations at random angles, where `λ̃_y` is rounding alone; deadband-straddling turned states,
//!   `λ̃_y = k·δ_λ`; coincident inner pairs, `r₀ = r₁`, at random places, and `ρ = (±0, ±0)`; and generic states
//!   (`canonicaliser_qa_m207_branches_agree`). The GPU gives the same words twice (`canonicaliser_qa_m207_twice`).
//!
//! The test names carry `canonicaliser_` so CI's Metal step selects them. The binary reads
//! `target/spirv/kernel.wgsl`, which `cargo xtask build-kernel` writes.
//!
//! **Domain** (pitfalls §10): `C` alone, on states held fixed; no trajectory. **PIT-10:** the claim is for identical
//! inputs; every state here is one `Ic<f32>` handed to both backends.
//!
//! Each test has a registered negative control (R-176).

use std::path::Path;

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::canonicalise_gpu;
use kernel::canon::{canonicalise, Canonical};
use kernel::decode::Ic;
use validation::decode_f32_eps::SplitMix;
use validation::gpu::GpuHarness;
use validation::negative_control;

const DELTA: f64 = 1e-12;

fn wgsl() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/spirv/kernel.wgsl"),
    )
    .expect("target/spirv/kernel.wgsl: run `cargo xtask build-kernel` first")
}

fn gpu(h: &GpuHarness, ics: &[Ic<f32>]) -> Vec<Canonical<f32>> {
    canonicalise_gpu(h.device(), h.queue(), &wgsl(), ics, FastMath::default())
        .expect("the canonicalise dispatch")
}

fn narrow(x: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: x.m.map(|v| v as f32),
        r: x.r.map(|v| v.map(|c| c as f32)),
        p: x.p.map(|v| v.map(|c| c as f32)),
    }
}

/// Masses each at least 0.2 before normalising, summing to 1.
fn masses(rng: &mut SplitMix) -> [f64; 3] {
    let w = [0.0; 3].map(|_| f64::from(rng.uniform(0.2, 1.2)));
    let t: f64 = w.iter().sum();
    w.map(|v| v / t)
}

fn momenta(rng: &mut SplitMix) -> [[f64; 2]; 3] {
    [0; 3].map(|_| {
        [
            f64::from(rng.uniform(-0.5, 0.5)),
            f64::from(rng.uniform(-0.5, 0.5)),
        ]
    })
}

fn turn(v: [f64; 2], th: f64) -> [f64; 2] {
    let (s, c) = th.sin_cos();
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

/// Collinear states: the three bodies on one line through the origin at a random angle, so `λ̃_y` after step 2 is
/// rounding alone and sits in or near the deadband.
fn collinear(n: usize, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    (0..n)
        .map(|_| {
            let m = masses(&mut rng);
            let th = f64::from(rng.uniform(0.0, std::f32::consts::TAU));
            let r = [0; 3].map(|_| turn([f64::from(rng.uniform(-1.0, 1.0)), 0.0], th));
            narrow(&Ic {
                m,
                r,
                p: momenta(&mut rng),
            })
        })
        .collect()
}

/// States with `λ̃_y = k·δ_λ` in the canonical frame (dd_decoder §3.2's reconstruction), `k` straddling `±1`, then
/// turned, translated and boosted.
fn deadband(seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    let mut out = Vec::new();
    for k in [-3.0, -1.1, -1.0, -0.9, 0.0, 0.9, 1.0, 1.1, 3.0] {
        for _ in 0..64 {
            let m = masses(&mut rng);
            let alpha = f64::from(rng.uniform(0.2, 1.3));
            let beta = (k * DELTA / alpha.sin()).asin();
            let m01 = m[0] + m[1];
            let rho = [alpha.cos() / (m[0] * m[1] / m01).sqrt(), 0.0];
            let lam = [
                alpha.sin() * beta.cos() / (m[2] * m01).sqrt(),
                alpha.sin() * beta.sin() / (m[2] * m01).sqrt(),
            ];
            let r01 = [-m[2] * lam[0], -m[2] * lam[1]];
            let r = [
                [r01[0] - m[1] / m01 * rho[0], r01[1]],
                [r01[0] + m[0] / m01 * rho[0], r01[1]],
                [m01 * lam[0], m01 * lam[1]],
            ];
            let th = f64::from(rng.uniform(-3.1, 3.1));
            let a = [
                f64::from(rng.uniform(-2.0, 2.0)),
                f64::from(rng.uniform(-2.0, 2.0)),
            ];
            let r = r.map(|v| {
                let w = turn(v, th);
                [w[0] + a[0], w[1] + a[1]]
            });
            out.push(narrow(&Ic {
                m,
                r,
                p: momenta(&mut rng),
            }));
        }
    }
    out
}

/// Coincident inner pairs, `r₀ = r₁` bit for bit at a random place, the third body elsewhere; and `ρ = (±0, ±0)`
/// with the third body on an axis, so the signed zeros survive steps 1a and 0.
fn coincident(n: usize, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    let mut out: Vec<Ic<f32>> = (0..n)
        .map(|_| {
            let m = masses(&mut rng);
            let pair = [
                f64::from(rng.uniform(-3.0, 3.0)),
                f64::from(rng.uniform(-3.0, 3.0)),
            ];
            let third = [
                f64::from(rng.uniform(-3.0, 3.0)),
                f64::from(rng.uniform(-3.0, 3.0)),
            ];
            narrow(&Ic {
                m,
                r: [pair, pair, third],
                p: momenta(&mut rng),
            })
        })
        .collect();
    let m = [0.3f32, 0.25, 0.45];
    let p = [[0.1f32, -0.3], [0.2, 0.05], [-0.25, 0.4]];
    out.push(Ic {
        m,
        r: [[0.0, 0.0], [-0.0, 0.0], [0.0, 1.5]],
        p,
    });
    out.push(Ic {
        m,
        r: [[0.0, 0.0], [0.0, -0.0], [1.5, 0.0]],
        p,
    });
    out.push(Ic {
        m,
        r: [[0.0, 0.0], [-0.0, -0.0], [0.0, 0.0]],
        p,
    });
    out
}

/// Generic states: positions and momenta uniform in `[−4, 4]`.
fn generic(n: usize, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    (0..n)
        .map(|_| {
            let m = masses(&mut rng);
            let r = [0; 3].map(|_| {
                [
                    f64::from(rng.uniform(-4.0, 4.0)),
                    f64::from(rng.uniform(-4.0, 4.0)),
                ]
            });
            narrow(&Ic {
                m,
                r,
                p: momenta(&mut rng),
            })
        })
        .collect()
}

/// Every family, named.
fn families() -> Vec<(&'static str, Vec<Ic<f32>>)> {
    vec![
        ("collinear", collinear(4096, 71)),
        ("deadband", deadband(73)),
        ("coincident", coincident(256, 79)),
        ("generic", generic(2048, 83)),
    ]
}

/// The branches of `cpu` and `gpu`, state by state, on the same inputs: every state's mirror and guard agree.
/// `Err` counts the disagreements and names the first.
fn branches(
    name: &str,
    states: &[Ic<f32>],
    cpu: &[Canonical<f32>],
    gpu: &[Canonical<f32>],
) -> Result<(), String> {
    let mut first = None;
    let (mut mirror, mut clamp) = (0, 0);
    for (k, ((x, c), g)) in states.iter().zip(cpu).zip(gpu).enumerate() {
        mirror += usize::from(c.mirrored != g.mirrored);
        clamp += usize::from(c.clamped != g.clamped);
        if first.is_none() && (c.mirrored != g.mirrored || c.clamped != g.clamped) {
            first = Some(format!(
                "state {k} {x:?}: CPU-f32 mirrored {} clamped {} (λ̃_y {:e}), GPU-f32 mirrored {} clamped {} (λ̃_y {:e})",
                c.mirrored, c.clamped, c.lambda_t_y, g.mirrored, g.clamped, g.lambda_t_y
            ));
        }
    }
    match first {
        None => Ok(()),
        Some(f) => Err(format!(
            "{name}: the branches differ on identical inputs, mirror at {mirror} and guard at {clamp} of {} states; first: {f}",
            states.len()
        )),
    }
}

#[test]
fn canonicaliser_qa_m207_branches_agree() {
    let h = GpuHarness::new().expect("a GPU device");
    let mut failures = Vec::new();
    for (name, states) in families() {
        let cpu: Vec<Canonical<f32>> = states.iter().map(canonicalise).collect();
        let g = gpu(&h, &states);
        assert_eq!(g.len(), states.len(), "{name}: the GPU's count");
        let mirrored = cpu.iter().filter(|c| c.mirrored).count();
        let clamped = cpu.iter().filter(|c| c.clamped).count();
        eprintln!("canonicaliser_qa_m207_branches_agree {name}: {} states, CPU-f32 mirrored {mirrored}, clamped {clamped}", states.len());
        if name == "coincident" {
            assert_eq!(
                clamped,
                states.len(),
                "every coincident state reaches the guard on CPU-f32"
            );
        }
        if name == "deadband" || name == "generic" {
            assert!(
                mirrored > 0 && mirrored < states.len(),
                "{name}: both sides of the mirror reached"
            );
        }
        if let Err(f) = branches(name, &states, &cpu, &g) {
            failures.push(f);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

negative_control!(
    canonicaliser_qa_m207_branches_agree,
    "a GPU output with one state's mirror flipped must fail, whatever its λ̃_y",
    expected = "the branches differ on identical inputs",
    {
        let states = generic(64, 83);
        let cpu: Vec<Canonical<f32>> = states.iter().map(canonicalise).collect();
        let mut g = cpu.clone();
        g[17].mirrored = !g[17].mirrored;
        branches("flipped", &states, &cpu, &g).unwrap();
    }
);

/// Two dispatches over `states` give the same words, notices included.
fn twice(a: &[Canonical<f32>], b: &[Canonical<f32>]) -> Result<(), String> {
    let bits = |c: &Canonical<f32>| {
        let mut w: Vec<u32> =
            c.ic.m
                .iter()
                .chain(c.ic.r.as_flattened())
                .chain(c.ic.p.as_flattened())
                .map(|v| v.to_bits())
                .collect();
        w.extend([c.i_in, c.lambda, c.phi, c.lambda_t_y].map(f32::to_bits));
        w.extend([u32::from(c.clamped), u32::from(c.mirrored)]);
        w
    };
    match a.iter().zip(b).position(|(x, y)| bits(x) != bits(y)) {
        None => Ok(()),
        Some(k) => Err(format!("the GPU gave two results at state {k}")),
    }
}

#[test]
fn canonicaliser_qa_m207_twice() {
    let h = GpuHarness::new().expect("a GPU device");
    for (name, states) in families() {
        let (a, b) = (gpu(&h, &states), gpu(&h, &states));
        twice(&a, &b).unwrap_or_else(|f| panic!("{name}: {f}"));
    }
}

negative_control!(
    canonicaliser_qa_m207_twice,
    "a second result differing in one notice must fail",
    expected = "two results",
    {
        let h = GpuHarness::new().expect("a GPU device");
        let states = generic(64, 83);
        let a = gpu(&h, &states);
        let mut b = a.clone();
        b[5].clamped = !b[5].clamped;
        twice(&a, &b).unwrap();
    }
);
