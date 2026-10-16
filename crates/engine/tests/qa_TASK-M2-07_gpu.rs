//! QA tests for TASK-M2-07 on the GPU, written from the requirements: `C`'s branch decisions on CPU-f32 and GPU-f32
//! on identical inputs. The CPU side is `kernel`'s `qa_TASK-M2-07`.
//! - REQ-DEC-017, REQ-ENC-015, REQ-ENC-016; R-34, R-84; dd_decoder §5 test 12 ("the degenerate/feasibility branch tags
//!   are bit-identical across all backends"): for every state fed to both, the mirror (`lookup_mirrored`) and the
//!   `ρ = 0` guard (`lookup_clamped`) are the same on CPU-f32 and GPU-f32, with no condition on the intermediate
//!   values agreeing first (pitfalls §9: a check conditioned on the bits agreeing masks the fork). The states:
//!   collinear configurations at random angles, where `λ̃_y` is rounding alone; deadband-straddling turned states,
//!   `λ̃_y = k·δ_λ`; coincident inner pairs, `r₀ = r₁`, at random places, and `ρ = (±0, ±0)`; and generic states
//!   (`canonicaliser_qa_m207_branches_agree`). The GPU gives the same words twice (`canonicaliser_qa_m207_twice`).
//! - REQ-DEC-017, R-82 (`canonicaliser_qa_m207_edge_range`): hierarchical states, the inner pair down to `2^−40` of
//!   the outer distance, at `λ̃_y = ∓k·δ_λ` with `k` `10⁻³` either side of 1, under exact quarter turns, reflections and
//!   power-of-two scales up to `2^±60` (`I_in` a finite normal f32): GPU-f32 and CPU-f32 each mirror exactly where
//!   `λ̃_y < −δ_λ`. The CPU side over the full exponent range is `kernel`'s `qa_TASK-M2-07_edge`.
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

// ── canonicaliser_qa_m207_edge_range (REQ-DEC-017; round 4) ─────────────────────────────────────────────────────

/// The `k` of `λ̃_y = −k·δ_λ`: inside the deadband (no mirror) and outside it (a mirror), `10⁻³` either side of 1.
const EDGE_KS: [f64; 4] = [0.5, 1.0 - 1e-3, 1.0 + 1e-3, 2.0];

/// A hierarchical f32 state with its inner pair `2^j·(1…2)` long on `+x` from the origin, the third body at
/// `x₂ ∈ [0.5, 1.5]` and `y₂` set in f64 so that `λ̃_y = √(m₂M₀₁)·y₂/√I_in = −k·δ_λ` (dd_encode §3.2 steps 1a, 0, 3;
/// with `ρ` on `+x`, `λ_y = y₂`), then rounded to f32; the rounded state's own `λ̃_y`, recomputed at f64, is checked
/// to land within `10⁻⁵·k` of `−k·δ_λ`, well inside the `10⁻³` margin.
fn edge_base(rng: &mut SplitMix, j: i32, k: f64) -> Ic<f32> {
    let m = masses(rng).map(|v| f64::from(v as f32));
    let a = f64::from((2f64.powi(j) * f64::from(rng.uniform(1.0, 2.0))) as f32);
    let x2 = f64::from(rng.uniform(0.5, 1.5));
    let m01 = m[0] + m[1];
    let lx = x2 - m[1] * a / m01;
    let i = m[0] * m[1] / m01 * a * a + m[2] * m01 * lx * lx;
    let y2 = -k * DELTA * (i / (m[2] * m01)).sqrt();
    let x = narrow(&Ic {
        m,
        r: [[0.0, 0.0], [a, 0.0], [x2, y2]],
        p: momenta(rng),
    });
    let r = x.r.map(|v| v.map(f64::from));
    let mf = x.m.map(f64::from);
    let com = [0, 1].map(|c| (0..3).map(|b| mf[b] * r[b][c]).sum::<f64>());
    let i_in: f64 = (0..3)
        .map(|b| mf[b] * ((r[b][0] - com[0]).powi(2) + (r[b][1] - com[1]).powi(2)))
        .sum();
    let landed = (mf[2] * (mf[0] + mf[1])).sqrt() * r[2][1] / i_in.sqrt() / DELTA;
    assert!(
        (landed + k).abs() <= 1e-5 * k,
        "the construction missed λ̃_y = −{k}·δ_λ: {landed}·δ_λ at 2^{j}"
    );
    x
}

/// `x` turned by `q` exact quarter turns, reflected (`y → −y`) where `flip`, and its positions scaled by `2^e`.
fn edge_variant(x: &Ic<f32>, q: usize, flip: bool, e: i32) -> Ic<f32> {
    let s = 2f32.powi(e);
    let f = |v: [f32; 2]| {
        let v = if flip { [v[0], -v[1]] } else { v };
        let v = (0..q).fold(v, |w, _| [-w[1], w[0]]);
        [v[0] * s, v[1] * s]
    };
    Ic {
        m: x.m,
        r: x.r.map(f),
        p: x.p,
    }
}

