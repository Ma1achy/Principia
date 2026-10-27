//! QA tests for TASK-M2-47 on the GPU, written from the requirements and R-547: step 0's skip, its coincidence select,
//! step 2's guard and step 3's mirror on CPU-f32 and GPU-f32, fed the same `Ic<f32>`. The CPU side's own checks are
//! `kernel`'s `qa_TASK-M2-47`, whose constructions these states repeat.
//! - REQ-DEC-002, REQ-ENC-006 (`canonicaliser_qa_m247_branches_agree`): `lookup_rescaled` (the skip), `lookup_clamped`
//!   (the guard and the coincidence) and `lookup_mirrored` are the same on both, with no condition on any value
//!   agreeing first (pitfalls §9), over: states whose `I_in` is exactly `1 ± τ_I`, inside it and just outside it; f32
//!   states at coordinates near `1e-36` whose inner pair is a subnormal apart, and the same with the pair coincident;
//!   three coincident bodies at the origin and off it; generic states scaled from `1e-37` to `1e37`; and canonical
//!   decodes. Where CPU-f32 skips step 0, the GPU's `λ` is 1 by its bits.
//! - REQ-ENC-006 (`canonicaliser_qa_m247_range`): on the GPU, the scaled states leave `C` with every output finite and
//!   `I = 1` to rounding, and the coincident bodies with `p = 0`, `λ = 1` and `φ = +0`.
//!
//! The names carry `canonicaliser_` so CI's Metal and lavapipe steps select them. The binary reads
//! `target/spirv/kernel.wgsl`, which `cargo xtask build-kernel` writes.
//!
//! **Tolerances** (applied per R-369: a test's bound). `I = 1` on the GPU: 16 f32 eps (WGSL's `sqrt` and division
//! carry ulp bounds, not correct rounding).
//!
//! **Domain** (pitfalls §10): `C` alone on states held fixed; no trajectory. **PIT-10:** the claim is for identical
//! inputs.
//!
//! Each test has a registered negative control (R-176).

use std::path::Path;

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::canonicalise_gpu;
use kernel::canon::{canonicalise, Canonical};
use kernel::decode::{decode, Ic, Latent};
use kernel::generated::links::Defaults;
use kernel::real::IeeeBits;
use validation::decode_f32_eps::SplitMix;
use validation::gpu::GpuHarness;
use validation::negative_control;

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

fn widen(x: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: x.m.map(f64::from),
        r: x.r.map(|v| v.map(f64::from)),
        p: x.p.map(|v| v.map(f64::from)),
    }
}

/// `log₂ v` for `v` a power of two.
fn log2_exact(v: f64) -> i32 {
    let e = v.log2().round() as i32;
    assert!(2f64.powi(e) == v, "{v} is not a power of two");
    e
}

/// Masses `(¼, ¼, ½)`, `r₀ = (−1−f, −g)`, `r₁ = (−1+f, −g)`, `r₂ = (1, g)`: `I_in = 1 + g² + f²/2` exactly, an even
/// exponent `e` taken as `g = 2^{e/2}` and an odd one as `f`, with `f²/2 = 2^e`.
fn above(exps: &[i32]) -> Ic<f32> {
    let (mut f, mut g) = (0.0f64, 0.0f64);
    for &e in exps {
        if e % 2 == 0 {
            g = 2f64.powi(e / 2);
        } else {
            f = 2f64.powi((e + 1) / 2);
        }
    }
    narrow(&Ic {
        m: [0.25, 0.25, 0.5],
        r: [[-1.0 - f, -g], [-1.0 + f, -g], [1.0, g]],
        p: [[0.3, -0.2], [-0.1, 0.4], [-0.1, -0.1]],
    })
}

