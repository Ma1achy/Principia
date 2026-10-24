//! Canonicalisation `C` on the GPU (`kernel::stage::canon`, `engine::decode_dispatch::canonicalise_gpu`; TASK-M2-07): the
//! kernel's f32 build, `target/spirv/kernel.wgsl`, dispatched natively, read back and checked against the same source on
//! the CPU at f64 and f32. CI runs this file in the `gpu-kernel` job, after `cargo xtask build-kernel`, and on Metal.
//! - `canonicaliser_mirror_tie`: states at `ρ = 0` whose steps 1a, 0, 1b and 2 are exact no-ops and on which the mirror
//!   decision's arithmetic is exact, so `λ̃_y` is exactly `±δ_λ` and its neighbours at each precision: no mirror for
//!   `|λ̃_y| ≤ δ_λ`, a mirror just below `−δ_λ`, the same output twice, and the same branch on CPU-f64, CPU-f32 and
//!   GPU-f32; the test reads `λ̃_y`, not `λ_y`. Turned, translated and boosted ties take the same branch on CPU-f32 and
//!   GPU-f32, every one (REQ-DEC-017).
//! - `canonicaliser_seam_sweep`: collinear f32 states at random angles, all at the `λ̃_y = 0` seam, some with `ρ = 0`:
//!   CPU-f32 and GPU-f32 take the same mirror branch and raise the same `lookup_clamped` on every one (REQ-DEC-017,
//!   REQ-ENC-016).
//! - `canonicaliser_scale_invariance`: 2000 generic states scaled by powers of two from `2^−60` to `2^60` and by
//!   `1e-8` to `1e18`: GPU-f32 takes CPU-f32's branch and the f64 one at every scale, and its notices at a power of
//!   two are those at `2⁰` (REQ-DEC-017).
//! - `canonicaliser_hierarchy`: hierarchical collinear states, the inner pair `10⁻³` to `10⁻⁸` of the outer distance:
//!   GPU-f32 takes CPU-f32's branch at the deadband's edge and mirrors at `λ̃_y = −2δ_λ`, where Metal flushes
//!   subnormals (REQ-DEC-017).
//! - `canonicaliser_hierarchy_edge`: states within 30 f32 steps of the deadband's edge, the inner pair `2^−12` to
//!   `2^−40` of the outer distance, on `+x` and turned: GPU-f32 takes CPU-f32's branch on every one, and on `+x`
//!   CPU-f32 takes R-82's decision at f64 on the same inputs except within 4 f32 steps of the edge (REQ-DEC-017).
//! - `canonicaliser_canonicalise_order`: an off-CoM, boosted, scaled, rotated state, mirrored or not: `I` is the
//!   CoM-frame one, and the output is the canonical representative on both backends (REQ-ENC-015).
//! - `canonicaliser_coincident_inner_pair`: `ρ = 0` raises `lookup_clamped` with `φ = +0` and the identity rotation on
//!   CPU-f64 and GPU-f32, `ρ = (±0, ±0)` included; a `ρ` one step off 0 does not (REQ-ENC-016, R-13, R-437).
//! - `canonicaliser_phi_signed_zero`: `ρ_x = −0` exactly with `ρ_y ≠ 0`: `φ` is `±π/2` on `ρ_y`'s side on CPU-f64,
//!   CPU-f32 and GPU-f32, Metal's `atan2(y, −0)` sign kept out by `rotation_angle`'s select (pitfalls §11; REQ-ENC-016).
//! - `canonicaliser_noop_gpu`: dd_decoder §5 test 5 on the GPU, `C` on the GPU's own canonical-frame decodes, saturated
//!   controls included, takes no mirror, turns by 0 and moves nothing beyond rounding (REQ-DEC-018).
//!
//! **Domain** (pitfalls §10): `C` alone, on states held fixed; no trajectory. **PIT-10:** the cross-backend branch claim
//! is for identical inputs. The backends' recorded `λ̃_y` and `φ` can differ in their last bits (WGSL's `√`, `/` and
//! `atan2` are not correctly rounded); the branches are decided before them (`kernel::canon`'s note) and do not.

use std::path::{Path, PathBuf};

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{canonicalise_gpu, DecodeStage};
use kernel::canon::{canonicalise, inertia, Canonical};
use kernel::chart::Latent;
use kernel::decode::{decode, Ic};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, LinkReal};
use kernel::real::IeeeBits;
use kernel::stage::decode::{DecodeMode, LatentDefaults};
use validation::decode_f32_eps::{interior, Batch, Fuzz, SplitMix};
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

/// The GPU's `C` of `ics`.
fn gpu(h: &GpuHarness, ics: &[Ic<f32>]) -> Vec<Canonical<f32>> {
    canonicalise_gpu(h.device(), h.queue(), &wgsl(), ics, FastMath::default())
        .expect("the canonicalise dispatch")
}

fn narrow(ic: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: ic.m.map(|x| x as f32),
        r: ic.r.map(|v| v.map(|x| x as f32)),
        p: ic.p.map(|v| v.map(|x| x as f32)),
    }
}

fn widen(ic: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: ic.m.map(f64::from),
        r: ic.r.map(|v| v.map(f64::from)),
        p: ic.p.map(|v| v.map(f64::from)),
    }
}

fn norm(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}

/// The largest component difference of `a` and `b`, positions over `a`'s largest `‖rᵢ‖` and momenta over its largest
/// `‖pᵢ‖` (1 where they are all zero), in `eps`.
fn state_error(a: &Ic<f64>, b: &Ic<f64>, eps: f64) -> f64 {
    let scale = |v: &[[f64; 2]; 3]| {
        let s = v.iter().map(|x| norm(*x)).fold(0.0, f64::max);
        if s == 0.0 {
            1.0
        } else {
            s
        }
    };
    let (sr, sp) = (scale(&a.r), scale(&a.p));
    let mut worst = 0.0f64;
    for i in 0..3 {
        for k in 0..2 {
            worst = worst.max((a.r[i][k] - b.r[i][k]).abs() / (sr * eps));
            worst = worst.max((a.p[i][k] - b.p[i][k]).abs() / (sp * eps));
        }
    }
    worst
}

/// The bits of every word of `c`, notices included, so two outputs compare bit for bit.
fn bits(c: &Canonical<f32>) -> Vec<u32> {
    let ic = &c.ic;
    ic.m.iter()
        .chain(ic.r.as_flattened())
        .chain(ic.p.as_flattened())
        .chain(&[c.i_in, c.lambda, c.phi, c.lambda_t_y])
        .map(|x| x.to_bits())
        .chain([u32::from(c.clamped), u32::from(c.mirrored)])
        .collect()
}

const EPS32: f64 = f32::EPSILON as f64;

// ── canonicaliser_mirror_tie (REQ-DEC-017) ──────────────────────────────────────────────────────────────────────

/// A tie state, masses `(¼, ¼, ½)`, `r₀ = r₁ = (−1, a)`, `r₂ = (1, −a)`, at rest: its CoM is exactly 0, `I = 1`
/// exactly, `ρ = 0`, so step 2 is the identity (`lookup_clamped`, `φ = 0`), `Λ = M₀₁r₂ − m₀r₀ − m₁r₁ = (1, −a)` and
/// `λ_y = Λ_y/M₀₁ = −2a`, so `λ̃_y = √(m₂M₀₁)·λ_y = ½·(−2a) = −a`, each exact. Every operation of the mirror decision
/// is exact on it but `a²` and `δ_λ²`, so its sides compare `a²/2` with `δ_λ²/2`, each one rounding: `C`'s branch at
/// the deadband's edge is the same comparison on every backend.
fn tie<R: LinkReal + IeeeBits>(a: R) -> Ic<R> {
    let (q, h, z, one) = (
        R::from(0.25).unwrap(),
        R::from(0.5).unwrap(),
        R::zero(),
        R::one(),
    );
    Ic {
        m: [q, q, h],
        r: [[-one, a], [-one, a], [one, -a]],
        p: [[z; 2]; 3],
    }
}

