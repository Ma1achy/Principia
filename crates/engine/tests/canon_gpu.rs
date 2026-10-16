//! Canonicalisation `C` on the GPU (`kernel::stage::canon`, `engine::decode_dispatch::canonicalise_gpu`; TASK-M2-07): the
//! kernel's f32 build, `target/spirv/kernel.wgsl`, dispatched natively, read back and checked against the same source on
//! the CPU at f64 and f32. CI runs this file in the `gpu-kernel` job, after `cargo xtask build-kernel`, and on Metal.
//! - `canonicaliser_mirror_tie`: states whose steps 1a, 0, 1b and 2 are exact no-ops, so `λ̃_y` is exactly `±δ_λ` and
//!   its neighbours at each precision: no mirror for `|λ̃_y| ≤ δ_λ`, a mirror just below `−δ_λ`, the same output twice,
//!   and the same branch on CPU-f64, CPU-f32 and GPU-f32; the test reads `λ̃_y`, not `λ_y` (REQ-DEC-017).
//! - `canonicaliser_canonicalise_order`: an off-CoM, boosted, scaled, rotated state, mirrored or not: `I` is the
//!   CoM-frame one, and the output is the canonical representative on both backends (REQ-ENC-015).
//! - `canonicaliser_coincident_inner_pair`: `ρ = 0` raises `lookup_clamped` with `φ = +0` and the identity rotation on
//!   CPU-f64 and GPU-f32, `ρ = (±0, ±0)` included; a `ρ` one step off 0 does not (REQ-ENC-016, R-13, R-437).
//! - `canonicaliser_noop_gpu`: dd_decoder §5 test 5 on the GPU, `C` on the GPU's own canonical-frame decodes, saturated
//!   controls included, takes no mirror, turns by 0 and moves nothing beyond rounding (REQ-DEC-018).
//!
//! **Domain** (pitfalls §10): `C` alone, on states held fixed; no trajectory. **PIT-10:** the cross-backend branch claim
//! is for identical inputs; where the backends' `λ̃_y` differ, so may their branches.

use std::path::{Path, PathBuf};

use engine::contract::fast_math::FastMath;
use engine::decode_dispatch::{canonicalise_gpu, DecodeStage};
use kernel::canon::{canonicalise, inertia, Canonical};
use kernel::chart::Latent;
use kernel::decode::{decode, Ic};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, LinkReal};
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

/// A state with masses `(¼, ¼, ½)`, `r₀ = (−d, a)`, `r₁ = (d, a)`, `r₂ = (0, −a)`, at rest: its CoM is exactly 0, `ρ`
/// is on `+x` with `ρ_y = 0`, and `λ_y = −2a`, so `λ̃_y = √(m₂M₀₁)·λ_y = ½·(−2a) = −a`, each exact. `d` makes `I` round
/// to a value whose square root is 1 ([`unit_d`]), so step 0 is the identity too.
fn tie<R: LinkReal>(a: R, d: R) -> Ic<R> {
    let (q, h, z) = (R::from(0.25).unwrap(), R::from(0.5).unwrap(), R::zero());
    Ic {
        m: [q, q, h],
        r: [[-d, a], [d, a], [z, -a]],
        p: [[z; 2]; 3],
    }
}