/// `x`, scaled from `unscaled` by a power of two, kept every coordinate and pair difference a finite normal (or a 0
/// where `unscaled` had one), and its CoM-frame `I_in` is a finite normal f32: the scaling was exact, the state is in
/// the decision's domain, and the GPU does no arithmetic on `∞` (R-436).
fn edge_in_range(x: &Ic<f32>, unscaled: &Ic<f32>) -> bool {
    let values = |r: [[f32; 2]; 3]| {
        let mut out: Vec<f32> = r.iter().flatten().copied().collect();
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            out.extend([r[j][0] - r[i][0], r[j][1] - r[i][1]]);
        }
        out
    };
    let exact = values(x.r)
        .into_iter()
        .zip(values(unscaled.r))
        .all(|(q, u)| (q == 0.0 && u == 0.0) || (q.is_finite() && q.abs() >= f32::MIN_POSITIVE));
    let r = x.r.map(|v| v.map(f64::from));
    let m = x.m.map(f64::from);
    let com = [0, 1].map(|c| (0..3).map(|b| m[b] * r[b][c]).sum::<f64>());
    let i_in: f64 = (0..3)
        .map(|b| m[b] * ((r[b][0] - com[0]).powi(2) + (r[b][1] - com[1]).powi(2)))
        .sum();
    exact && i_in < f64::from(f32::MAX) / 4.0 && i_in > 4.0 * f64::from(f32::MIN_POSITIVE)
}

/// The hierarchical edge states at every ratio, `k`, quarter turn, reflection and scale in range, each with R-82's
/// mirror: taken iff unreflected and `k > 1`.
fn edge_states() -> Vec<(Ic<f32>, bool)> {
    let mut rng = SplitMix::new(2207);
    let mut out = Vec::new();
    for j in [0, -10, -20, -30, -40] {
        for k in EDGE_KS {
            for _ in 0..16 {
                let x = edge_base(&mut rng, j, k);
                for e in [-60, -40, -20, 0, 20, 40, 60] {
                    for q in 0..4 {
                        for flip in [false, true] {
                            let v = edge_variant(&x, q, flip, e);
                            if edge_in_range(&v, &edge_variant(&x, q, flip, 0)) {
                                out.push((v, !flip && k > 1.0));
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// One backend's mirror flags, `flags`, on the edge states against R-82's; `Err` counts the misses and names the first.
fn edge_range(states: &[(Ic<f32>, bool)], flags: &[bool], name: &str) -> Result<(), String> {
    let misses: Vec<usize> = (0..states.len())
        .filter(|&s| flags[s] != states[s].1)
        .collect();
    match misses.first() {
        None => Ok(()),
        Some(&s) => Err(format!(
            "{name} mirrors against R-82 on {} of {} hierarchical edge states; first: state {s}, the mirror must be {} \
             ({:?})",
            misses.len(),
            states.len(),
            states[s].1,
            states[s].0
        )),
    }
}

#[test]
fn canonicaliser_qa_m207_edge_range() {
    let h = GpuHarness::new().expect("a GPU device");
    let states = edge_states();
    let ics: Vec<Ic<f32>> = states.iter().map(|s| s.0).collect();
    let mirrored = states.iter().filter(|s| s.1).count();
    assert!(
        states.len() > 5000 && mirrored > 0 && mirrored < states.len(),
        "the edge states: {} in range, {mirrored} to mirror",
        states.len()
    );
    let g: Vec<bool> = gpu(&h, &ics).iter().map(|c| c.mirrored).collect();
    assert_eq!(g.len(), ics.len(), "the GPU's count");
    let cpu: Vec<bool> = ics.iter().map(|x| canonicalise(x).mirrored).collect();
    let failures: Vec<String> = [("GPU-f32", &g), ("CPU-f32", &cpu)]
        .into_iter()
        .filter_map(|(name, flags)| edge_range(&states, flags, name).err())
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    eprintln!(
        "canonicaliser_qa_m207_edge_range: {} hierarchical states (inner pair to 2^-40, scaled 2^±60, I_in normal) at \
         λ̃_y = ∓k·δ_λ, k = {EDGE_KS:?}: GPU-f32 and CPU-f32 each mirror exactly the {mirrored} with λ̃_y < −δ_λ",
        states.len()
    );
}

/// R-82's test in plain f32 floats, `c < 0` and `m₂c² > δ_λ²·M₀₁·‖ρ‖²·I_in`, `c = ρ × (M₀₁r₂ − m₀r₀ − m₁r₁)`.
#[cfg(feature = "controls")]
fn edge_plain(x: &Ic<f32>) -> bool {
    let (m, r) = (x.m, x.r);
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let m01 = m[0] + m[1];
    let big = |c: usize| m01 * r[2][c] - m[0] * r[0][c] - m[1] * r[1][c];
    let c = rho[0] * big(1) - rho[1] * big(0);
    let com = [0, 1].map(|c| m[0] * r[0][c] + m[1] * r[1][c] + m[2] * r[2][c]);
    let i: f32 = (0..3)
        .map(|b| m[b] * ((r[b][0] - com[0]).powi(2) + (r[b][1] - com[1]).powi(2)))
        .sum();
    let d = DELTA as f32;
    c < 0.0 && m[2] * c * c > d * d * m01 * (rho[0] * rho[0] + rho[1] * rho[1]) * i
}

negative_control!(
    canonicaliser_qa_m207_edge_range,
    "R-82's test in plain f32 floats underflows on the hierarchical edge states and must fail",
    expected = "plain f32 mirrors against R-82",
    {
        let states = edge_states();
        let flags: Vec<bool> = states.iter().map(|s| edge_plain(&s.0)).collect();
        edge_range(&states, &flags, "plain f32").unwrap();
    }
);