/// The tie's companion off the pole, `r₀ = (−1, a)`, `r₁ = (1, a)`, `r₂ = (0, −a)`: `ρ = (2, 0)`, `I = ½` and
/// `λ̃_y = −√2·a`, near the deadband's edge, not on it, for the turned ties.
fn axis_tie(a: f64) -> Ic<f64> {
    Ic {
        m: [0.25, 0.25, 0.5],
        r: [[-1.0, a], [1.0, a], [0.0, -a]],
        p: [[0.0; 2]; 3],
    }
}

/// The tie's `a` values, so `λ̃_y = −a`: `±δ_λ`, one step either side of each, `±2δ_λ`, and `±0`; and whether each
/// must mirror, `−a < −δ_λ`.
fn tie_cases<R: LinkReal>(up: fn(R) -> R, down: fn(R) -> R) -> Vec<(R, bool)> {
    let d = R::DELTA_LAMBDA;
    let two = d + d;
    vec![
        (d, false),
        (up(d), true),
        (down(d), false),
        (two, true),
        (-d, false),
        (-up(d), false),
        (-down(d), false),
        (-two, false),
        (R::zero(), false),
        (-R::zero(), false),
    ]
}

/// The tie on one backend's outputs: for each case `(a, mirror)` and its `C`, the branch input read back is `−a` exactly
/// (`λ̃_y`, not `λ_y = −2a`), steps 0 and 2 were the identity, and the mirror is taken iff `mirror`, which is `−a < −δ_λ`.
fn check_tie<R: LinkReal + Into<f64> + std::fmt::LowerExp>(
    cases: &[(R, bool)],
    outs: &[Canonical<R>],
    backend: &str,
) {
    for ((a, mirror), c) in cases.iter().zip(outs) {
        let at = format!("{backend}, λ̃_y = −({a:e})");
        assert_eq!(c.lambda, R::one(), "step 0 rescaled at {at}");
        assert!(c.phi == R::zero(), "step 2 turned by {:?} at {at}", c.phi);
        assert!(
            c.lambda_t_y == -*a,
            "the branch input is {:?}, not λ̃_y = {:?}, at {at}",
            c.lambda_t_y,
            -*a
        );
        assert_eq!(
            c.mirrored, *mirror,
            "the mirror is {} at {at}, against λ̃_y < −δ_λ",
            c.mirrored
        );
        // `r₂ = (0, −a)` comes out as `(0, a)` iff mirrored: compared on the bits, so `a = ±0` reads its own sign.
        let bits = |x: R| Into::<f64>::into(x).to_bits();
        let flipped = bits(c.ic.r[2][1]) == bits(*a);
        assert_eq!(
            flipped, c.mirrored,
            "r₂'s y does not follow the mirror flag at {at}"
        );
        assert!(c.clamped, "no lookup_clamped at {at}, where ρ = 0");
    }
}

/// The tie's states rotated by `θ`, translated and boosted: `λ̃_y` is now within rounding of `−a`, not exact.
fn turned(x: &Ic<f64>, theta: f64) -> Ic<f64> {
    let (s, c) = theta.sin_cos();
    let rot = |v: [f64; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    Ic {
        m: x.m,
        r: x.r.map(|v| {
            let v = rot(v);
            [v[0] + 0.75, v[1] - 1.25]
        }),
        p: [0, 1, 2].map(|i| {
            let v = rot(x.p[i]);
            [v[0] + 0.3 * x.m[i], v[1] - 0.2 * x.m[i]]
        }),
    }
}

#[test]
fn canonicaliser_mirror_tie() {
    let h = harness();
    // CPU-f64.
    let cases64 = tie_cases::<f64>(f64::next_up, f64::next_down);
    let outs64: Vec<Canonical<f64>> = cases64
        .iter()
        .map(|(a, _)| canonicalise(&tie(*a)))
        .collect();
    check_tie(&cases64, &outs64, "CPU-f64");
    for ((a, _), c) in cases64.iter().zip(&outs64) {
        assert_eq!(
            canonicalise(&tie(*a)),
            *c,
            "CPU-f64 gave two results at a = {a:e}"
        );
    }
    // CPU-f32 and GPU-f32, on identical inputs.
    let cases32 = tie_cases::<f32>(f32::next_up, f32::next_down);
    let states: Vec<Ic<f32>> = cases32.iter().map(|(a, _)| tie(*a)).collect();
    let cpu32: Vec<Canonical<f32>> = states.iter().map(canonicalise).collect();
    check_tie(&cases32, &cpu32, "CPU-f32");
    let (first, second) = (gpu(&h, &states), gpu(&h, &states));
    check_tie(&cases32, &first, "GPU-f32");
    for (k, (g, c)) in first.iter().zip(&cpu32).enumerate() {
        assert_eq!(
            bits(g),
            bits(&second[k]),
            "GPU-f32 gave two results at case {k}"
        );
        assert_eq!(
            g.lambda_t_y.to_bits(),
            c.lambda_t_y.to_bits(),
            "GPU-f32 and CPU-f32 read different λ̃_y at case {k}"
        );
    }
    eprintln!(
        "canonicaliser_mirror_tie: {} exact ties per precision; δ_λ = {:e} (f64), {:e} (f32)",
        cases64.len(),
        f64::DELTA_LAMBDA,
        f32::DELTA_LAMBDA
    );
    // Turned, translated and boosted ties: λ̃_y within rounding of the deadband's edges. Each backend gives the same
    // output twice, and CPU-f32 and GPU-f32 take the same branch on every one, on identical inputs (PIT-10).
    let turns: Vec<Ic<f64>> = cases64
        .iter()
        .flat_map(|(a, _)| {
            [0.3, 1.9, -2.6]
                .into_iter()
                .flat_map(move |t| [turned(&tie(*a), t), turned(&axis_tie(*a / 2f64.sqrt()), t)])
        })
        .collect();
    let narrow_turns: Vec<Ic<f32>> = turns.iter().map(narrow).collect();
    let (first, second) = (gpu(&h, &narrow_turns), gpu(&h, &narrow_turns));
    let cpu_turns: Vec<Canonical<f32>> = narrow_turns.iter().map(canonicalise).collect();
    for (k, x) in turns.iter().enumerate() {
        assert_eq!(
            canonicalise(x),
            canonicalise(x),
            "CPU-f64 gave two results at turn {k}"
        );
        assert_eq!(
            bits(&first[k]),
            bits(&second[k]),
            "GPU-f32 gave two results at turn {k}"
        );
    }
    let mirrors = check_branches(&first, &cpu_turns, "turn");
    eprintln!(
        "canonicaliser_mirror_tie: {} turned ties, {mirrors} mirrored, the same branch on CPU-f32 and GPU-f32",
        turns.len()
    );
}

/// The backends' branches agree on every state: `lookup_mirrored` and `lookup_clamped` of GPU-f32's `gpu` against
/// CPU-f32's `cpu`, the same inputs; the number mirrored.
fn check_branches(gpu: &[Canonical<f32>], cpu: &[Canonical<f32>], what: &str) -> usize {
    assert_eq!(gpu.len(), cpu.len());
    for (k, (g, c)) in gpu.iter().zip(cpu).enumerate() {
        assert_eq!(
            g.mirrored, c.mirrored,
            "the backends' mirror branches differ at {what} {k}: GPU-f32 {g:?}, CPU-f32 {c:?}"
        );
        assert_eq!(
            g.clamped, c.clamped,
            "the backends' lookup_clamped differ at {what} {k}: GPU-f32 {g:?}, CPU-f32 {c:?}"
        );
    }
    cpu.iter().filter(|c| c.mirrored).count()
}

negative_control!(
    canonicaliser_mirror_tie,
    "a mirror test reading λ_y = λ̃_y/√μ_λ, the defect R-82 names, mirrors at λ̃_y = −δ_λ and must fail",
    expected = "the mirror is true at CPU-f64, λ̃_y = −(1e-12)",
    {
        let cases = tie_cases::<f64>(f64::next_up, f64::next_down);
        let outs: Vec<Canonical<f64>> = cases
            .iter()
            .map(|(a, _)| {
                let mut c = canonicalise(&tie(*a));
                // λ_y = λ̃_y/√(m₂M₀₁) = 2λ̃_y here.
                c.mirrored = 2.0 * c.lambda_t_y < -f64::DELTA_LAMBDA;
                c
            })
            .collect();
        check_tie(&cases, &outs, "CPU-f64")
    }
);

// ── canonicaliser_seam_sweep (REQ-DEC-017, REQ-ENC-016) ─────────────────────────────────────────────────────────

/// The sweep's size.
const SWEEP: usize = 20_000;

/// Collinear f32 states through a random point at a random angle, so `λ̃_y` is 0 in the reals and its computed value is
/// rounding: the `λ̃_y = 0` seam, at random masses and momenta. Every eighth has `r₁ = r₀` bit for bit, `ρ = 0`.
fn seam_states() -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(41);
    (0..SWEEP)
        .map(|n| {
            let w = [0; 3].map(|_| f64::from(rng.uniform(0.2, 1.2)));
            let total: f64 = w.iter().sum();
            let theta = f64::from(rng.uniform(0.0, 1.0)) * std::f64::consts::TAU;
            let (sn, cs) = theta.sin_cos();
            let origin = [0; 2].map(|_| f64::from(rng.uniform(-0.5, 0.5)));
            let mut t = [0; 3].map(|_| f64::from(rng.uniform(-1.0, 1.0)));
            if n % 8 == 0 {
                t[1] = t[0];
            }
            Ic {
                m: w.map(|x| (x / total) as f32),
                r: t.map(|ti| [(origin[0] + ti * cs) as f32, (origin[1] + ti * sn) as f32]),
                p: [0; 3].map(|_| [rng.uniform(-0.5, 0.5), rng.uniform(-0.5, 0.5)]),
            }
        })
        .collect()
}

