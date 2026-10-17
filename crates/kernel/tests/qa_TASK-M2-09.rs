//! QA tests for TASK-M2-09, written from the requirements (chart_reference §2.1, §2.2, §5.2; dd_decoder §3.4, §5 tests
//! 8 and 12; R-82; payload §2), not from the implementation. Every check runs at the f64 and the f32 instantiation of
//! the one source, natively (R-265).
//! - REQ-DEC-020 (`qa_m209_targets_*`): over random `(u, v)` through §2.1's warp, and over a grid of `(L_z, K*)` at
//!   fixed decoded configurations with unequal masses, `K* → K_min` included: `Σpᵢ = 0`, `L_z(p) = L_z` and
//!   `K(p) = K*`. The residuals are formed in double-double from the construction's own output, so they measure the
//!   construction, not the check; the tolerances are REQ-DEC-046's proposal (below). `qa_m209_follows_section_2_2`
//!   compares the momenta with an independent f64 transcription of §2.2 that recomputes `M` and `I`: the three
//!   invariants alone hold for any unit direction free of drift and spin, so they cannot see which seed was taken or
//!   whether its direction is §2.2's; this test carries the direction, and its control is a transcription taking the
//!   smallest qualifying seed. (A crossed `m₀ ↔ m₁` in the seed map's `ρ̇` coefficients is invisible after the CoM
//!   projection, which restores `v₁ − v₀ = ρ̇` either way, so it is not a control here.)
//! - REQ-DEC-021 (`qa_m209_seed_*`): the seed rule on given norms, and through the construction on decoded and
//!   hand-built configurations: the largest norm wins, not the first; an exact tie keeps the earlier seed; `ρ = 0`
//!   selects a fallback, not `DEGENERATE`; `DEGENERATE` only where no norm exceeds `ε_w`, `ε_w` itself not qualifying.
//! - REQ-DEC-022, REQ-DEC-040 (`qa_m209_infeasible_*`): `K* < L_z²/(2I)` decided exactly, against references that are
//!   exact by construction (an f32 triple's products are exact in f64; an f64 square is split error-free by `fma`), at
//!   both precisions for identical inputs, which is the CPU half of dd_decoder test 12's branch identity; a pixel on
//!   the infeasible side is `Infeasible`, labelled `decode_failed` (payload §2's state 5) with detail 3, its momenta
//!   finite and the rigid rotation, and it takes precedence over `DEGENERATE`.
//!
//! **Tolerances** (REQ-DEC-046, proposed per R-71 by TASK-M2-09 and pending the human at the M2 gate; not picked here):
//! `|Σpᵢ| ≤ 4·eps·Σ|pᵢ|`, `|L_z(p) − L_z| ≤ 4·eps·(|L_z| + √(2K*))`, `|K(p) − K*| ≤ 8·eps·K*`, `eps` the
//! instantiation's machine epsilon. `qa_m209_follows_section_2_2` holds `‖p − p_§2.2‖` to the `L_z` tolerance's scale,
//! the momenta's own size, `4·eps·(|L_z| + √(2K*))` per component (the same proposal).
//!
//! **Domain** (pitfalls §10): the stateless construction, inputs held fixed; no trajectory. The GPU half of the branch
//! identity is `engine`'s `invariant_gpu`, which runs only in the `gpu-kernel` profile's filter.
//!
//! Each test has a registered negative control naming the panic it expects (R-176, R-212).

use std::fmt::Debug;

use kernel::decode::config::{self, Config};
use kernel::decode::invariant::{
    choose, construct, construct_by_norms, infeasible, infeasible_label, Construction, ExactSign,
    Outcome, Seed,
};
use kernel::decode::mass::{self, Masses};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::STATE_DECODE_FAILED;
use proptest::prelude::*;
use validation::decode_f32_eps::SplitMix;
use validation::negative_control;
use validation::prop;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// An instantiation under test.
trait Q: ExactSign + LinkReal + Copy + Debug {
    /// Machine epsilon.
    const EPS: f64;
    /// The name in messages.
    const LABEL: &'static str;
    /// Widened to f64, exactly.
    fn f(self) -> f64;
    /// Rounded from f64.
    fn of(x: f64) -> Self;
    /// The next float down.
    fn down(self) -> Self;
    /// The next float up.
    fn up(self) -> Self;
}

impl Q for f64 {
    const EPS: f64 = f64::EPSILON;
    const LABEL: &'static str = "f64";
    fn f(self) -> f64 {
        self
    }
    fn of(x: f64) -> Self {
        x
    }
    fn down(self) -> Self {
        self.next_down()
    }
    fn up(self) -> Self {
        self.next_up()
    }
}

impl Q for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const LABEL: &'static str = "f32";
    fn f(self) -> f64 {
        f64::from(self)
    }
    fn of(x: f64) -> Self {
        x as f32
    }
    fn down(self) -> Self {
        self.next_down()
    }
    fn up(self) -> Self {
        self.next_up()
    }
}

/// REQ-DEC-046's proposed tolerances, in units of `eps` times the quantity's scale (the module's note).
const TOL_P: f64 = 4.0;
const TOL_L: f64 = 4.0;
const TOL_K: f64 = 8.0;

// ── Double-double residuals ─────────────────────────────────────────────────────────────────────────────────────

/// An unevaluated sum `hi + lo`, enough to form a residual of f64 inputs without the check's own rounding mattering.
#[derive(Clone, Copy)]
struct Dd(f64, f64);

impl Dd {
    fn zero() -> Self {
        Dd(0.0, 0.0)
    }
    fn add(self, x: f64) -> Self {
        let s = self.0 + x;
        let bb = s - self.0;
        let err = (self.0 - (s - bb)) + (x - bb);
        let lo = self.1 + err;
        let hi = s + lo;
        Dd(hi, lo - (hi - s))
    }
    fn add_prod(self, a: f64, b: f64) -> Self {
        let p = a * b;
        let e = a.mul_add(b, -p);
        self.add(p).add(e)
    }
    fn div(self, m: f64) -> Self {
        let q1 = self.0 / m;
        let r = (-q1).mul_add(m, self.0) + self.1;
        Dd(q1, 0.0).add(r / m)
    }
    fn value(self) -> f64 {
        self.0 + self.1
    }
}