/// The `d` nearest `√2` for which [`tie`]'s state has `√I = 1` exactly at the precision of `R`, found by stepping
/// from `√2` one representable value at a time.
fn unit_d<R: LinkReal>(up: fn(R) -> R, down: fn(R) -> R) -> R {
    let a = R::DELTA_LAMBDA;
    let start = R::from(2f64.sqrt()).unwrap();
    let (mut lo, mut hi) = (start, start);
    for _ in 0..64 {
        for d in [lo, hi] {
            let c = canonicalise(&tie(a, d));
            if c.i_in.sqrt() == R::one() {
                return d;
            }
        }
        lo = down(lo);
        hi = up(hi);
    }
    panic!("no d near √2 gives √I = 1 at {}", R::NAME)
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
        assert!(
            !c.clamped,
            "lookup_clamped at {at}, where ρ is on +x and non-zero"
        );
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
    let d64 = unit_d::<f64>(f64::next_up, f64::next_down);
    let cases64 = tie_cases::<f64>(f64::next_up, f64::next_down);
    let outs64: Vec<Canonical<f64>> = cases64
        .iter()
        .map(|(a, _)| canonicalise(&tie(*a, d64)))
        .collect();
    check_tie(&cases64, &outs64, "CPU-f64");
    for ((a, _), c) in cases64.iter().zip(&outs64) {
        assert_eq!(
            canonicalise(&tie(*a, d64)),
            *c,
            "CPU-f64 gave two results at a = {a:e}"
        );
    }
    // CPU-f32 and GPU-f32, on identical inputs.
    let d32 = unit_d::<f32>(f32::next_up, f32::next_down);
    let cases32 = tie_cases::<f32>(f32::next_up, f32::next_down);
    let states: Vec<Ic<f32>> = cases32.iter().map(|(a, _)| tie(*a, d32)).collect();
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
        "canonicaliser_mirror_tie: d = {d64:e} (f64), {d32:e} (f32); δ_λ = {:e} (f64), {:e} (f32)",
        f64::DELTA_LAMBDA,
        f32::DELTA_LAMBDA
    );
    // Turned, translated and boosted ties: λ̃_y within rounding of the deadband's edges. Each backend gives the same
    // output twice, and its mirror is its own λ̃_y's test; CPU-f32 and GPU-f32 take the same branch where they read the
    // same λ̃_y (PIT-10).
    let turns: Vec<Ic<f64>> = cases64
        .iter()
        .flat_map(|(a, _)| [0.3, 1.9, -2.6].map(|t| turned(&tie(*a, 2f64.sqrt()), t)))
        .collect();
    let narrow_turns: Vec<Ic<f32>> = turns.iter().map(narrow).collect();
    let (first, second) = (gpu(&h, &narrow_turns), gpu(&h, &narrow_turns));
    let mut same = 0;
    for (k, x) in turns.iter().enumerate() {
        let c64 = canonicalise(x);
        assert_eq!(canonicalise(x), c64, "CPU-f64 gave two results at turn {k}");
        assert_eq!(c64.mirrored, c64.lambda_t_y < -f64::DELTA_LAMBDA);
        let c32 = canonicalise(&narrow_turns[k]);
        assert_eq!(
            bits(&first[k]),
            bits(&second[k]),
            "GPU-f32 gave two results at turn {k}"
        );
        assert_eq!(first[k].mirrored, first[k].lambda_t_y < -f32::DELTA_LAMBDA);
        if first[k].lambda_t_y.to_bits() == c32.lambda_t_y.to_bits() {
            same += 1;
            assert_eq!(
                first[k].mirrored, c32.mirrored,
                "the backends' branches differ at turn {k}"
            );
        }
    }
    eprintln!(
        "canonicaliser_mirror_tie: {same} of {} turned ties read the same λ̃_y on CPU-f32 and GPU-f32",
        turns.len()
    );
}

negative_control!(
    canonicaliser_mirror_tie,
    "a mirror test reading λ_y = λ̃_y/√μ_λ, the defect R-82 names, mirrors at λ̃_y = −δ_λ and must fail",
    expected = "the mirror is true at CPU-f64, λ̃_y = −(1e-12)",
    {
        let d = unit_d::<f64>(f64::next_up, f64::next_down);
        let cases = tie_cases::<f64>(f64::next_up, f64::next_down);
        let outs: Vec<Canonical<f64>> = cases
            .iter()
            .map(|(a, _)| {
                let mut c = canonicalise(&tie(*a, d));
                // λ_y = λ̃_y/√(m₂M₀₁) = 2λ̃_y here.
                c.mirrored = 2.0 * c.lambda_t_y < -f64::DELTA_LAMBDA;
                c
            })
            .collect();
        check_tie(&cases, &outs, "CPU-f64")
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