#[test]
fn canonicaliser_seam_sweep() {
    let h = harness();
    let states = seam_states();
    let g = gpu(&h, &states);
    let c: Vec<Canonical<f32>> = states.iter().map(canonicalise).collect();
    let mirrors = check_branches(&g, &c, "seam state");
    let clamped = c.iter().filter(|x| x.clamped).count();
    let lambda_bits = g
        .iter()
        .zip(&c)
        .filter(|(a, b)| a.lambda_t_y.to_bits() != b.lambda_t_y.to_bits())
        .count();
    let phi_bits = g
        .iter()
        .zip(&c)
        .filter(|(a, b)| a.phi.to_bits() != b.phi.to_bits())
        .count();
    assert!(
        mirrors > 0 && mirrors < SWEEP,
        "the sweep reached one branch only ({mirrors} of {SWEEP} mirrored)"
    );
    assert!(clamped >= SWEEP / 8, "only {clamped} states at ρ = 0");
    eprintln!(
        "canonicaliser_seam_sweep: {SWEEP} seam states, {mirrors} mirrored, {clamped} at ρ = 0, the same branches on \
         CPU-f32 and GPU-f32; the recorded λ̃_y differs in {lambda_bits} and φ in {phi_bits} (not branched on)"
    );
}

negative_control!(
    canonicaliser_seam_sweep,
    "branching on each backend's own recorded λ̃_y, formed through √, / and the rotation, must fail",
    expected = "the backends' mirror branches differ",
    {
        let h = harness();
        let states = seam_states();
        let mut g = gpu(&h, &states);
        let mut c: Vec<Canonical<f32>> = states.iter().map(canonicalise).collect();
        for x in g.iter_mut().chain(c.iter_mut()) {
            x.mirrored = x.lambda_t_y < -f32::DELTA_LAMBDA;
        }
        check_branches(&g, &c, "seam state");
    }
);

// ── canonicaliser_scale_invariance (REQ-DEC-017) ───────────────────────────────────────────────────────────────

/// Generic f64 states, random masses, positions and momenta, `λ̃_y` anywhere.
fn generic(n: usize) -> Vec<Ic<f64>> {
    let mut rng = SplitMix::new(47);
    let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    (0..n)
        .map(|_| {
            let w = [u(0.2, 1.2), u(0.2, 1.2), u(0.2, 1.2)];
            let total: f64 = w.iter().sum();
            Ic {
                m: w.map(|x| x / total),
                r: [0; 3].map(|_| [u(-1.0, 1.0), u(-1.0, 1.0)]),
                p: [0; 3].map(|_| [u(-0.5, 0.5), u(-0.5, 0.5)]),
            }
        })
        .collect()
}

/// The scales: powers of two near `1e±8`, `1e±9`, `1e±12` and `1e±18`, under which the f32 inputs scale exactly; and
/// decimal ones, `1e-8`, `1e-9`, `1e-10`, `1e-12`, `1e16` and `1e18`, where the f32 inputs round anew.
const POW2_SCALES: [i32; 8] = [-60, -40, -30, -27, 27, 30, 40, 60];
const DECIMAL_SCALES: [f64; 6] = [1e-8, 1e-9, 1e-10, 1e-12, 1e16, 1e18];

/// `x`'s positions times `k`, then narrowed to f32.
fn scaled32(x: &Ic<f64>, k: f64) -> Ic<f32> {
    narrow(&Ic {
        m: x.m,
        r: x.r.map(|v| v.map(|q| q * k)),
        p: x.p,
    })
}

/// The mirror flags on GPU-f32 against `cpu`'s at each scale, on identical inputs, and against the f64 flag of the
/// unscaled state; and, at the powers of two, against GPU-f32's own at `2⁰`. The number mirrored at `2⁰`.
fn check_scales(h: &GpuHarness, cpu: fn(&Ic<f32>) -> bool) -> usize {
    let states = generic(2000);
    let truth: Vec<bool> = states.iter().map(|x| canonicalise(x).mirrored).collect();
    let base = gpu(
        h,
        &states.iter().map(|x| scaled32(x, 1.0)).collect::<Vec<_>>(),
    );
    let scales = POW2_SCALES
        .iter()
        .map(|e| (format!("2^{e}"), 2f64.powi(*e)))
        .chain(DECIMAL_SCALES.iter().map(|k| (format!("{k:e}"), *k)));
    for (name, k) in scales {
        let inputs: Vec<Ic<f32>> = states.iter().map(|x| scaled32(x, k)).collect();
        let g = gpu(h, &inputs);
        for (j, x) in inputs.iter().enumerate() {
            assert_eq!(
                g[j].mirrored,
                cpu(x),
                "GPU-f32 and CPU-f32 mirror differently at scale {name}, state {j}"
            );
            assert_eq!(
                g[j].mirrored, truth[j],
                "GPU-f32's mirror at scale {name} is not the f64 one at state {j}"
            );
            if k.log2().fract() == 0.0 {
                assert_eq!(
                    bits(&g[j])[19..],
                    bits(&base[j])[19..],
                    "GPU-f32's notices at scale {name} differ from 2⁰'s at state {j}"
                );
            }
        }
    }
    truth.iter().filter(|m| **m).count()
}