/// Masses `((1−μ)/2, (1−μ)/2, μ)`, `r₀ = (1, 0)`, `r₁ = (−1, 0)`, `r₂ = 0`: `I_in = 1 − μ` exactly.
fn below(mu: f64) -> Ic<f32> {
    narrow(&Ic {
        m: [(1.0 - mu) / 2.0, (1.0 - mu) / 2.0, mu],
        r: [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]],
        p: [[0.3, -0.2], [-0.1, 0.4], [-0.2, -0.2]],
    })
}

/// The exact-`I` states at `1 ± τ_I`, inside and just outside, each with whether step 0 runs (R-547).
fn edge_states() -> Vec<(Ic<f32>, bool)> {
    let tau = f64::from(f32::TAU_I);
    let e = f64::from(f32::EPSILON);
    let (t, k) = (log2_exact(tau), log2_exact(e));
    let extra = if (t - k) % 2 == 0 { k + 1 } else { k };
    vec![
        (above(&[]), false),
        (above(&[t - 1]), false),
        (above(&[t]), false),
        (above(&[t, extra]), true),
        (above(&[t + 2]), true),
        (below(tau / 2.0), false),
        (below(tau), false),
        (below(tau + e / 2.0), true),
        (below(2.0 * tau), true),
    ]
}

/// f32 states at coordinates near `1e-36`, the inner pair `1..=7` ulps apart per axis (a subnormal difference), the
/// third body about `1e-36` off.
fn subnormal_states(n: usize, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    (0..n)
        .map(|_| {
            let base = [0; 2].map(|_| rng.uniform(0.8, 1.6) * 1e-36);
            let mut r1 = base;
            for v in r1.iter_mut() {
                let k = 1 + (rng.next_u64() % 7);
                let up = rng.next_u64().is_multiple_of(2);
                for _ in 0..k {
                    *v = if up { v.next_up() } else { v.next_down() };
                }
            }
            let r2 = [
                base[0] + rng.uniform(-1.0, 1.0) * 1e-36,
                base[1] + rng.uniform(-1.0, 1.0) * 1e-36,
            ];
            let w = [0; 3].map(|_| rng.uniform(0.2, 1.2));
            let t: f32 = w.iter().sum();
            Ic {
                m: w.map(|v| v / t),
                r: [base, r1, r2],
                p: [0; 3].map(|_| [rng.uniform(-0.5, 0.5), rng.uniform(-0.5, 0.5)]),
            }
        })
        .collect()
}

/// Three coincident bodies at each point, with nonzero momenta.
fn coincident_states() -> Vec<Ic<f32>> {
    [
        [0.0, 0.0],
        [0.3, -0.7],
        [-1.0e-20, 3.0e-21],
        [1.0e18, -2.5e17],
        [-7.0, 1.0e-3],
    ]
    .iter()
    .map(|&q| {
        narrow(&Ic {
            m: [0.25, 0.375, 0.375],
            r: [q, q, q],
            p: [[0.5, -0.25], [-0.25, 0.75], [-0.25, -0.5]],
        })
    })
    .collect()
}

/// Generic states (masses ≥ 0.2 before normalising, bodies ≥ 0.3 apart, off the origin, moving) scaled by `s`
/// (`r ← s·r`, `p ← p/√s`).
fn scaled_states(n: usize, seed: u64, scales: &[f64]) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    let mut out = Vec::new();
    while out.len() < n * scales.len() {
        let w = [0; 3].map(|_| f64::from(rng.uniform(0.2, 1.2)));
        let t: f64 = w.iter().sum();
        let a = [0; 2].map(|_| f64::from(rng.uniform(-1.0, 1.0)));
        let r = [0; 3].map(|_| {
            [
                f64::from(rng.uniform(-1.0, 1.0)) + a[0],
                f64::from(rng.uniform(-1.0, 1.0)) + a[1],
            ]
        });
        let p = [0; 3].map(|_| {
            [
                f64::from(rng.uniform(-0.5, 0.5)),
                f64::from(rng.uniform(-0.5, 0.5)),
            ]
        });
        let apart = [(0, 1), (0, 2), (1, 2)].iter().all(|&(i, j)| {
            ((r[i][0] - r[j][0]).powi(2) + (r[i][1] - r[j][1]).powi(2)).sqrt() >= 0.3
        });
        if !apart {
            continue;
        }
        for &s in scales {
            out.push(narrow(&Ic {
                m: w.map(|v| v / t),
                r: r.map(|v| v.map(|c| c * s)),
                p: p.map(|v| v.map(|c| c / s.sqrt())),
            }));
        }
    }
    out
}