/// The residuals of a construction, each in units of `eps` times its scale: `(Σp, L_z, K)`.
fn residuals<R: Q>(m: [R; 3], r: [[R; 2]; 3], p: [[R; 2]; 3], lz: R, k: R) -> (f64, f64, f64) {
    let (lz, k) = (lz.f(), k.f());
    let mut sx = Dd::zero();
    let mut sy = Dd::zero();
    let mut l = Dd::zero().add(-lz);
    let mut two_k = Dd::zero().add(-2.0 * k);
    let mut size = 0.0;
    for i in 0..3 {
        let (px, py) = (p[i][0].f(), p[i][1].f());
        let (rx, ry) = (r[i][0].f(), r[i][1].f());
        sx = sx.add(px);
        sy = sy.add(py);
        size += px.abs() + py.abs();
        l = l.add_prod(rx, py).add_prod(-ry, px);
        two_k = two_k.add(
            Dd::zero()
                .add_prod(px, px)
                .add_prod(py, py)
                .div(m[i].f())
                .value(),
        );
    }
    let sum = sx.value().abs().max(sy.value().abs());
    let rp = if size == 0.0 {
        sum
    } else {
        sum / (R::EPS * size)
    };
    let l_scale = lz.abs() + (2.0 * k).max(0.0).sqrt();
    let rl = if l_scale == 0.0 {
        l.value().abs()
    } else {
        l.value().abs() / (R::EPS * l_scale)
    };
    let rk = if k == 0.0 {
        two_k.value().abs()
    } else {
        (two_k.value() / 2.0).abs() / (R::EPS * k)
    };
    (rp, rl, rk)
}

/// REQ-DEC-020's check on one construction: `Σp = 0`, `L_z(p) = L_z`, `K(p) = K*`, to REQ-DEC-046's proposal.
fn check_targets<R: Q>(
    masses: &Masses<R>,
    cfg: &Config<R>,
    c: &Construction<R>,
    lz: R,
    k: R,
    at: &str,
) {
    assert_eq!(
        c.outcome,
        Outcome::Feasible,
        "{at}: a feasible pair is not Feasible"
    );
    let (rp, rl, rk) = residuals(masses.m, cfg.r, c.p, lz, k);
    assert!(rp <= TOL_P, "{at}: Σp is {rp} eps of Σ|p|");
    assert!(rl <= TOL_L, "{at}: L_z(p) misses L_z by {rl} eps");
    assert!(rk <= TOL_K, "{at}: K(p) misses K* by {rk} eps");
}

// ── Inputs ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// The masses and configuration of the latent `[z_μ₁, z_μ₂, z_α, z_β]` at `R`, with the configuration control `z_α`
/// the decoder's construction reads its seed from.
fn decoded<R: Q>(z: [f64; 4]) -> (Masses<R>, Config<R>, R) {
    let masses = mass::decode::<Defaults, R>([R::of(z[0]), R::of(z[1])]);
    let za = R::of(z[2]);
    let cfg = config::decode::<Defaults, R>([za, R::of(z[3])], &masses);
    (masses, cfg, za)
}

/// Fixed latents with unequal masses (PIT-9) and spread shapes.
const LATENTS: [[f64; 4]; 6] = [
    [0.0, 0.0, 0.0, 0.0],
    [1.3, -0.7, 0.4, -1.1],
    [-2.0, 1.5, 2.5, 0.3],
    [0.9, 2.2, -2.5, 1.7],
    [-1.4, -1.9, 1.0, -2.6],
    [3.0, -3.0, -0.6, 3.1],
];

/// §2.1's warp at `I = 1`: `K = K_max·v^γ`, `L_max = √(2K)`, `L_z = (2u − 1)·L_max`, as f64 `(L_z, K*)`.
fn warp(u: f64, v: f64, k_max: f64, gamma: f64) -> (f64, f64) {
    let k = k_max * v.powf(gamma);
    ((2.0 * u - 1.0) * (2.0 * k).sqrt(), k)
}

// ── Exact references for the feasibility branch ─────────────────────────────────────────────────────────────────

/// `2·i·k < l²` exactly, for finite f32 inputs: each product of two f32 has at most 48 significant bits and an
/// exponent within f64's normal range, so every operation below is exact in f64.
fn exact_f32(i: f32, k: f32, l: f32) -> bool {
    let (i, k, l) = (f64::from(i), f64::from(k), f64::from(l));
    2.0 * (i * k) < l * l
}

/// `2k < l²` exactly, for finite f64 `k` and `l` with `|l| ≥ 2⁻⁴⁸⁰` and `|2k|` finite: `l² = h + e` exactly (`fma`), and
/// `2k − h` is exact where `2k` and `h` lie within a factor 2 (Sterbenz); outside that `|2k − h| > h/2 > |e|`.
fn exact_f64_unit_i(k: f64, l: f64) -> bool {
    let h = l * l;
    let e = l.mul_add(l, -h);
    let k2 = 2.0 * k;
    if k2 > 2.0 * h || k2 < 0.5 * h {
        k2 < h
    } else {
        k2 - h < e
    }
}

/// The feasibility check: `infeasible` at f32 and at f64 for the same f32 triple, against the exact reference.
fn check_branch_f32(i: f32, k: f32, l: f32) {
    let want = exact_f32(i, k, l);
    assert_eq!(
        infeasible::<f32>(i, k, l),
        want,
        "f32 branch is not 2·I·K* < L_z² at (I, K*, L_z) = ({i:e}, {k:e}, {l:e})"
    );
    assert_eq!(
        infeasible::<f64>(f64::from(i), f64::from(k), f64::from(l)),
        want,
        "f64 branch is not 2·I·K* < L_z² at (I, K*, L_z) = ({i:e}, {k:e}, {l:e})"
    );
}