#[test]
fn canonicaliser_scale_invariance() {
    let h = harness();
    let mirrors = check_scales(&h, |x| canonicalise(x).mirrored);
    assert!(mirrors > 0, "no generic state mirrored");
    eprintln!(
        "canonicaliser_scale_invariance: 2000 generic states, {mirrors} mirrored; GPU-f32 takes the CPU's branch and \
         the f64 one at {} scales, 1e-8 and 1e-9 included",
        POW2_SCALES.len() + DECIMAL_SCALES.len()
    );
}

/// The decision without its range rescale, at f32, for the control.
#[cfg(feature = "controls")]
fn unscaled(x: &Ic<f32>) -> bool {
    let (m, r) = (x.m, x.r);
    let com = [0, 1].map(|k| m[0] * r[0][k] + m[1] * r[1][k] + m[2] * r[2][k]);
    let x = r.map(|v| [v[0] - com[0], v[1] - com[1]]);
    let rho = [x[1][0] - x[0][0], x[1][1] - x[0][1]];
    let m01 = m[0] + m[1];
    let big = |k: usize| m01 * x[2][k] - m[0] * x[0][k] - m[1] * x[1][k];
    let c = rho[0] * big(1) - rho[1] * big(0);
    let d = f32::DELTA_LAMBDA;
    c < 0.0 && c * c * m[2] > d * d * m01 * (rho[0] * rho[0] + rho[1] * rho[1]) * inertia(m, x)
}

negative_control!(
    canonicaliser_scale_invariance,
    "a CPU decision without the range rescale loses the branch at the far scales and must fail",
    expected = "GPU-f32 and CPU-f32 mirror differently at scale",
    {
        check_scales(&harness(), unscaled);
    }
);

// ── canonicaliser_hierarchy (REQ-DEC-017) ───────────────────────────────────────────────────────────────────────

/// The inner pair's separation over the outer distance.
const RATIOS: [f64; 6] = [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8];

/// Hierarchical collinear f32 states: `r₀ = 0`, `r₁ = ρ` with `‖ρ‖ = ratio`, and `r₂` at distance `0.5…2` along `ρ`'s line
/// from the pair's centre of mass, offset across it so that `λ̃_y` is `−k·δ_λ`; `k` is `1 ± 10⁻⁶` at the deadband's
/// edge, or 2, which must mirror. On `+x` where `axis`, so the offset survives the rounding to f32; at a random angle
/// otherwise, where it does not, and the state is at the seam.
fn hierarchy(ratio: f64, axis: bool, edge: bool, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    (0..2000)
        .map(|_| {
            let w = [u(0.01, 1.0), u(0.01, 1.0), u(0.01, 1.0)];
            let total: f64 = w.iter().sum();
            let m = w.map(|x| f64::from((x / total) as f32));
            let m01 = m[0] + m[1];
            let theta = if axis {
                0.0
            } else {
                u(0.0, 1.0) * std::f64::consts::TAU
            };
            let (dir, across) = ([theta.cos(), theta.sin()], [-theta.sin(), theta.cos()]);
            let rho = dir.map(|q| ratio * q);
            let inner = rho.map(|q| m[1] * q / m01);
            let l = u(0.5, 2.0);
            let i = m[0] * m[1] / m01 * ratio * ratio + m[2] * m01 * l * l;
            let k = if edge { 1.0 + u(-1e-6, 1e-6) } else { 2.0 };
            let ly = -k * f64::DELTA_LAMBDA * i.sqrt() / (m[2] * m01).sqrt();
            let r2 = [0, 1].map(|j| inner[j] + l * dir[j] + ly * across[j]);
            Ic {
                m: m.map(|x| x as f32),
                r: [[0.0, 0.0], rho.map(|q| q as f32), r2.map(|q| q as f32)],
                p: [[0.1, 0.0], [0.0, -0.2], [0.05, 0.1]],
            }
        })
        .collect()
}

/// One backend's mirror flags on the hierarchical states, `flags`, against CPU-f32's on identical inputs at the edge,
/// on `+x` and at a random angle, and against R-82 (a mirror) at `λ̃_y = −2δ_λ` on `+x`, at every ratio; the number
/// mirrored at the edge.
fn check_hierarchy(flags: &dyn Fn(&[Ic<f32>]) -> Vec<bool>, name: &str) -> usize {
    let mut mirrored = 0;
    for (n, ratio) in RATIOS.into_iter().enumerate() {
        for axis in [true, false] {
            let states = hierarchy(ratio, axis, true, 61 + n as u64);
            let got = flags(&states);
            for (j, x) in states.iter().enumerate() {
                let cpu = canonicalise(x).mirrored;
                mirrored += usize::from(cpu);
                assert_eq!(
                    got[j], cpu,
                    "{name} and CPU-f32 mirror differently at ratio {ratio:e}, on +x {axis}, edge state {j}: {x:?}"
                );
            }
        }
        let states = hierarchy(ratio, true, false, 71 + n as u64);
        let got = flags(&states);
        for (j, x) in states.iter().enumerate() {
            assert!(
                got[j] && canonicalise(x).mirrored,
                "{name} misses the mirror at ratio {ratio:e}, λ̃_y = −2δ_λ, state {j}: {x:?}"
            );
        }
    }
    mirrored
}

#[test]
fn canonicaliser_hierarchy() {
    let h = harness();
    let mirrored = check_hierarchy(
        &|states| gpu(&h, states).iter().map(|c| c.mirrored).collect(),
        "GPU-f32",
    );
    eprintln!(
        "canonicaliser_hierarchy: ratios {RATIOS:?}, 2000 states each at the edge (on +x and turned) and at −2δ_λ; \
         GPU-f32 takes CPU-f32's branch on every edge state ({mirrored} mirrored) and mirrors every −2δ_λ one"
    );
}

/// `x`, or a zero of its sign where it is subnormal: a GPU that flushes subnormals.
#[cfg(feature = "controls")]
fn flush(x: f32) -> f32 {
    if x != 0.0 && x.abs() < f32::MIN_POSITIVE {
        0.0f32.copysign(x)
    } else {
        x
    }
}

/// The f32 product of 66974583's decision, its operands split at 12 bits, on a backend that flushes every subnormal.
#[cfg(feature = "controls")]
fn flushed_product(a: f32, b: f32) -> f32 {
    let ah = f32::from_bits(a.to_bits() & 0xFFFF_F000);
    let bh = f32::from_bits(b.to_bits() & 0xFFFF_F000);
    let (al, bl) = (a - ah, b - bh);
    let low = flush(flush(flush(ah * bl) + flush(al * bh)) + flush(al * bl));
    flush(flush(ah * bh) + low)
}