const SCALES: [f64; 8] = [1e-37, 1e-30, 1e-25, 3e-23, 1e-19, 3e19, 1e30, 1e37];

fn canonical_decodes(n: usize, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    (0..n)
        .map(|_| {
            let z = [0; 8].map(|_| rng.uniform(-4.0, 4.0));
            decode::<Defaults, f32>(&Latent::new(z)).ic
        })
        .collect()
}

fn families() -> Vec<(&'static str, Vec<Ic<f32>>)> {
    let sub = subnormal_states(1000, 0x5ab);
    let pairs = sub
        .iter()
        .take(200)
        .map(|x| Ic {
            r: [x.r[0], x.r[0], x.r[2]],
            ..*x
        })
        .collect();
    vec![
        (
            "the τ_I edge",
            edge_states().into_iter().map(|s| s.0).collect(),
        ),
        ("subnormal differences", sub),
        ("coincident inner pairs", pairs),
        ("coincident bodies", coincident_states()),
        ("scaled", scaled_states(64, 47, &SCALES)),
        ("canonical decodes", canonical_decodes(1000, 0x4747)),
    ]
}

/// The branches of each backend's `C` on `ics`: `(lookup_rescaled, lookup_clamped, lookup_mirrored)`, and where the
/// CPU skipped step 0, the GPU's `λ` 1 by its bits. `Err` counts the disagreements and names the first.
fn branches(
    name: &str,
    ics: &[Ic<f32>],
    cpu: &[Canonical<f32>],
    g: &[Canonical<f32>],
) -> Result<(), String> {
    let flags = |c: &Canonical<f32>| (c.rescaled, c.clamped, c.mirrored);
    let bad: Vec<usize> = (0..ics.len())
        .filter(|&i| {
            flags(&cpu[i]) != flags(&g[i])
                || (!cpu[i].rescaled && g[i].lambda.to_bits() != 1.0f32.to_bits())
        })
        .collect();
    match bad.first() {
        None => Ok(()),
        Some(&i) => Err(format!(
            "{name}: CPU-f32 and GPU-f32 differ on {} of {} states; first, state {i}: (rescaled, clamped, mirrored) \
             CPU {:?} GPU {:?}, GPU λ = {:e}; {:?}",
            bad.len(),
            ics.len(),
            flags(&cpu[i]),
            flags(&g[i]),
            g[i].lambda,
            ics[i]
        )),
    }
}