fn check_branch_f64_unit(k: f64, l: f64) {
    let want = exact_f64_unit_i(k, l);
    assert_eq!(
        infeasible::<f64>(1.0, k, l),
        want,
        "f64 branch is not 2·K* < L_z² at (K*, L_z) = ({k:e}, {l:e})"
    );
}

/// A finite f32 drawn over its whole range by bits, subnormals and both signs included.
fn any_finite_f32(g: &mut SplitMix) -> f32 {
    loop {
        let x = f32::from_bits(g.next_u64() as u32);
        if x.is_finite() {
            return x;
        }
    }
}

/// `(I, K*, L_z)` triples at f32 on and around the boundary `K* = L_z²/(2I)`, at every scale, plus whole-range draws.
fn f32_triples(n: usize) -> Vec<[f32; 3]> {
    let mut g = SplitMix::new(20932);
    let mut out = Vec::new();
    // Exact boundary points: short significands whose L_z²/2 and products are exact at f32.
    for &l in &[
        0.0f32,
        1.0,
        -1.0,
        3.0,
        0.5,
        -1.5,
        2.0f32.powi(-70),
        2.0f32.powi(60),
        2.0f32.powi(-75),
    ] {
        for &i in &[1.0f32, 2.0, 0.25, 0.75] {
            let k = l * l / (2.0 * i);
            for kk in [k, k.down(), k.up(), -k, 0.0, -0.0] {
                out.push([i, kk, l]);
            }
        }
    }
    while out.len() < n {
        let i = match g.next_u64() % 3 {
            0 => 1.0,
            1 => f32::from_bits(0x3F00_0000 + (g.next_u64() as u32 & 0x00FF_FFFF)),
            _ => any_finite_f32(&mut g).abs(),
        };
        let l = any_finite_f32(&mut g);
        if g.next_u64().is_multiple_of(4) {
            out.push([i, any_finite_f32(&mut g), l]);
            continue;
        }
        let k0 = (f64::from(l) * f64::from(l) / (2.0 * f64::from(i))) as f32;
        if !k0.is_finite() || i == 0.0 {
            continue;
        }
        let shift = (g.next_u64() % 9) as i32 - 4;
        let mut k = k0;
        for _ in 0..shift.abs() {
            k = if shift > 0 { k.up() } else { k.down() };
        }
        if k.is_finite() {
            out.push([i, k, l]);
        }
    }
    out
}

// ── REQ-DEC-020: the targets ────────────────────────────────────────────────────────────────────────────────────

/// One property case at `R`: latent `z`, warp inputs `(u, v, K_max, γ)`. Infeasible pairs (the warp's rounding at
/// `u ∈ {0, 1}`) must be exactly the reference's, and then tagged; every other pair meets its targets.
fn property_case<R: Q>(z: [f64; 4], u: f64, v: f64, k_max: f64, gamma: f64) {
    let (masses, cfg, za) = decoded::<R>(z);
    let (lz, k) = warp(u, v, k_max, gamma);
    let (lz, k) = (R::of(lz), R::of(k));
    let c = construct(&masses, &cfg, za, lz, k);
    let at = format!(
        "{} z={z:?} (u,v)=({u},{v}) K_max={k_max} γ={gamma}",
        R::LABEL
    );
    if exact_f64_unit_i(k.f(), lz.f()) {
        assert_eq!(
            c.outcome,
            Outcome::Infeasible,
            "{at}: an infeasible pair is not tagged"
        );
    } else {
        check_targets(&masses, &cfg, &c, lz, k, &at);
    }
}

fn targets_strategy() -> impl Strategy<Value = ([f64; 4], f64, f64, f64, f64)> {
    (
        proptest::array::uniform4(-4.0f64..4.0),
        prop_oneof![Just(0.0f64), Just(1.0f64), Just(0.5f64), 0.0f64..=1.0],
        prop_oneof![Just(1.0f64), Just(0.0f64), 0.0f64..=1.0],
        -6.0f64..6.0,
        prop_oneof![Just(1.0f64), Just(2.0f64), 1.0f64..4.0],
    )
        .prop_map(|(z, u, v, lk, g)| (z, u, v, 10f64.powf(lk), g))
}

/// REQ-DEC-020 over random `(u, v)` (property test, `prop::CASES` cases, fixed printed seed).
#[test]
fn qa_m209_targets_property() {
    prop::run(&targets_strategy(), |(z, u, v, k_max, g)| {
        property_case::<f64>(z, u, v, k_max, g);
        property_case::<f32>(z, u, v, k_max, g);
        Ok(())
    });
}

negative_control!(
    qa_m209_targets_property,
    "a construction aimed at K*·(1 + 2⁻¹⁰) must miss K* — the check on the real construction's output",
    expected = "K(p) misses K*",
    {
        let (masses, cfg, za) = decoded::<f64>(LATENTS[1]);
        let (lz, k) = warp(0.3, 0.7, 2.0, 1.0);
        let c = construct(&masses, &cfg, za, lz, k * (1.0 + 2f64.powi(-10)));
        check_targets(&masses, &cfg, &c, lz, k, "control");
    }
);