/// 66974583's decision, the differences rescaled by one power of two for the largest, emulated on a flushing GPU.
#[cfg(feature = "controls")]
fn flushed_decision(x: &Ic<f32>) -> bool {
    let (m, r) = (x.m, x.r);
    let diff = |a: [f32; 2], b: [f32; 2]| [b[0] - a[0], b[1] - a[1]];
    let (d01, d02, d12) = (diff(r[0], r[1]), diff(r[0], r[2]), diff(r[1], r[2]));
    let largest = [d01, d02, d12]
        .iter()
        .flatten()
        .map(|q| q.abs())
        .fold(0.0f32, f32::max);
    let k = largest.log2().floor() as i32;
    let s = 2f32.powi(-k);
    let (rho, d02, d12) = (d01.map(|q| q * s), d02.map(|q| q * s), d12.map(|q| q * s));
    let p = flushed_product;
    let sq = |v: [f32; 2]| flush(p(v[0], v[0]) + p(v[1], v[1]));
    let d2 = flush(
        flush(p(p(m[0], m[1]), sq(rho)) + p(p(m[0], m[2]), sq(d02))) + p(p(m[1], m[2]), sq(d12)),
    );
    let pole = rho == [0.0, 0.0];
    let axis = if pole { [1.0, 0.0] } else { rho };
    let lam = |j: usize| flush(p(m[0], d02[j]) + p(m[1], d12[j]));
    let c = flush(p(axis[0], lam(1)) - p(axis[1], lam(0)));
    let d = f32::DELTA_LAMBDA;
    let left = p(p(c, c), m[2]);
    let right = p(p(p(p(d, d), m[0] + m[1]), sq(axis)), d2);
    c < 0.0 && left > right
}

negative_control!(
    canonicaliser_hierarchy,
    "66974583's decision on a GPU that flushes subnormals, emulated, forks on hierarchical states and must fail",
    expected = "a flushing GPU",
    {
        check_hierarchy(
            &|states| states.iter().map(flushed_decision).collect(),
            "a flushing GPU",
        );
    }
);

// ── canonicaliser_hierarchy_edge (REQ-DEC-017) ─────────────────────────────────────────────────────────────────

/// The inner pair's half-separation over the outer distance, as powers of two.
const EDGE_RATIOS: [i32; 4] = [-12, -15, -20, -40];

/// `m₂c² − δ_λ²·M₀₁·‖ρ‖²·d²` at f64 on f64 inputs: positive where `|λ̃_y| > δ_λ`.
fn edge_side(m: [f64; 3], r: [[f64; 2]; 3]) -> f64 {
    let diff = |a: [f64; 2], b: [f64; 2]| [b[0] - a[0], b[1] - a[1]];
    let (rho, d02, d12) = (diff(r[0], r[1]), diff(r[0], r[2]), diff(r[1], r[2]));
    let lam = [0, 1].map(|k| m[0] * d02[k] + m[1] * d12[k]);
    let c = rho[0] * lam[1] - rho[1] * lam[0];
    let sq = |v: [f64; 2]| v[0] * v[0] + v[1] * v[1];
    let d2 = m[0] * m[1] * sq(rho) + m[0] * m[2] * sq(d02) + m[1] * m[2] * sq(d12);
    let d = f64::DELTA_LAMBDA;
    m[2] * c * c - d * d * (m[0] + m[1]) * sq(rho) * d2
}

/// R-82's decision on `x`'s f32 values, at f64: `c < 0` and `m₂c² > δ_λ²·M₀₁·‖ρ‖²·d²`.
fn edge_truth(x: &Ic<f32>) -> bool {
    let m = x.m.map(f64::from);
    let r = x.r.map(|v| v.map(f64::from));
    let lam = [0, 1].map(|k| m[0] * (r[2][k] - r[0][k]) + m[1] * (r[2][k] - r[1][k]));
    let c = (r[1][0] - r[0][0]) * lam[1] - (r[1][1] - r[0][1]) * lam[0];
    c < 0.0 && edge_side(m, r) > 0.0
}

/// States at the deadband's edge, the inner pair `2^j` (times `1…2`) of the outer distance: `r = [(−u, a), (u, a),
/// (x₂, −a)]`, turned by `θ`, with `a` bisected at f64 to where [`edge_side`] changes sign and swept over the 61 f32
/// values from 30 below it to 30 above, for 40 random mass draws.
fn hierarchy_edge(j: i32, theta: f64, seed: u64) -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(seed);
    let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    let (cs, sn) = (theta.cos(), theta.sin());
    let turn = |v: [f64; 2]| [v[0] * cs - v[1] * sn, v[0] * sn + v[1] * cs];
    let mut out = Vec::new();
    for _ in 0..40 {
        let w = [u(0.2, 1.2), u(0.2, 1.2), u(0.2, 1.2)];
        let total: f64 = w.iter().sum();
        let m32 = w.map(|x| (x / total) as f32);
        let m = m32.map(f64::from);
        let half = f64::from((2f64.powi(j) * u(1.0, 2.0)) as f32);
        let x2 = f64::from(u(0.5, 1.5) as f32);
        let state = |a: f64| [[-half, a], [half, a], [x2, -a]].map(turn);
        let (mut lo, mut hi) = (0.0f64, 1e-6f64);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if edge_side(m, state(mid)) > 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let mut a = (0.5 * (lo + hi)) as f32;
        for _ in 0..30 {
            a = a.next_down();
        }
        for _ in 0..61 {
            out.push(Ic {
                m: m32,
                r: state(f64::from(a)).map(|v| v.map(|q| q as f32)),
                p: [[0.0; 2]; 3],
            });
            a = a.next_up();
        }
    }
    out
}

/// How far from the f64 edge, in f32 steps of `a`, CPU-f32 may decide differently from R-82's decision at f64 on `+x`:
/// the decision's own rounding, a few ulps of `c²` and of its right side (measured: at most 2).
const EDGE_STEPS: usize = 4;

/// One backend's mirror flags on the edge states, `flags`, against CPU-f32's on identical inputs, on `+x` and turned,
/// at every ratio; and on `+x`, where `ρ_y = 0` and `c = ρ_xΛ_y` cancels nothing, CPU-f32 against R-82's decision at f64
/// on the same f32 inputs, to within [`EDGE_STEPS`] of the edge. Turned, `c = ρ_xΛ_y − ρ_yΛ_x` cancels about as many
/// bits as `‖ρ‖` is below the outer distance, and the f32 inputs, rounded after the turn, sit at random within
/// about `ε_f32`, not `δ_λ`, of the edge, so f32's arithmetic resolves `λ̃_y` only to about `ε_f32` there; the count
/// is printed, not bounded. Returns the number of states, the number mirrored, and the counts differing from f64 on
/// `+x` and turned.
fn check_hierarchy_edge(
    flags: &dyn Fn(&[Ic<f32>]) -> Vec<bool>,
    name: &str,
) -> (usize, usize, [usize; 2]) {
    let (mut n, mut mirrored, mut off) = (0, 0, [0, 0]);
    for (k, j) in EDGE_RATIOS.into_iter().enumerate() {
        for theta in [0.0, 0.7] {
            let states = hierarchy_edge(j, theta, 81 + k as u64);
            let got = flags(&states);
            for (i, x) in states.iter().enumerate() {
                let cpu = canonicalise(x).mirrored;
                mirrored += usize::from(cpu);
                if cpu != edge_truth(x) {
                    off[usize::from(theta != 0.0)] += 1;
                    let step = (i % 61).abs_diff(30);
                    assert!(
                        theta != 0.0 || step <= EDGE_STEPS,
                        "CPU-f32 differs from R-82's decision at f64 {step} f32 steps from the edge, ratio 2^{j} on \
                         +x, edge state {i}: {x:?}"
                    );
                }
                assert_eq!(
                    got[i], cpu,
                    "{name} and CPU-f32 mirror differently at ratio 2^{j}, turned {theta}, edge state {i}: {x:?}"
                );
                n += 1;
            }
        }
    }
    (n, mirrored, off)
}

#[test]
fn canonicaliser_hierarchy_edge() {
    let h = harness();
    let (n, mirrored, off) = check_hierarchy_edge(
        &|states| gpu(&h, states).iter().map(|c| c.mirrored).collect(),
        "GPU-f32",
    );
    eprintln!(
        "canonicaliser_hierarchy_edge: ratios 2^{EDGE_RATIOS:?}, on +x and turned, {n} states within 30 f32 steps of \
         the deadband's edge; GPU-f32 takes CPU-f32's branch on every one ({mirrored} mirrored); CPU-f32 differs \
         from R-82's decision at f64 on {} on +x, each within {EDGE_STEPS} steps of the edge, and on {} turned",
        off[0],
        off[1]
    );
}