#[test]
fn canonicaliser_qa_m247_branches_agree() {
    let h = GpuHarness::new().expect("a GPU device");
    let mut failures = Vec::new();
    let mut summary = Vec::new();
    for (name, ics) in families() {
        let g = gpu(&h, &ics);
        assert_eq!(g.len(), ics.len(), "{name}: the GPU's count");
        let cpu: Vec<Canonical<f32>> = ics.iter().map(canonicalise).collect();
        let count = |f: fn(&Canonical<f32>) -> bool| cpu.iter().filter(|c| f(c)).count();
        summary.push(format!(
            "{name}: {} states ({} rescaled, {} clamped, {} mirrored)",
            ics.len(),
            count(|c| c.rescaled),
            count(|c| c.clamped),
            count(|c| c.mirrored)
        ));
        if let Err(e) = branches(name, &ics, &cpu, &g) {
            failures.push(e);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!(
        "canonicaliser_qa_m247_branches_agree on {}: the same skip, guard and mirror; {}",
        h.adapter_info().name,
        summary.join("; ")
    );
}

negative_control!(
    canonicaliser_qa_m247_branches_agree,
    "the CPU's result for the state at 1 + τ_I read against the GPU's for the state just above it differs in the \
     skip, so the comparison must fail",
    expected = "differ on",
    {
        let h = GpuHarness::new().expect("a GPU device");
        let edge = edge_states();
        let ics = [edge[2].0];
        let g = gpu(&h, &[edge[3].0]);
        let cpu = [canonicalise(&ics[0])];
        branches("swapped", &ics, &cpu, &g).unwrap();
    }
);

/// `Σmᵢ‖rᵢ − R_com‖²`, in f64.
fn inertia(x: &Ic<f64>) -> f64 {
    let t: f64 = x.m.iter().sum();
    let c = [0, 1].map(|k| (0..3).map(|i| x.m[i] * x.r[i][k]).sum::<f64>() / t);
    (0..3)
        .map(|i| x.m[i] * ((x.r[i][0] - c[0]).powi(2) + (x.r[i][1] - c[1]).powi(2)))
        .sum()
}

/// One GPU output against REQ-ENC-006: every output finite; the bodies apart, `I = 1` within 16 f32 eps; the bodies
/// coincident, `p = 0`, `λ = 1`, `φ = +0`.
fn check_output(c: &Canonical<f32>, coincident: bool, at: &str) -> Result<f64, String> {
    let w = widen(&c.ic);
    let finite =
        w.r.iter()
            .chain(w.p.iter())
            .all(|v| v[0].is_finite() && v[1].is_finite())
            && c.lambda.is_finite()
            && c.phi.is_finite()
            && c.lambda_t_y.is_finite();
    if !finite {
        return Err(format!("{at}: an output is not finite: {c:?}"));
    }
    if coincident {
        let zero_p = w.p.iter().all(|v| v[0] == 0.0 && v[1] == 0.0);
        return if zero_p && c.lambda == 1.0 && c.phi.to_bits() == 0 && c.clamped && !c.rescaled {
            Ok(0.0)
        } else {
            Err(format!("{at}: not the coincident select: {c:?}"))
        };
    }
    let err = (inertia(&w) - 1.0).abs() / f64::from(f32::EPSILON);
    if err > 16.0 {
        return Err(format!("{at}: I after C is 1 to {err} eps"));
    }
    Ok(err)
}

#[test]
fn canonicaliser_qa_m247_range() {
    let h = GpuHarness::new().expect("a GPU device");
    let scaled = scaled_states(64, 47, &SCALES);
    let coincident = coincident_states();
    let g = gpu(&h, &scaled);
    let gc = gpu(&h, &coincident);
    let mut worst = 0.0f64;
    let mut failures = Vec::new();
    for (i, c) in g.iter().enumerate() {
        match check_output(
            c,
            false,
            &format!("scaled state {i} (s = {:e})", SCALES[i % SCALES.len()]),
        ) {
            Ok(e) => worst = worst.max(e),
            Err(e) => failures.push(e),
        }
    }
    for (i, c) in gc.iter().enumerate() {
        if let Err(e) = check_output(c, true, &format!("coincident state {i}")) {
            failures.push(e);
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures; first: {}",
        failures.len(),
        failures.first().map(String::as_str).unwrap_or("")
    );
    println!(
        "canonicaliser_qa_m247_range on {}: {} scaled states at {SCALES:?}, every output finite, I = 1 to {worst} eps; \
         {} coincident states keep the select",
        h.adapter_info().name,
        g.len(),
        gc.len()
    );
}

negative_control!(
    canonicaliser_qa_m247_range,
    "a GPU output with its positions doubled is at I = 4, so the check must fail",
    expected = "I after C is 1 to",
    {
        let h = GpuHarness::new().expect("a GPU device");
        let mut c = gpu(&h, &scaled_states(1, 47, &[1.0]))[0];
        c.ic.r = c.ic.r.map(|v| v.map(|k| 2.0 * k));
        check_output(&c, false, "doubled").unwrap();
    }
);