/// The grid of `(L_z, K*)` at one configuration: `K* = K_min·(1 + δ)`, `δ` from 0 (where exact) through the last ulps
/// to 10³, and `L_z = 0` with `K* ≥ 0`.
fn grid_at<R: Q>(z: [f64; 4]) -> usize {
    let (masses, cfg, za) = decoded::<R>(z);
    let mut n = 0;
    let lzs = [
        -2.5, -1.0, -0.375, -1e-3, 0.0, 1e-6, 0.125, 0.75, 1.0, 1.5, 3.0,
    ];
    let deltas = [
        0.0, 1e-15, 1e-12, 1e-9, 1e-7, 1e-5, 1e-3, 1e-1, 1.0, 10.0, 1e3,
    ];
    for &lz in &lzs {
        let lz = R::of(lz);
        let kmin = lz.f() * lz.f() / 2.0;
        for &d in &deltas {
            let mut k = R::of(kmin * (1.0 + d));
            // Step up to the first feasible K*, so `δ = 0` lands on K_min or one ulp above it: the near edge.
            while exact_f64_unit_i(k.f(), lz.f()) {
                k = k.up();
            }
            // At L_z = 0 the edge is K* = 0 itself; the ulps above it are subnormal, where a relative residual has no
            // scale, so the edge is checked alone there (K* > 0 at L_z = 0 is the list below).
            let edge = if kmin == 0.0 {
                vec![k]
            } else {
                vec![k, k.up(), k.up().up()]
            };
            for kk in edge {
                let c = construct(&masses, &cfg, za, lz, kk);
                let at = format!("{} z={z:?} L_z={lz:?} K*={kk:?}", R::LABEL);
                check_targets(&masses, &cfg, &c, lz, kk, &at);
                n += 1;
            }
        }
    }
    for &k in &[0.0, 1e-30, 1e-3, 1.0, 1e4] {
        let (lz, k) = (R::of(0.0), R::of(k));
        let c = construct(&masses, &cfg, za, lz, k);
        check_targets(
            &masses,
            &cfg,
            &c,
            lz,
            k,
            &format!("{} z={z:?} L_z=0 K*={k:?}", R::LABEL),
        );
        n += 1;
    }
    n
}

/// REQ-DEC-020 on a grid of feasible `(L_z, K*)`, near `K* → K_min` included, at fixed configurations.
#[test]
fn qa_m209_targets_grid() {
    let mut n = 0;
    for z in LATENTS {
        n += grid_at::<f64>(z);
        n += grid_at::<f32>(z);
    }
    eprintln!("qa_m209_targets_grid: {n} (configuration, L_z, K*) samples checked");
    assert!(n > 1000, "the grid is smaller than designed: {n}");
}

negative_control!(
    qa_m209_targets_grid,
    "a momentum set carrying a CoM drift of 1e-9 must fail Σp = 0",
    expected = "Σp is",
    {
        let (masses, cfg, za) = decoded::<f64>(LATENTS[2]);
        let mut c = construct(&masses, &cfg, za, 1.0, 0.75);
        c.p[2][0] += 1e-9;
        check_targets(&masses, &cfg, &c, 1.0, 0.75, "control");
    }
);

// ── REQ-DEC-020: §2.2 exactly ──────────────────────────────────────────────────────────────────────────────────

/// Seed velocities by §2.2's map.
fn seed_velocities(m: [f64; 3], rho_dot: [f64; 2], lam_dot: [f64; 2]) -> [[f64; 2]; 3] {
    let big_m = m[0] + m[1] + m[2];
    let m01 = m[0] + m[1];
    let (c0, c1) = (m[1] / m01, m[0] / m01);
    let d = m[2] / big_m;
    [
        [
            -d * lam_dot[0] - c0 * rho_dot[0],
            -d * lam_dot[1] - c0 * rho_dot[1],
        ],
        [
            -d * lam_dot[0] + c1 * rho_dot[0],
            -d * lam_dot[1] + c1 * rho_dot[1],
        ],
        [m01 / big_m * lam_dot[0], m01 / big_m * lam_dot[1]],
    ]
}

/// §2.2 transcribed at f64 from the doc, `M` and `I` recomputed from the masses and positions: `(p, seed index or 4,
/// norms)`. `smallest` takes the smallest qualifying seed instead of the largest (the control's wrong rule).
fn section_2_2(
    m: [f64; 3],
    r: [[f64; 2]; 3],
    rho: [f64; 2],
    lam: [f64; 2],
    lz: f64,
    k: f64,
    smallest: bool,
) -> ([[f64; 2]; 3], usize, [f64; 4]) {
    let jv = |v: [f64; 2]| [-v[1], v[0]];
    let big_m = m[0] + m[1] + m[2];
    let inertia: f64 = (0..3)
        .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
        .sum();
    let zero = [0.0, 0.0];
    let family = [(rho, zero), (zero, lam), (jv(rho), zero), (zero, jv(lam))];
    let mut ws = [[[0.0; 2]; 3]; 4];
    let mut norms = [0.0; 4];
    for (s, &(rd, ld)) in family.iter().enumerate() {
        let v = seed_velocities(m, rd, ld);
        let c = [
            (0..3).map(|i| m[i] * v[i][0]).sum::<f64>(),
            (0..3).map(|i| m[i] * v[i][1]).sum::<f64>(),
        ];
        let w1: Vec<[f64; 2]> = (0..3)
            .map(|i| [v[i][0] - c[0] / big_m, v[i][1] - c[1] / big_m])
            .collect();
        let l: f64 = (0..3)
            .map(|i| m[i] * (r[i][0] * w1[i][1] - r[i][1] * w1[i][0]))
            .sum();
        let beta = l / inertia;
        for i in 0..3 {
            let jr = jv(r[i]);
            ws[s][i] = [w1[i][0] - beta * jr[0], w1[i][1] - beta * jr[1]];
        }
        norms[s] = (0..3)
            .map(|i| m[i] * (ws[s][i][0].powi(2) + ws[s][i][1].powi(2)))
            .sum();
    }
    let mut best = 4;
    for s in 0..4 {
        if norms[s] > 1e-10 && (best == 4 || (norms[s] > norms[best]) != smallest) {
            best = s;
        }
    }
    let omega = lz / inertia;
    let kmin = lz * lz / (2.0 * inertia);
    let a = if best == 4 {
        0.0
    } else {
        (2.0 * (k - kmin)).max(0.0).sqrt()
    };
    let n = if best == 4 { 1.0 } else { norms[best].sqrt() };
    let w = if best == 4 { [[0.0; 2]; 3] } else { ws[best] };
    let mut p = [[0.0; 2]; 3];
    for i in 0..3 {
        let jr = jv(r[i]);
        for x in 0..2 {
            p[i][x] = m[i] * (omega * jr[x] + a * w[i][x] / n);
        }
    }
    (p, best, norms)
}