negative_control!(
    canonicaliser_hierarchy_edge,
    "66974583's decision on a GPU that flushes subnormals, emulated, forks at the edge on hierarchical states and must \
     fail",
    expected = "a flushing GPU and CPU-f32 mirror differently",
    {
        check_hierarchy_edge(
            &|states| states.iter().map(flushed_decision).collect(),
            "a flushing GPU",
        );
    }
);

// ── canonicaliser_canonicalise_order (REQ-ENC-015) ──────────────────────────────────────────────────────────────

/// A gauge transform `g = (a, u, θ, s, Π)` of dd_encode §3.3: `rᵢ' = s·R(θ)·Π·rᵢ + a`, `pᵢ' = s^{−1/2}·R(θ)·Π·pᵢ + mᵢu`.
#[derive(Clone, Copy, Debug)]
struct Gauge {
    a: [f64; 2],
    u: [f64; 2],
    theta: f64,
    s: f64,
    mirror: bool,
}

fn apply(g: &Gauge, x: &Ic<f64>) -> Ic<f64> {
    let (sn, cs) = g.theta.sin_cos();
    let map = |v: [f64; 2], k: f64| {
        let y = if g.mirror { -v[1] } else { v[1] };
        [k * (cs * v[0] - sn * y), k * (sn * v[0] + cs * y)]
    };
    Ic {
        m: x.m,
        r: x.r.map(|v| {
            let w = map(v, g.s);
            [w[0] + g.a[0], w[1] + g.a[1]]
        }),
        p: [0, 1, 2].map(|i| {
            let w = map(x.p[i], g.s.powf(-0.5));
            [w[0] + x.m[i] * g.u[0], w[1] + x.m[i] * g.u[1]]
        }),
    }
}

/// The gauges: three scales, three angles, both mirrors, a translation of several times the state's size, a boost.
fn gauges() -> Vec<Gauge> {
    let mut out = Vec::new();
    for (k, s) in [0.5, 3.0, 40.0].into_iter().enumerate() {
        for theta in [0.7, 2.9, -1.3] {
            for mirror in [false, true] {
                out.push(Gauge {
                    a: [3.0 * s, -2.0 * s - k as f64],
                    u: [0.4, -1.1],
                    theta,
                    s,
                    mirror,
                });
            }
        }
    }
    out
}

/// The order's tolerances, in eps of each precision: the output against the canonical representative, per
/// [`state_error`]; `I_in` against `s²`, relative.
const ORDER_EPS: f64 = 16.0;
const ORDER_I_EPS: f64 = 16.0;

/// Test for one backend's `c = C(g·x)`: `I_in` is the CoM-frame `s²·I(x) = s²`, not `Σmᵢ‖rᵢ'‖²` about the origin; the
/// mirror follows `Π`; and the output is `x` within `tol` eps. The worst output error, in eps.
fn check_order(g: &Gauge, x: &Ic<f64>, c: &Canonical<f64>, eps: f64, at: &str) -> f64 {
    let di = (c.i_in - g.s * g.s).abs() / (g.s * g.s * eps);
    assert!(
        di <= ORDER_I_EPS,
        "I_in = {} is not the CoM-frame s² = {} ({di} eps) at {at}",
        c.i_in,
        g.s * g.s
    );
    assert_eq!(c.mirrored, g.mirror, "the mirror does not undo Π at {at}");
    let err = state_error(x, &c.ic, eps);
    assert!(err <= ORDER_EPS, "C(g·x) is {err} eps from x at {at}");
    err
}