/// The check: at f64, on a decoded configuration and a feasible pair, the construction's seed is §2.2's (away from a
/// near tie between the top two norms, `64·eps` relative), and its momenta are §2.2's to the L_z tolerance's scale.
fn check_section_2_2(z: [f64; 4], lz: f64, k: f64, smallest: bool) -> bool {
    let (masses, cfg, za) = decoded::<f64>(z);
    let c = construct(&masses, &cfg, za, lz, k);
    let (p, best, norms) = section_2_2(masses.m, cfg.r, cfg.rho, cfg.lambda, lz, k, smallest);
    let mut sorted = norms;
    sorted.sort_by(|a, b| b.total_cmp(a));
    if sorted[0] - sorted[1] <= 64.0 * f64::EPSILON * sorted[0] {
        return false;
    }
    let seed_index = c.seed.map_or(4, |s| s as usize);
    assert_eq!(
        seed_index, best,
        "z={z:?}: the construction's seed differs from §2.2's"
    );
    let tol = TOL_L * f64::EPSILON * (lz.abs() + (2.0 * k).sqrt());
    for (i, (got, want)) in c.p.iter().zip(&p).enumerate() {
        for (x, (g, w)) in got.iter().zip(want).enumerate() {
            let d = (g - w).abs();
            assert!(d <= tol, "z={z:?} L_z={lz} K*={k}: p[{i}][{x}] differs from §2.2's by {d:e} (tolerance {tol:e})");
        }
    }
    true
}

/// REQ-DEC-020: the construction is §2.2's, seed map, projection, normalisation and mixing included.
#[test]
fn qa_m209_follows_section_2_2() {
    let mut g = SplitMix::new(20922);
    let mut n = 0;
    for _ in 0..2000 {
        let mut z = [0.0; 4];
        for zi in &mut z {
            *zi = (g.next_u64() as f64 / u64::MAX as f64) * 8.0 - 4.0;
        }
        let u = g.next_u64() as f64 / u64::MAX as f64;
        let v = g.next_u64() as f64 / u64::MAX as f64;
        let (lz, k) = warp(u.clamp(0.01, 0.99), v.max(1e-3), 3.0, 1.0);
        n += usize::from(check_section_2_2(z, lz, k, false));
    }
    assert!(n > 1500, "too few cases away from a near tie: {n}");
}

negative_control!(
    qa_m209_follows_section_2_2,
    "a §2.2 transcription taking the smallest qualifying seed must differ from the construction",
    expected = "differs from §2.2's",
    {
        // Unequal masses, a shape away from the α = π/4 tie: the smallest and the largest seed differ.
        check_section_2_2([1.3, -0.7, 0.4, -1.1], 0.4, 1.0, true);
    }
);

// ── REQ-DEC-021: the seed rule ─────────────────────────────────────────────────────────────────────────────────

const SEEDS: [Seed; 4] = [Seed::Rho, Seed::Lambda, Seed::JRho, Seed::JLambda];

/// The seed rule at `R` on given norms against the rule as R-82 states it.
fn check_choose<R: Q>(norms: [f64; 4], want: Option<Seed>) {
    let n = [
        R::of(norms[0]),
        R::of(norms[1]),
        R::of(norms[2]),
        R::of(norms[3]),
    ];
    assert_eq!(
        choose(n),
        want,
        "{}: the seed rule on norms {norms:?}",
        R::LABEL
    );
}

/// REQ-DEC-021: several qualify, the largest wins, wherever it sits; not the first.
#[test]
fn qa_m209_seed_largest() {
    let cases: [([f64; 4], Seed); 6] = [
        ([0.3, 0.9, 0.5, 0.1], Seed::Lambda),
        ([0.3, 0.2, 0.4, 0.35], Seed::JRho),
        ([0.1, 0.2, 0.3, 0.4], Seed::JLambda),
        ([0.4, 0.3, 0.2, 0.1], Seed::Rho),
        ([1e-9, 0.0, 2e-9, 1.5e-9], Seed::JRho),
        ([0.5, 0.25, 0.75, 0.5], Seed::JRho),
    ];
    for (norms, want) in cases {
        check_choose::<f64>(norms, Some(want));
        check_choose::<f32>(norms, Some(want));
    }
    // Through the construction: decoded configurations where several seeds qualify and the largest is not the first.
    let mut later = 0;
    for z in LATENTS
        .iter()
        .chain(&[[0.2, -0.4, 3.0, 0.5], [-1.0, 0.8, 1.2, 2.0]])
    {
        let (_, best, norms) = {
            let (masses, cfg, _) = decoded::<f64>(*z);
            section_2_2(masses.m, cfg.r, cfg.rho, cfg.lambda, 0.0, 1.0, false)
        };
        let mut sorted = norms;
        sorted.sort_by(|a, b| b.total_cmp(a));
        // Away from a near tie the largest is §2.2's argmax; at the exact tie (`z_α = 0`, α = π/4) R-82 keeps the
        // earlier seed, (ρ, 0).
        let want = if z[2] == 0.0 {
            Some(0)
        } else if sorted[0] - sorted[1] > 64.0 * f64::EPSILON * sorted[0] {
            Some(best)
        } else {
            None
        };
        let qualifying = norms.iter().filter(|&&x| x > 1e-10).count();
        if want.is_some_and(|b| b > 0) && qualifying > 1 {
            later += 1;
        }
        if let Some(want) = want {
            let (masses, cfg, za) = decoded::<f64>(*z);
            let c = construct(&masses, &cfg, za, 0.0, 1.0);
            assert_eq!(
                c.seed.map_or(4, |s| s as usize),
                want,
                "z={z:?}: not the largest qualifying seed"
            );
            let (m32, c32, za32) = decoded::<f32>(*z);
            let c = construct(&m32, &c32, za32, 0.0f32, 1.0f32);
            assert_eq!(
                c.seed.map_or(4, |s| s as usize),
                want,
                "f32 z={z:?}: not the largest qualifying seed"
            );
        }
    }
    assert!(
        later > 0,
        "no fixture where several seeds qualify and the largest is not the first"
    );
}