/// Canonical representatives: decodes at f64 whose `λ̃_y` lies well outside the deadband, so `Π` alone decides the
/// mirror.
fn representatives() -> Vec<Ic<f64>> {
    [
        [0.4f64, -0.3, 0.5, -0.2, 0.3, 0.1, 0.6, -0.4],
        [-0.8, 1.1, -0.4, 0.7, -0.6, 0.2, -0.3, 0.9],
        [1.2, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ]
    .iter()
    .map(|z| decode::<Defaults, f64>(&kernel::decode::Latent::new(*z)).ic)
    .collect()
}

#[test]
fn canonicaliser_canonicalise_order() {
    let h = harness();
    let mut inputs = Vec::new();
    let mut cases = Vec::new();
    for (j, x) in representatives().into_iter().enumerate() {
        for (k, g) in gauges().into_iter().enumerate() {
            let y = apply(&g, &x);
            let origin = inertia(y.m, y.r);
            assert!(
                (origin - g.s * g.s).abs() > g.s * g.s,
                "the gauge's translation does not separate the CoM-frame I from the origin's"
            );
            inputs.push(narrow(&y));
            cases.push((j, k, g, x, y));
        }
    }
    let gpu_out = gpu(&h, &inputs);
    let (mut w64, mut w32) = (0.0f64, 0.0f64);
    for (n, (j, k, g, x, y)) in cases.iter().enumerate() {
        let at = format!("representative {j}, gauge {k} {g:?}");
        w64 = w64.max(check_order(
            g,
            x,
            &canonicalise(y),
            f64::EPSILON,
            &format!("CPU-f64 {at}"),
        ));
        let c = &gpu_out[n];
        let wide = Canonical {
            ic: widen(&c.ic),
            i_in: c.i_in.into(),
            lambda: c.lambda.into(),
            phi: c.phi.into(),
            lambda_t_y: c.lambda_t_y.into(),
            clamped: c.clamped,
            mirrored: c.mirrored,
        };
        w32 = w32.max(check_order(g, x, &wide, EPS32, &format!("GPU-f32 {at}")));
    }
    eprintln!(
        "canonicaliser_canonicalise_order: worst output error {w64} eps (CPU-f64), {w32} eps (GPU-f32); tolerance \
         {ORDER_EPS}"
    );
}

negative_control!(
    canonicaliser_canonicalise_order,
    "a C that computes I before subtracting the CoM records Σmᵢ‖rᵢ‖² about the origin and must fail",
    expected = "is not the CoM-frame",
    {
        let x = representatives()[0];
        let g = gauges()[0];
        let y = apply(&g, &x);
        let mut c = canonicalise(&y);
        c.i_in = inertia(y.m, y.r);
        check_order(&g, &x, &c, f64::EPSILON, "I before the CoM");
    }
);

// ── canonicaliser_coincident_inner_pair (REQ-ENC-016) ───────────────────────────────────────────────────────────

/// States whose inner pair coincides, `r₀ = r₁` bit for bit, the third body elsewhere, with momenta.
fn coincident() -> Vec<Ic<f64>> {
    let mut rng = SplitMix::new(17);
    let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    (0..24)
        .map(|_| {
            let w = [u(0.1, 1.0), u(0.1, 1.0), u(0.1, 1.0)];
            let total: f64 = w.iter().sum();
            let pair = [u(-3.0, 3.0), u(-3.0, 3.0)];
            Ic {
                m: w.map(|x| x / total),
                r: [pair, pair, [u(-3.0, 3.0), u(-3.0, 3.0)]],
                p: [0; 3].map(|_| [u(-1.0, 1.0), u(-1.0, 1.0)]),
            }
        })
        .collect()
}

/// `I_in = 0`: every body at the origin, `r₁ = (s_x·0, s_y·0)` for each sign, so step 2 sees `ρ = (±0, ±0)`.
fn signed_poles() -> Vec<(&'static str, Ic<f64>)> {
    let m = [0.25, 0.375, 0.375];
    let p = [[0.1, -0.2], [0.3, 0.05], [-0.4, 0.15]];
    [
        ("(+0, +0)", 0.0, 0.0),
        ("(−0, +0)", -0.0, 0.0),
        ("(−0, −0)", -0.0, -0.0),
        ("(+0, −0)", 0.0, -0.0),
    ]
    .map(|(name, x, y)| {
        (
            name,
            Ic {
                m,
                r: [[0.0, 0.0], [x, y], [0.0, 0.0]],
                p,
            },
        )
    })
    .to_vec()
}

/// `φ` at the pole is `+0`, bit for bit, whatever the signs of `ρ`'s zeros (R-437).
fn check_pole_phi(phi: f64, at: &str) {
    assert_eq!(
        phi.to_bits(),
        0.0f64.to_bits(),
        "φ at ρ = {at} is {phi}, not +0"
    );
}

/// The identity rotation at the pole: `C`'s output is the CoM-frame state over `√I`, mirrored by its own flag, within
/// `tol` eps.
fn check_identity(x: &Ic<f64>, c: &Canonical<f64>, eps: f64, at: &str) {
    let com = [0, 1].map(|k| (0..3).map(|i| x.m[i] * x.r[i][k]).sum::<f64>());
    let i: f64 = (0..3)
        .map(|n| x.m[n] * ((x.r[n][0] - com[0]).powi(2) + (x.r[n][1] - com[1]).powi(2)))
        .sum();
    let total = [0, 1].map(|k| x.p[0][k] + x.p[1][k] + x.p[2][k]);
    let flip = if c.mirrored { -1.0 } else { 1.0 };
    let expected = Ic {
        m: x.m,
        r: x.r.map(|v| {
            [
                (v[0] - com[0]) / i.sqrt(),
                flip * (v[1] - com[1]) / i.sqrt(),
            ]
        }),
        p: [0, 1, 2].map(|n| {
            let q = i.powf(0.25);
            [
                q * (x.p[n][0] - x.m[n] * total[0]),
                flip * q * (x.p[n][1] - x.m[n] * total[1]),
            ]
        }),
    };
    let err = state_error(&expected, &c.ic, eps);
    assert!(
        err <= ORDER_EPS,
        "the rotation at ρ = 0 is not the identity ({err} eps) at {at}"
    );
}

#[test]
fn canonicaliser_coincident_inner_pair() {
    let h = harness();
    let states = coincident();
    let narrow_states: Vec<Ic<f32>> = states.iter().map(narrow).collect();
    let gpu_out = gpu(&h, &narrow_states);
    for (k, x) in states.iter().enumerate() {
        let c64 = canonicalise(x);
        let at = format!("coincident state {k}");
        assert!(c64.clamped, "CPU-f64 raised no lookup_clamped at {at}");
        check_pole_phi(c64.phi, &format!("+0 (CPU-f64, {at})"));
        check_identity(x, &c64, f64::EPSILON, &format!("CPU-f64 {at}"));
        let g = &gpu_out[k];
        assert!(g.clamped, "GPU-f32 raised no lookup_clamped at {at}");
        check_pole_phi(g.phi.into(), &format!("+0 (GPU-f32, {at})"));
        let wide = Canonical {
            ic: widen(&g.ic),
            i_in: g.i_in.into(),
            lambda: g.lambda.into(),
            phi: g.phi.into(),
            lambda_t_y: g.lambda_t_y.into(),
            clamped: g.clamped,
            mirrored: g.mirrored,
        };
        check_identity(
            &widen(&narrow_states[k]),
            &wide,
            EPS32,
            &format!("GPU-f32 {at}"),
        );
    }
    // ρ = (±0, ±0) exactly, at I_in = 0 (RQ-296 holds C's state there; the guard and φ are R-13's and R-437's).
    let poles = signed_poles();
    let narrow_poles: Vec<Ic<f32>> = poles.iter().map(|(_, x)| narrow(x)).collect();
    let gpu_poles = gpu(&h, &narrow_poles);
    for (k, (name, x)) in poles.iter().enumerate() {
        for (backend, c) in [
            ("CPU-f64", canonicalise(x)),
            ("CPU-f32", {
                let c = canonicalise(&narrow_poles[k]);
                Canonical {
                    ic: widen(&c.ic),
                    i_in: c.i_in.into(),
                    lambda: c.lambda.into(),
                    phi: c.phi.into(),
                    lambda_t_y: c.lambda_t_y.into(),
                    clamped: c.clamped,
                    mirrored: c.mirrored,
                }
            }),
        ] {
            assert!(
                c.clamped,
                "{backend} raised no lookup_clamped at ρ = {name}"
            );
            check_pole_phi(c.phi, &format!("{name} ({backend})"));
        }
        let g = &gpu_poles[k];
        assert!(g.clamped, "GPU-f32 raised no lookup_clamped at ρ = {name}");
        check_pole_phi(g.phi.into(), &format!("{name} (GPU-f32)"));
        let finite =
            g.ic.r
                .as_flattened()
                .iter()
                .chain(g.ic.p.as_flattened())
                .all(|v| v.is_finite())
                && g.lambda.is_finite();
        assert!(
            finite,
            "GPU-f32 formed a non-finite value at I_in = 0, ρ = {name}"
        );
    }
    // One step off 0, on either axis and either side: not the guard.
    for (name, off) in [
        ("(+δ, 0)", [1.0, 0.0]),
        ("(−δ, 0)", [-1.0, 0.0]),
        ("(0, +δ)", [0.0, 1.0]),
        ("(0, −δ)", [0.0, -1.0]),
    ] {
        let step = 2f64.powi(-20);
        let mut x = states[0];
        x.r[1] = [x.r[0][0] + off[0] * step, x.r[0][1] + off[1] * step];
        let c64 = canonicalise(&x);
        let c32 = gpu(&h, &[narrow(&x)])[0];
        assert!(!c64.clamped, "CPU-f64 raised lookup_clamped at ρ = {name}");
        assert!(!c32.clamped, "GPU-f32 raised lookup_clamped at ρ = {name}");
    }
}

negative_control!(
    canonicaliser_coincident_inner_pair,
    "φ taken from atan2 at the pole, which gives π on the CPU for a −0 ρ_x, must fail",
    expected = "φ at ρ = (−0, +0) is 3.14",
    {
        for (name, x) in signed_poles() {
            let rho = [x.r[1][0] - x.r[0][0], x.r[1][1] - x.r[0][1]];
            check_pole_phi(rho[1].atan2(rho[0]), name);
        }
    }
);

// ── canonicaliser_phi_signed_zero (REQ-ENC-016; pitfalls §11) ───────────────────────────────────────────────────

/// `φ`'s limit off `±π/2`, in f32 eps of `π/2` (applied per R-369, a test tolerance): a backend reading `ρ_x = −0`'s
/// sign turns `φ` by `π`, `2²³` eps; WGSL's `atan2` is not correctly rounded, and 4 eps leaves it room.
const SIGNED_ZERO_EPS: f64 = 4.0;

/// States whose `ρ = r₁ − r₀` has `ρ_x = −0` exactly and `ρ_y ≠ 0` (`r₁_x = −0`, `r₀_x = +0`), each sign of `ρ_y`, one
/// `ρ_y` small, and one with `ρ_x = +0` beside them: `φ = ±π/2` on `ρ_y`'s side.
fn signed_zero_rho() -> Vec<(&'static str, f64, Ic<f64>)> {
    let m = [0.25, 0.375, 0.375];
    let p = [[0.1, -0.2], [0.3, 0.05], [-0.4, 0.15]];
    let tiny = 2f64.powi(-20);
    [
        ("(−0, +2)", 1.0, [0.0, -1.0], [-0.0, 1.0], [0.7, 0.2]),
        ("(−0, −2)", -1.0, [0.0, 1.0], [-0.0, -1.0], [-0.7, 0.2]),
        ("(−0, +2⁻²⁰)", 1.0, [0.0, 0.0], [-0.0, tiny], [0.5, -0.3]),
        ("(+0, −2)", -1.0, [-0.0, 1.0], [0.0, -1.0], [0.4, 0.6]),
    ]
    .map(|(name, sign, r0, r1, r2)| {
        (
            name,
            sign,
            Ic {
                m,
                r: [r0, r1, r2],
                p,
            },
        )
    })
    .to_vec()
}

/// `φ` is `sign·π/2` within [`SIGNED_ZERO_EPS`] `eps` of `π/2`; returns the miss in `eps`.
fn check_quarter(phi: f64, sign: f64, eps: f64, at: &str) -> f64 {
    let quarter = std::f64::consts::FRAC_PI_2;
    let err = (phi - sign * quarter).abs() / (eps * quarter);
    assert!(
        err <= SIGNED_ZERO_EPS,
        "φ = {phi:e} at ρ = {at}, not {}π/2 ({err:.3e} eps)",
        if sign < 0.0 { "−" } else { "+" }
    );
    err
}

/// Metal's `atan2` through naga, as measured (pitfalls §11): `atan(y/x)`, turned by `π` to `y`'s side where `x < 0`; a
/// `−0` `x` is not below 0, so `atan2(y, −0)` is `atan(∓∞)`, the opposite sign of `π/2` to IEEE's.
#[cfg(feature = "controls")]
fn metal_atan2(y: f32, x: f32) -> f32 {
    let t = (y / x).atan();
    match (x < 0.0, y >= 0.0) {
        (true, true) => t + std::f32::consts::PI,
        (true, false) => t - std::f32::consts::PI,
        _ => t,
    }
}

#[test]
fn canonicaliser_phi_signed_zero() {
    let h = harness();
    let states = signed_zero_rho();
    let narrowed: Vec<Ic<f32>> = states.iter().map(|(_, _, x)| narrow(x)).collect();
    let gpu_out = gpu(&h, &narrowed);
    let mut worst = 0.0f64;
    for (k, (name, sign, x)) in states.iter().enumerate() {
        // The case is reached: step 2's `ρ_x`, formed as `C` forms it, is the zero the name gives, its sign included.
        let n = &narrowed[k];
        let rho_x = n.r[1][0] - n.r[0][0];
        assert_eq!(
            (rho_x == 0.0, rho_x.is_sign_negative()),
            (true, name.starts_with("(−0")),
            "ρ_x = {rho_x:e} at ρ = {name}"
        );
        check_quarter(canonicalise(x).phi, *sign, f64::EPSILON, name);
        check_quarter(canonicalise(n).phi.into(), *sign, EPS32, name);
        worst = worst.max(check_quarter(gpu_out[k].phi.into(), *sign, EPS32, name));
    }
    let info = h.adapter_info();
    eprintln!(
        "canonicaliser_phi_signed_zero: worst GPU-f32 φ {worst:.2} f32 eps of π/2 on {} ({:?})",
        info.name, info.backend
    );
}

negative_control!(
    canonicaliser_phi_signed_zero,
    "φ from Metal's atan2 with ρ_x's zero sign kept, which turns φ by π at ρ_x = −0, must fail",
    expected = "not +π/2",
    {
        for (name, sign, x) in signed_zero_rho() {
            let n = narrow(&x);
            let rho = [n.r[1][0] - n.r[0][0], n.r[1][1] - n.r[0][1]];
            check_quarter(metal_atan2(rho[1], rho[0]).into(), sign, EPS32, name);
        }
    }
);

// ── canonicaliser_noop_gpu (REQ-DEC-018) ────────────────────────────────────────────────────────────────────────

/// The GPU no-op's tolerance, per [`state_error`], in f32 eps.
const NOOP_GPU_EPS: f64 = 4.0;

/// The latent chart at the point `z`: `z₀ = z`, `q₁ = q₂ = 0`.
fn at(z: [f32; 8]) -> Latent<f32> {
    Latent {
        z0: z,
        q1: [0.0; 8],
        q2: [0.0; 8],
    }
}

/// Fuzzed interiors, each control alone at `±30` and `±1000`, and `(z_α, z_β)` together at each pair of those.
fn noop_batches() -> Vec<Batch> {
    let mut b = interior(Fuzz {
        seed: 23,
        charts: 16,
        per_chart: 64,
    });
    let mut rng = SplitMix::new(29);
    let sat = [-1000.0f32, -30.0, 30.0, 1000.0];
    for k in 0..8 {
        for s in sat {
            let mut z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
            z[k] = s;
            b.push((at(z), vec![[0.5, 0.5]]));
        }
    }
    for a in sat {
        for c in sat {
            let mut z: [f32; 8] = core::array::from_fn(|_| rng.uniform(-2.0, 2.0));
            z[0] = a;
            z[1] = c;
            b.push((at(z), vec![[0.5, 0.5]]));
        }
    }
    b
}

/// Test 5 on one GPU decode `x` and its GPU `C`: no mirror, `φ = 0`, the state unchanged within [`NOOP_GPU_EPS`]; the
/// error.
fn check_noop_gpu(x: &Ic<f32>, c: &Canonical<f32>, at: &str) -> f64 {
    assert!(!c.mirrored, "the mirror was taken at {at}");
    assert!(
        c.phi == 0.0,
        "the rotation angle is {}, not 0, at {at}",
        c.phi
    );
    let err = state_error(&widen(x), &widen(&c.ic), EPS32);
    assert!(
        err <= NOOP_GPU_EPS,
        "C moved the decode by {err} eps at {at}"
    );
    err
}

#[test]
fn canonicaliser_noop_gpu() {
    let h = harness();
    let stage = DecodeStage::<LatentDefaults>::new(h.device(), &wgsl(), FastMath::default())
        .expect("the decode stage's pipeline");
    let decodes: Vec<Ic<f32>> = noop_batches()
        .iter()
        .flat_map(|(chart, uv)| {
            stage
                .run(h.device(), h.queue(), chart, DecodeMode::Full, uv)
                .expect("the decode dispatch")
        })
        .map(|o| o.ic)
        .collect();
    let outs = gpu(&h, &decodes);
    let (mut worst, mut seam, mut clamped) = (0.0f64, 0, 0);
    for (k, (x, c)) in decodes.iter().zip(&outs).enumerate() {
        worst = worst.max(check_noop_gpu(x, c, &format!("GPU decode {k}")));
        seam += usize::from(c.lambda_t_y.abs() <= f32::DELTA_LAMBDA);
        clamped += usize::from(c.clamped);
    }
    assert!(seam > 0, "no GPU decode reached the λ̃_y = 0 seam");
    eprintln!(
        "canonicaliser_noop_gpu: {} decodes; worst state change {worst} eps (tolerance {NOOP_GPU_EPS}); {seam} at the \
         seam; {clamped} at ρ = 0",
        decodes.len()
    );
}

negative_control!(
    canonicaliser_noop_gpu,
    "a GPU output whose mirror flag is set must fail test 5",
    expected = "the mirror was taken",
    {
        let h = harness();
        let x = narrow(&representatives()[0]);
        let mut c = gpu(&h, &[x])[0];
        c.mirrored = true;
        check_noop_gpu(&x, &c, "a mirrored output");
    }
);