negative_control!(
    qa_m209_seed_largest,
    "claiming the first qualifying seed where a later one is larger must fail",
    expected = "the seed rule on norms",
    check_choose::<f64>([0.3, 0.9, 0.5, 0.1], Some(Seed::Rho))
);

/// REQ-DEC-021: an exact tie keeps the earlier seed, at every pair of positions.
#[test]
fn qa_m209_seed_tie() {
    for a in 0..4 {
        for b in a + 1..4 {
            let mut norms = [0.125; 4];
            norms[a] = 0.5;
            norms[b] = 0.5;
            check_choose::<f64>(norms, Some(SEEDS[a]));
            check_choose::<f32>(norms, Some(SEEDS[a]));
        }
    }
    check_choose::<f64>([0.25; 4], Some(Seed::Rho));
    check_choose::<f32>([0.25; 4], Some(Seed::Rho));
}

negative_control!(
    qa_m209_seed_tie,
    "claiming the later of two tied seeds must fail",
    expected = "the seed rule on norms",
    check_choose::<f64>([0.125, 0.5, 0.125, 0.5], Some(Seed::JLambda))
);

/// A hand-built configuration of unit `I` from `ρ̃` and `λ̃` with the masses of latent `z`, at `R`.
fn built<R: Q>(z: [f64; 2], rho_t: [f64; 2], lam_t: [f64; 2]) -> (Masses<R>, Config<R>) {
    let masses = mass::decode::<Defaults, R>([R::of(z[0]), R::of(z[1])]);
    let rt = [R::of(rho_t[0]), R::of(rho_t[1])];
    let lt = [R::of(lam_t[0]), R::of(lam_t[1])];
    let (rho, lambda, r) = config::reconstruct(&masses, rt, lt);
    let zero = R::of(0.0);
    let cfg = Config {
        alpha: zero,
        beta: zero,
        rho_t: rt,
        lambda_t: lt,
        rho,
        lambda,
        r,
    };
    (masses, cfg)
}

/// The fallback check at `R`: with `ρ = 0` the primary does not qualify, a fallback does, and the pixel meets its
/// targets, not `DEGENERATE`.
fn check_fallback<R: Q>(masses: &Masses<R>, cfg: &Config<R>) {
    for (lz, k) in [(0.0, 1.0), (0.5, 0.5), (-1.0, 3.0)] {
        let (lz, k) = (R::of(lz), R::of(k));
        let c = construct_by_norms(masses, cfg, lz, k);
        assert!(
            c.norms[0].f() <= R::EPS_W.f(),
            "{}: the primary seed still qualifies",
            R::LABEL
        );
        assert!(
            matches!(c.seed, Some(s) if s != Seed::Rho),
            "{}: ρ = 0 did not select a fallback: {:?}",
            R::LABEL,
            c.seed
        );
        check_targets(
            masses,
            cfg,
            &c,
            lz,
            k,
            &format!("{} ρ = 0 fallback", R::LABEL),
        );
    }
}

/// REQ-DEC-021 / §5.2: forcing the primary seed to degenerate (`ρ = 0`) selects a fallback, not `DEGENERATE`.
#[test]
fn qa_m209_seed_fallback() {
    for z in [[0.0, 0.0], [1.3, -0.7], [-2.0, 1.5]] {
        for lam in [[1.0, 0.0], [0.0, 1.0], [0.6, 0.8], [-0.28, 0.96]] {
            let (m, c) = built::<f64>(z, [0.0, 0.0], lam);
            check_fallback(&m, &c);
            let (m, c) = built::<f32>(z, [0.0, 0.0], lam);
            check_fallback(&m, &c);
        }
    }
}

negative_control!(
    qa_m209_seed_fallback,
    "a configuration with ρ ≠ 0 must fail the check that the primary does not qualify (the check sees ρ)",
    expected = "the primary seed still qualifies",
    {
        let (m, c) = built::<f64>([1.3, -0.7], [0.6, 0.0], [0.0, 0.8]);
        check_fallback(&m, &c);
    }
);

/// The `DEGENERATE` check at `R`: no norm above `ε_w` (each at most `ε_w`, `ε_w` itself included) gives none; any one
/// norm one float above `ε_w` gives that seed.
fn check_degenerate_rule<R: Q>(eps_w: R) {
    let e = eps_w.f();
    let below = [0.0, e, R::of(e).down().f(), e * 0.5];
    for a in below {
        for b in below {
            let norms = [a, b, b, a];
            check_choose::<R>(norms, None);
        }
    }
    let above = R::of(e).up().f();
    for s in 0..4 {
        let mut norms = [e; 4];
        norms[s] = above;
        check_choose::<R>(norms, Some(SEEDS[s]));
    }
}

/// REQ-DEC-021: `DEGENERATE` fires only where no seed has `‖w⁽²⁾‖²_m > ε_w`; through the construction, only on a
/// configuration whose every seed vanishes (all bodies at the CoM), never on a decoded configuration.
#[test]
fn qa_m209_seed_degenerate() {
    check_degenerate_rule::<f64>(f64::EPS_W);
    check_degenerate_rule::<f32>(f32::EPS_W);
    assert_eq!(
        f64::EPS_W,
        1e-10,
        "ε_w is chart_reference §2.2's default 1e−10"
    );
    for z in [[0.0, 0.0], [1.3, -0.7]] {
        let (m, c) = built::<f64>(z, [0.0, 0.0], [0.0, 0.0]);
        let d = construct_by_norms(&m, &c, 0.0, 0.5);
        assert_eq!(
            d.outcome,
            Outcome::Degenerate,
            "all bodies at the CoM: not DEGENERATE"
        );
        assert_eq!(d.seed, None, "all bodies at the CoM: a seed was chosen");
        assert!(
            d.p.iter().flatten().all(|x| x.is_finite()),
            "a DEGENERATE pixel's momenta are not finite"
        );
        let (m, c) = built::<f32>(z, [0.0, 0.0], [0.0, 0.0]);
        let d = construct_by_norms(&m, &c, 0.0f32, 0.5);
        assert_eq!(
            d.outcome,
            Outcome::Degenerate,
            "f32 all bodies at the CoM: not DEGENERATE"
        );
        assert!(
            d.p.iter().flatten().all(|x| x.is_finite()),
            "f32 DEGENERATE momenta not finite"
        );
    }
    let mut g = SplitMix::new(20990);
    for _ in 0..2000 {
        let mut z = [0.0; 4];
        for zi in &mut z {
            *zi = (g.next_u64() as f64 / u64::MAX as f64) * 16.0 - 8.0;
        }
        let (m, c, za) = decoded::<f64>(z);
        assert_ne!(
            construct(&m, &c, za, 0.25, 1.0).outcome,
            Outcome::Degenerate,
            "z={z:?}: DEGENERATE with a seed qualifying"
        );
        let (m, c, za) = decoded::<f32>(z);
        assert_ne!(
            construct(&m, &c, za, 0.25f32, 1.0).outcome,
            Outcome::Degenerate,
            "f32 z={z:?}: DEGENERATE with a seed qualifying"
        );
    }
}

negative_control!(
    qa_m209_seed_degenerate,
    "a rule that let ε_w itself qualify would pick a seed where none is above ε_w — claimed here, it must fail",
    expected = "the seed rule on norms",
    check_choose::<f64>([1e-10, 0.0, 0.0, 0.0], Some(Seed::Rho))
);

/// The seed-identity check: the f32 construction at latent `z32` and the f64 one at `z64` take the same seed.
fn check_seed_same(z32: [f64; 4], z64: [f64; 4]) {
    let (m32, c32, za32) = decoded::<f32>(z32);
    let (m64, c64, za64) = decoded::<f64>(z64);
    let s32 = construct(&m32, &c32, za32, 0.25f32, 1.0).seed;
    let s64 = construct(&m64, &c64, za64, 0.25f64, 1.0).seed;
    assert_eq!(
        s32, s64,
        "the seed forks between f32 (z={z32:?}) and f64 (z={z64:?})"
    );
}

/// REQ-DEC-021 with dd_decoder §5 test 12 (branch tags bit-identical across backends) and R-82's tie rule: the seed is
/// a branch decision, so identical latents take one seed at f32 and at f64; at `z_α = 0`, `α = π/4` under the
/// default α-link (chart_reference §0.2), the first two seeds' norms `cos²α` and `sin²α` tie exactly, and the rule
/// takes `(ρ, 0)` at both precisions.
#[test]
fn qa_m209_seed_backend_identity() {
    let mut g = SplitMix::new(20955);
    let unit = |g: &mut SplitMix| g.next_u64() as f64 / u64::MAX as f64;
    for _ in 0..20_000 {
        let z = [
            unit(&mut g) * 8.0 - 4.0,
            unit(&mut g) * 8.0 - 4.0,
            unit(&mut g) * 8.0 - 4.0,
            unit(&mut g) * 8.0 - 4.0,
        ];
        check_seed_same(z, z);
    }
    for _ in 0..2_000 {
        // Around the tie: z_α within a few f32 ulps of 0, z_α = 0 itself included.
        let za = [0.0, 1e-9, -1e-9, 1e-7, -1e-7, 1e-6, -1e-6][(g.next_u64() % 7) as usize];
        let z = [
            unit(&mut g) * 8.0 - 4.0,
            unit(&mut g) * 8.0 - 4.0,
            za,
            unit(&mut g) * 8.0 - 4.0,
        ];
        check_seed_same(z, z);
        // Off the tie the rule is the larger of cos²α and sin²α: α increases through π/4 at z_α = 0 under the
        // default link (chart_reference §0.2, α = α_min + (π/2 − 2α_min)·σ(z_α)), so z_α < 0 is (ρ, 0), z_α > 0 (0, λ).
        if za != 0.0 {
            let want = if za < 0.0 { Seed::Rho } else { Seed::Lambda };
            let (m, c, zf) = decoded::<f64>(z);
            assert_eq!(
                construct(&m, &c, zf, 0.25, 1.0).seed,
                Some(want),
                "z={z:?}: not the larger of cos²α, sin²α at f64"
            );
            let (m, c, zf) = decoded::<f32>(z);
            assert_eq!(
                construct(&m, &c, zf, 0.25f32, 1.0).seed,
                Some(want),
                "z={z:?}: not the larger of cos²α, sin²α at f32"
            );
        }
        if za == 0.0 {
            let (m, c, za) = decoded::<f64>(z);
            assert_eq!(
                construct(&m, &c, za, 0.25, 1.0).seed,
                Some(Seed::Rho),
                "z={z:?}: the α = π/4 tie is not (ρ, 0) at f64"
            );
            let (m, c, za) = decoded::<f32>(z);
            assert_eq!(
                construct(&m, &c, za, 0.25f32, 1.0).seed,
                Some(Seed::Rho),
                "z={z:?}: the α = π/4 tie is not (ρ, 0) at f32"
            );
        }
    }
}

negative_control!(
    qa_m209_seed_backend_identity,
    "latents on opposite sides of the α = π/4 tie must take different seeds: the check sees a fork",
    expected = "the seed forks between f32",
    check_seed_same([0.3, -0.2, -1.5, 0.7], [0.3, -0.2, 1.5, 0.7])
);

// ── REQ-DEC-022, REQ-DEC-040: the infeasible pixel ─────────────────────────────────────────────────────────────

/// REQ-DEC-022: `K* < L_z²/(2I)` exactly, f32 and f64 taking one branch for identical inputs (dd_decoder test 12, CPU).
#[test]
fn qa_m209_infeasible_branch_exact() {
    let triples = f32_triples(200_000);
    let mut sides = [0usize; 2];
    for [i, k, l] in &triples {
        check_branch_f32(*i, *k, *l);
        sides[usize::from(exact_f32(*i, *k, *l))] += 1;
    }
    assert!(
        sides[0] > 10_000 && sides[1] > 10_000,
        "one side of the boundary is barely sampled: {sides:?}"
    );
    // f64, `I = 1`, full significands, around the boundary at every scale.
    let mut g = SplitMix::new(20964);
    let mut n = 0;
    while n < 200_000 {
        let l = f64::from_bits(g.next_u64() & 0x7FFF_FFFF_FFFF_FFFF);
        let l = if g.next_u64().is_multiple_of(2) {
            l
        } else {
            -l
        };
        if !l.is_finite() || l.abs() < 2f64.powi(-480) || l.abs() > 2f64.powi(500) {
            continue;
        }
        let mut k = l * l / 2.0;
        let shift = (g.next_u64() % 9) as i32 - 4;
        for _ in 0..shift.abs() {
            k = if shift > 0 {
                k.next_up()
            } else {
                k.next_down()
            };
        }
        check_branch_f64_unit(k, l);
        n += 1;
    }
}

negative_control!(
    qa_m209_infeasible_branch_exact,
    "a boundary triple whose K* = L_z²/2 is claimed infeasible must fail: the boundary itself is feasible",
    expected = "branch is not 2·I·K* < L_z²",
    {
        // A wrong-side input: one ulp below the boundary, whose reference is infeasible, passed with the boundary's
        // own claim (feasible) — asserting the f32 branch on it against the boundary's answer.
        let (i, l) = (1.0f32, 3.0f32);
        let k = 4.5f32.next_down();
        assert_eq!(
            infeasible::<f32>(i, k, l),
            exact_f32(i, 4.5, l),
            "f32 branch is not 2·I·K* < L_z² at (I, K*, L_z) = ({i:e}, {k:e}, {l:e})"
        );
    }
);

/// The tag check at `R` on one pair: infeasible exactly on `K* < L_z²/2`, then labelled, finite and the rigid rotation.
fn check_tag<R: Q>(masses: &Masses<R>, cfg: &Config<R>, za: R, lz: R, k: R) {
    let c = construct(masses, cfg, za, lz, k);
    let at = format!("{} L_z={lz:?} K*={k:?}", R::LABEL);
    assert!(
        c.p.iter().flatten().all(|x| x.f().is_finite()),
        "{at}: momenta not finite"
    );
    let short = exact_f64_unit_i(k.f(), lz.f());
    assert_eq!(
        c.outcome == Outcome::Infeasible,
        short,
        "{at}: tagged infeasible off K* < L_z²/2"
    );
    if short {
        for i in 0..3 {
            let jr = [-cfg.r[i][1], cfg.r[i][0]];
            for (px, jx) in c.p[i].iter().zip(jr) {
                let rigid = masses.m[i] * (lz * jx);
                assert!(
                    *px == rigid,
                    "{at}: infeasible momenta are not the rigid rotation m·L_z·J r"
                );
            }
        }
    }
}

/// REQ-DEC-022, REQ-DEC-040: a pixel with `K* < K_min` is `Infeasible` exactly there, labelled `decode_failed` (5)
/// with detail 3, never NaN, its momenta finite; `K* = K_min` is feasible with `a = 0`; f32 and f64 agree on the tag.
#[test]
fn qa_m209_infeasible_tagged() {
    assert_eq!(
        infeasible_label(),
        (STATE_DECODE_FAILED, 3),
        "the infeasible label is not decode_failed/3"
    );
    assert_eq!(
        STATE_DECODE_FAILED, 5,
        "payload §2: decode_failed is state 5"
    );
    for z in LATENTS {
        let (m64, c64, za64) = decoded::<f64>(z);
        let (m32, c32, za32) = decoded::<f32>(z);
        for l in [
            0.0f32,
            1.0,
            -1.0,
            3.0,
            -0.5,
            1.5,
            2.0f32.powi(-60),
            1e-3,
            0.1,
            -7.3,
        ] {
            let kmin = (f64::from(l) * f64::from(l) / 2.0) as f32;
            for k in [
                kmin,
                kmin.down(),
                kmin.down().down(),
                kmin.up(),
                -1.0,
                -0.0,
                0.0,
                kmin * 0.5,
                kmin * 2.0,
            ] {
                check_tag(&m32, &c32, za32, l, k);
                check_tag(&m64, &c64, za64, f64::from(l), f64::from(k));
                let o32 = construct(&m32, &c32, za32, l, k).outcome == Outcome::Infeasible;
                let o64 = construct(&m64, &c64, za64, f64::from(l), f64::from(k)).outcome
                    == Outcome::Infeasible;
                assert_eq!(
                    o32, o64,
                    "z={z:?} L_z={l:e} K*={k:e}: f32 and f64 tag differently"
                );
            }
            // Where K_min is exact, K* = K_min is feasible with a = 0.
            if f64::from(kmin) == f64::from(l) * f64::from(l) / 2.0 {
                let c = construct(&m64, &c64, za64, f64::from(l), f64::from(kmin));
                assert_eq!(
                    c.outcome,
                    Outcome::Feasible,
                    "z={z:?} L_z={l:e}: K* = K_min is not feasible"
                );
                assert_eq!(c.a, 0.0, "z={z:?} L_z={l:e}: a ≠ 0 at K* = K_min");
            }
        }
        // f64 one ulp either side of an exactly representable K_min = 0.5.
        check_tag(&m64, &c64, za64, 1.0, 0.5f64.next_down());
        check_tag(&m64, &c64, za64, 1.0, 0.5);
    }
}

negative_control!(
    qa_m209_infeasible_tagged,
    "a pair one ulp below K_min checked against a configuration-free claim of feasibility must fail",
    expected = "tagged infeasible off K* < L_z²/2",
    {
        // The check run with the reference's sides swapped: the pair below K_min is fed as if it were above.
        let (m, c, za) = decoded::<f64>(LATENTS[1]);
        let got = construct(&m, &c, za, 1.0, 0.5f64.next_down()).outcome == Outcome::Infeasible;
        assert_eq!(got, exact_f64_unit_i(0.5, 1.0), "f64 L_z=1 K*=0.5⁻: tagged infeasible off K* < L_z²/2");
    }
);
