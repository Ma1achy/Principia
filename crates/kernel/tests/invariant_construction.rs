//! The invariant-momentum construction (`kernel::decode::invariant`; chart_reference §2.2; dd_decoder §3.4, §5 test 8;
//! REQ-DEC-020, REQ-DEC-021, REQ-DEC-022, REQ-DEC-046), each check at the f64 and the f32 instantiation natively (R-265):
//! - `invariant_construction_targets`: over random `(u, v)` through §2.1's warp and a grid of `(L_z, K*)`, near
//!   `K* → K_min` included, at fixed decoded configurations with unequal masses (PIT-9): `Σpᵢ = 0`, `L_z(p) = L_z` and
//!   `K(p) = K*`, within the proposed tolerances ([`TOL_P`], [`TOL_L`], [`TOL_K`]; REQ-DEC-046), whose worst residuals
//!   it prints (decoder test 8; chart_reference §5.2).
//! - `invariant_construction_feasibility_exact`: `infeasible(I, K*, L_z)` is `2·I·K* < L_z²` in exact arithmetic, against
//!   an integer reference, over near-boundary triples at every scale, subnormals included, sparse-bit triples whose
//!   difference lands in each limb column, the exponent cases `E = −4 … 4`, zeros and signs; an f32 triple widened to
//!   f64 takes the same branch (REQ-DEC-022).
//! - `invariant_construction_infeasible_tagged`: a pair with `K* < L_z²/2` is `Infeasible`, exactly on that inequality
//!   (`K* = L_z²/2` is feasible with `a = 0`, one ulp below is not), labelled `decode_failed`/3, never dropped or NaN,
//!   its momenta the rigid rotation; it takes precedence over `DEGENERATE` (REQ-DEC-022, REQ-DEC-040).
//! - `invariant_construction_binade`: each operand's sign, zero flag and `s·2^e` read from its bits, exactly,
//!   subnormals included, and the limbs of `s`, each on its grid, summing to `s`.
//! - `invariant_construction_entries`: the two compute entry points, called natively, write what the construction gives.
//! - `seed_selection_largest`: where several seeds qualify the largest-norm one is taken, not the first, on a dyadic
//!   fixture and on decoded configurations against an independent transcription of §2.2 (REQ-DEC-021).
//! - `seed_selection_tie`: an exact tie of norms keeps the earlier seed (REQ-DEC-021).
//! - `seed_selection_fallback`: with the primary seed degenerate (`ρ = 0`) a fallback is taken, not `DEGENERATE`.
//! - `seed_selection_degenerate`: `DEGENERATE` fires only where no seed has `‖w⁽²⁾‖²_m > ε_w`, `ε_w` itself not
//!   qualifying and the next float above it qualifying, at every seed position.
//!
//! **The tolerances** (REQ-DEC-046, proposed per R-71, for the human to confirm at the M2 gate): each residual is in
//! units of the precision's `eps` times its scale: `|Σpᵢ|` of `Σ|pᵢ|`, `|L_z(p) − L_z|` of `|L_z| + √(2K*)` (the rigid
//! part and the mixed part, `|L(w)| ≤ √I‖w‖_m = 1`), and `|K(p) − K*|` of `K*`. The residuals are formed in f64 from the
//! construction's output, so at f32 they measure the f32 construction alone.
//!
//! **Domain** (pitfalls §10): the construction alone, its inputs held fixed; no trajectory.

mod decode_support;

use decode_support::*;
use kernel::decode::config::Config;
use kernel::decode::invariant::{
    self, choose, construct, infeasible, infeasible_label, Binade, Construction, ExactSign,
    Outcome, Seed,
};
use kernel::decode::mass::{self, Masses};
use kernel::decode::{self as dec, Latent};
use kernel::generated::links::*;
use kernel::payload::STATE_DECODE_FAILED;
use spirv_std::glam::UVec3;
use validation::decode_f32_eps::SplitMix;
use validation::negative_control;

/// `|Σpᵢ|` in units of `eps·Σ|pᵢ|`, at most (REQ-DEC-046, proposed).
const TOL_P: f64 = 4.0;
/// `|L_z(p) − L_z|` in units of `eps·(|L_z| + √(2K*))`, at most (REQ-DEC-046, proposed).
const TOL_L: f64 = 4.0;
/// `|K(p) − K*|` in units of `eps·K*`, at most (REQ-DEC-046, proposed).
const TOL_K: f64 = 8.0;

// ── Precisions and the exact reference ──────────────────────────────────────────────────────────────────────────

/// A precision under test, with its bit layout.
trait Q: P + ExactSign {
    /// The fraction's width.
    const FRAC: u32;
    /// The exponent field's bias.
    const BIAS: i64;
    /// The float of the given bits.
    fn raw(bits: u64) -> Self;
}

impl Q for f64 {
    const FRAC: u32 = 52;
    const BIAS: i64 = 1023;
    fn raw(bits: u64) -> Self {
        f64::from_bits(bits)
    }
}

impl Q for f32 {
    const FRAC: u32 = 23;
    const BIAS: i64 = 127;
    fn raw(bits: u64) -> Self {
        f32::from_bits(bits as u32)
    }
}

/// The positive float of exponent field `field` and fraction `fraction`.
fn make<R: Q>(field: i64, fraction: u64) -> R {
    R::raw(((field as u64) << R::FRAC) | (fraction & ((1u64 << R::FRAC) - 1)))
}

/// `x` moved `n` representable values along the positive axis (`x > 0` finite, the result too).
fn step<R: Q>(x: R, n: i64) -> R {
    R::raw((x.bits() as i64 + n) as u64)
}

/// `±mant·2^exp` of a finite f64, exactly.
fn parts(x: f64) -> (bool, u128, i64) {
    let b = x.to_bits();
    let field = ((b >> 52) & 0x7FF) as i64;
    let fraction = (b & ((1 << 52) - 1)) as u128;
    let neg = b >> 63 != 0;
    if field == 0 {
        (neg, fraction, -1074)
    } else {
        (neg, fraction | (1 << 52), field - 1075)
    }
}

/// `2·i·k < l²` in exact (integer) arithmetic, for finite `i`, `k`, `l`.
fn reference(i: f64, k: f64, l: f64) -> bool {
    let (ni, mi, ei) = parts(i);
    let (nk, mk, ek) = parts(k);
    let (_, ml, el) = parts(l);
    if mi == 0 || mk == 0 {
        return ml != 0;
    }
    if ni != nk {
        return true;
    }
    let (a, ea) = (mi * mk, ei + ek + 1);
    let (b, eb) = (ml * ml, 2 * el);
    if b == 0 {
        return false;
    }
    let len = |x: u128| 128 - i64::from(x.leading_zeros());
    let (ta, tb) = (len(a) + ea, len(b) + eb);
    if ta != tb {
        return ta < tb;
    }
    if ea >= eb {
        (a << (ea - eb)) < b
    } else {
        a < (b << (eb - ea))
    }
}

// ── The triples ─────────────────────────────────────────────────────────────────────────────────────────────────

/// A positive float with exponent field in `lo..=hi` and a random fraction.
fn rand_pos<R: Q>(g: &mut SplitMix, lo: i64, hi: i64) -> R {
    let field = lo + (g.next_u64() % (hi - lo + 1) as u64) as i64;
    make(field, g.next_u64())
}

/// A fraction with at most three bits set.
fn sparse(g: &mut SplitMix, frac: u32) -> u64 {
    (0..g.next_u64() % 4).fold(0, |f, _| f | 1 << (g.next_u64() % u64::from(frac)))
}

/// `K* ≈ L_z²/(2I)` at precision `R`, stepped up to 3 values either way: triples on both sides of the boundary at every
/// scale, `I` near 1 and anywhere, subnormal `K*`, `L_z` and `I` among them.
fn near_boundary<R: Q>(g: &mut SplitMix, n: usize) -> Vec<(R, R, R)> {
    let top = 2 * R::BIAS;
    let mut out = Vec::new();
    while out.len() < n {
        let i: R = match g.next_u64() % 4 {
            0 => R::one(),
            1 => rand_pos(g, R::BIAS - 3, R::BIAS + 3),
            2 => rand_pos(g, 0, 3),
            _ => rand_pos(g, 1, top),
        };
        let l: R = rand_pos(g, 0, top);
        let k0 = R::of(l.f() * l.f() / (2.0 * i.f()));
        if !(k0.f().is_finite() && k0.f() > 0.0) {
            continue;
        }
        let k = step(k0, (g.next_u64() % 7) as i64 - 3);
        if k.f().is_finite() && k.f() > 0.0 {
            out.push((i, k, l));
        }
    }
    out
}

/// Sparse-bit significands around 1, so `2IK* − L_z²` is often exactly 0 or lands in one limb column.
fn sparse_triples<R: Q>(g: &mut SplitMix, n: usize) -> Vec<(R, R, R)> {
    (0..n)
        .map(|_| {
            let (ei, ek) = ((g.next_u64() % 3) as i64 - 1, (g.next_u64() % 3) as i64 - 2);
            let i = make::<R>(R::BIAS + ei, sparse(g, R::FRAC));
            let k = make::<R>(R::BIAS + ek, sparse(g, R::FRAC));
            (i, k, make::<R>(R::BIAS, sparse(g, R::FRAC)))
        })
        .collect()
}

/// `L_z = 1 + 2^−j`, `I = 1`, `K* = ½ + 2^−j` and its neighbours: `2IK* − L_z² = −2^−2j`, the exact difference in
/// each column `j` reaches, and one value up.
fn column_triples<R: Q>() -> Vec<(R, R, R)> {
    (1..=i64::from(R::FRAC))
        .flat_map(|j| {
            let l = make::<R>(R::BIAS, 1 << (i64::from(R::FRAC) - j));
            let k = R::of(0.5 + (2f64).powi(-(j as i32)));
            [
                (R::one(), k, l),
                (R::one(), step(k, 1), l),
                (R::one(), step(k, -1), l),
            ]
        })
        .collect()
}

/// `E = e_I + e_K + 1 − 2e_L` from −4 to 4 at the extreme significands `1`, `1 + ulp`, `1.5`, `2 − ulp`.
fn exponent_triples<R: Q>() -> Vec<(R, R, R)> {
    let top = (1u64 << R::FRAC) - 1;
    let sigs = [0, 1, 1 << (R::FRAC - 1), top];
    let mut out = Vec::new();
    for e in -4i64..=4 {
        for &a in &sigs {
            for &b in &sigs {
                for &c in &sigs {
                    out.push((
                        make::<R>(R::BIAS, a),
                        make::<R>(R::BIAS + e - 1, b),
                        make::<R>(R::BIAS, c),
                    ));
                }
            }
        }
    }
    out
}

/// Zeros, signs, the smallest subnormal and the largest finite value, in every combination.
fn sign_triples<R: Q>() -> Vec<(R, R, R)> {
    let tiny = R::raw(1);
    let big = make::<R>(2 * R::BIAS, u64::MAX);
    let vals = [
        R::zero(),
        -R::zero(),
        tiny,
        -tiny,
        R::one(),
        -R::one(),
        big,
        -big,
    ];
    let mut out = Vec::new();
    for &i in &vals {
        for &k in &vals {
            for &l in &vals {
                out.push((i, k, l));
            }
        }
    }
    out
}

/// Every family of triples.
fn triples<R: Q>(seed: u64) -> Vec<(R, R, R)> {
    let mut g = SplitMix::new(seed);
    let mut t = near_boundary::<R>(&mut g, 40_000);
    t.extend(sparse_triples::<R>(&mut g, 40_000));
    t.extend(column_triples::<R>());
    t.extend(exponent_triples::<R>());
    t.extend(sign_triples::<R>());
    t
}

/// A feasibility test under check: `(I, K*, L_z) → infeasible`.
type Test<R> = fn(R, R, R) -> bool;

/// `test` agrees with the exact reference on every triple; returns how many were infeasible.
fn check_exact<R: Q>(ts: &[(R, R, R)], test: Test<R>) -> usize {
    let mut short = 0;
    for &(i, k, l) in ts {
        let want = reference(i.f(), k.f(), l.f());
        assert_eq!(
            test(i, k, l),
            want,
            "the test at (I, K*, L_z) = ({:e}, {:e}, {:e}) ({}) differs from the exact 2IK* < L_z² ({want})",
            i.f(),
            k.f(),
            l.f(),
            R::NAME
        );
        short += usize::from(want);
    }
    short
}

#[test]
fn invariant_construction_feasibility_exact() {
    let t64 = triples::<f64>(64);
    let n64 = check_exact(&t64, infeasible::<f64>);
    let t32 = triples::<f32>(32);
    let n32 = check_exact(&t32, infeasible::<f32>);
    for &(i, k, l) in &t32 {
        assert_eq!(
            infeasible(i as f64, k as f64, l as f64),
            infeasible(i, k, l),
            "f64 and f32 take different branches at ({i:e}, {k:e}, {l:e})"
        );
    }
    assert!(
        n64 > t64.len() / 4 && n64 < 3 * t64.len() / 4,
        "f64: {n64} of {} infeasible",
        t64.len()
    );
    assert!(
        n32 > t32.len() / 4 && n32 < 3 * t32.len() / 4,
        "f32: {n32} of {} infeasible",
        t32.len()
    );
}

negative_control!(
    invariant_construction_feasibility_exact,
    "the rounded comparison K* < L_z·L_z/(2I) at f32 must fail",
    expected = "differs from the exact",
    {
        check_exact(&triples::<f32>(32), |i, k, l| k < l * l / (2.0 * i));
    }
);

// ── Configurations ──────────────────────────────────────────────────────────────────────────────────────────────

/// A configuration from its masses, `ρ` and `λ`: `r₀₁ = −m₂λ`, `r₀ = r₀₁ − (m₁/M₀₁)ρ`, `r₁ = r₀₁ + (m₀/M₀₁)ρ`,
/// `r₂ = M₀₁λ` (chart_reference §0.2), the angles unused.
fn config_of<R: Q>(masses: &Masses<R>, rho: [R; 2], lambda: [R; 2]) -> Config<R> {
    let [m0, m1, m2] = masses.m;
    let m01 = masses.m01;
    let r01 = [-(m2 * lambda[0]), -(m2 * lambda[1])];
    let (w0, w1) = (m1 / m01, m0 / m01);
    Config {
        alpha: R::zero(),
        beta: R::zero(),
        rho_t: [R::zero(); 2],
        lambda_t: [R::zero(); 2],
        rho,
        lambda,
        r: [
            [r01[0] - w0 * rho[0], r01[1] - w0 * rho[1]],
            [r01[0] + w1 * rho[0], r01[1] + w1 * rho[1]],
            [m01 * lambda[0], m01 * lambda[1]],
        ],
    }
}

/// A dyadic fixture: masses, `ρ`, `λ`, every product exact and `I = 1` exactly.
fn dyadic<R: Q>(m: [f64; 3], rho: [f64; 2], lambda: [f64; 2]) -> (Masses<R>, Config<R>) {
    let masses = mass::from_masses(m.map(R::of));
    let config = config_of(&masses, rho.map(R::of), lambda.map(R::of));
    (masses, config)
}

/// The decoded configurations at fixed controls: the golden IC's `z = 0`, then unequal masses under the defaults
/// (PIT-9) over the configuration controls' range, a tight inner pair and a wide one among them.
fn decoded<R: Q>() -> Vec<(Masses<R>, Config<R>)> {
    let zs: [[f64; 4]; 10] = [
        [0.0; 4],
        [0.3, -0.7, 0.8, -0.4],
        [-1.2, 1.9, -0.6, 1.1],
        [2.5, 0.2, 1.5, 0.3],
        [-2.0, -2.5, -1.3, -0.9],
        [0.9, 3.0, 0.4, 2.0],
        [5.0, -1.0, 0.2, -3.0],
        [-5.0, 4.0, -2.5, 0.7],
        [1.0, -6.0, 3.0, 3.0],
        [-3.5, 0.5, -3.0, -3.0],
    ];
    zs.iter()
        .map(|z| {
            let full = [z[0], z[1], 0.0, 0.0, 0.0, 0.0, z[2], z[3]];
            let d = dec::decode::<Defaults, R>(&Latent::new(full.map(R::of)));
            (d.masses, d.config)
        })
        .collect()
}

// ── invariant_construction_targets ──────────────────────────────────────────────────────────────────────────────

/// A construction under check.
type Construct<R> = fn(&Masses<R>, &Config<R>, R, R) -> Construction<R>;

/// The `(L_z, K*)` pairs: §2.1's warp `K = K_max·t^γ`, `L_z = (2s − 1)·√(2IK)` at random `(s, t)` and at `s` near 0 and
/// 1, and a grid of `K*` and `L_z/√(2K*)`, both edges of the parabola included.
fn pairs(seed: u64) -> Vec<(f64, f64)> {
    let mut g = SplitMix::new(seed);
    let mut unit = || (g.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    let mut out = Vec::new();
    for (k_max, gamma) in [(1.0f64, 1.0), (0.5, 2.5), (40.0, 1.5)] {
        for _ in 0..1000 {
            let (s, t) = (unit(), unit());
            let k = k_max * f64::powf(t, gamma);
            out.push(((2.0 * s - 1.0) * (2.0 * k).sqrt(), k));
        }
        for s in [0.0, 1e-9, 1e-5, 1.0 - 1e-5, 1.0 - 1e-9, 1.0] {
            let k = k_max * f64::powf(unit(), gamma);
            out.push(((2.0 * s - 1.0) * (2.0 * k).sqrt(), k));
        }
    }
    for k in [0.0f64, 1e-6, 1e-3, 0.1, 1.0, 10.0, 1e3] {
        for f in [0.0, 0.25, 0.5, 0.9, 0.999, 1.0 - 2e-7, 1.0] {
            out.push((f * (2.0 * k).sqrt(), k));
            out.push((-f * (2.0 * k).sqrt(), k));
        }
    }
    out
}

/// The residuals of one construction, each in its unit (the module's note): `(Σp, L_z, K)`.
fn residuals<R: Q>(
    masses: &Masses<R>,
    config: &Config<R>,
    lz: R,
    k: R,
    c: &Construction<R>,
) -> [f64; 3] {
    let m = masses.m.map(|x| x.f());
    let r = config.r.map(|v| v.map(|x| x.f()));
    let p = c.p.map(|v| v.map(|x| x.f()));
    let (lz, k) = (lz.f(), k.f());
    let sum = [p[0][0] + p[1][0] + p[2][0], p[0][1] + p[1][1] + p[2][1]];
    let size: f64 = p.iter().map(|v| v[0].abs() + v[1].abs()).sum();
    let l: f64 = (0..3).map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0]).sum();
    let kin: f64 = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum();
    let eps = R::eps();
    let over = |d: f64, scale: f64| if d == 0.0 { 0.0 } else { d / (eps * scale) };
    [
        over(sum[0].abs() + sum[1].abs(), size),
        over((l - lz).abs(), lz.abs() + (2.0 * k).sqrt()),
        over((kin - k).abs(), k),
    ]
}

/// Every pair at every decoded configuration: the outcome is the exact test's, and a feasible construction meets its
/// targets within the tolerances.
fn check_targets<R: Q>(build: Construct<R>, worst: &Worst) {
    for (masses, config) in decoded::<R>() {
        for (lz, k) in pairs(9) {
            let (lz, k) = (R::of(lz), R::of(k));
            let c = build(&masses, &config, lz, k);
            let feasible = !reference(1.0, k.f(), lz.f());
            assert_eq!(
                c.outcome == Outcome::Feasible,
                feasible,
                "the outcome at (L_z, K*) = ({:e}, {:e}) is {:?}",
                lz.f(),
                k.f(),
                c.outcome
            );
            if !feasible {
                continue;
            }
            let [rp, rl, rk] = residuals(&masses, &config, lz, k, &c);
            worst.see(&format!("{} Σp", R::NAME), rp);
            worst.see(&format!("{} L_z", R::NAME), rl);
            worst.see(&format!("{} K", R::NAME), rk);
            let at = format!(
                "at (L_z, K*) = ({:e}, {:e}), m = {:?} ({})",
                lz.f(),
                k.f(),
                masses.m,
                R::NAME
            );
            assert!(rp <= TOL_P, "Σp is {rp} eps of Σ|p| {at}");
            assert!(rl <= TOL_L, "L_z(p) misses L_z by {rl} eps {at}");
            assert!(rk <= TOL_K, "K(p) misses K* by {rk} eps {at}");
        }
    }
}

/// `pᵢ = mᵢ(L_z·J rᵢ + a·wᵢ)`: the rigid rotation at `a = 0`, and a control's broken construction.
fn assemble<R: Q>(m: [R; 3], r: &[[R; 2]; 3], lz: R, a: R, w: &[[R; 2]; 3]) -> [[R; 2]; 3] {
    std::array::from_fn(|i| {
        let rot = [-r[i][1], r[i][0]];
        [0, 1].map(|d| m[i] * (lz * rot[d] + a * w[i][d]))
    })
}

#[test]
fn invariant_construction_targets() {
    let worst = Worst::default();
    check_targets::<f64>(construct::<f64>, &worst);
    check_targets::<f32>(construct::<f32>, &worst);
    worst.print("invariant_construction_targets");
}

negative_control!(
    invariant_construction_targets,
    "a construction whose a is 1e-4 too large must miss K*",
    expected = "misses K*",
    check_targets::<f64>(
        |m, c, lz, k| {
            let out = construct(m, c, lz, k);
            Construction {
                p: assemble(m.m, &c.r, lz, out.a * 1.0001, &out.w),
                ..out
            }
        },
        &Worst::default()
    )
);

mod targets_pit9 {
    use super::*;

    negative_control!(
        invariant_construction_targets,
        "a projection whose CoM drift is taken with m₀ and m₁ swapped must fail Σp = 0 at unequal masses (PIT-9)",
        expected = "Σp is",
        check_targets::<f64>(
            |m, c, lz, k| {
                let out = construct(m, c, lz, k);
                let swapped = [m.m[1], m.m[0], m.m[2]];
                let v = invariant::velocities(m, c.rho, [0.0; 2]);
                let w = invariant::project(swapped, &c.r, &v);
                let norm = invariant::mass_norm2(m.m, &w).sqrt();
                let w = w.map(|wi| wi.map(|x| x / norm));
                Construction {
                    p: assemble(m.m, &c.r, lz, out.a, &w),
                    ..out
                }
            },
            &Worst::default()
        )
    );
}

// ── invariant_construction_infeasible_tagged ────────────────────────────────────────────────────────────────────

/// `L_z` with at most 12 significant bits, so `L_z²/2` is exact at both precisions.
const SHORT: [f64; 6] = [0.75, -1.375, 0.001953125, 3.5, -0.0625, 1.0];

/// Each pair is tagged as the exact test says, `K* = L_z²/2` feasible with `a = 0` and one value below infeasible;
/// infeasible momenta are finite and the rigid rotation; the label is `decode_failed`/3; infeasibility takes precedence
/// over `DEGENERATE`.
fn check_tagged<R: Q>(build: Construct<R>) {
    let mut g = SplitMix::new(17);
    let mut cases = Vec::new();
    for &l in &SHORT {
        let (lz, edge) = (R::of(l), R::of(l * l / 2.0));
        assert_eq!(edge.f(), l * l / 2.0, "L_z²/2 is exact");
        cases.push((lz, edge, Outcome::Feasible));
        cases.push((lz, step(edge, -1), Outcome::Infeasible));
        cases.push((lz, step(edge, 1), Outcome::Feasible));
        cases.push((lz, R::zero(), Outcome::Infeasible));
        cases.push((lz, -R::one(), Outcome::Infeasible));
    }
    for _ in 0..2000 {
        let l = (g.next_u64() >> 11) as f64 / (1u64 << 50) as f64 - 4.0;
        let lz = R::of(l);
        let k = step(R::of(lz.f() * lz.f() / 2.0), (g.next_u64() % 5) as i64 - 2);
        let want = if reference(1.0, k.f(), lz.f()) {
            Outcome::Infeasible
        } else {
            Outcome::Feasible
        };
        cases.push((lz, k, want));
    }
    let coincident = mass::from_masses([0.25, 0.25, 0.5].map(R::of));
    let point = config_of(&coincident, [R::zero(); 2], [R::zero(); 2]);
    let mut configs = decoded::<R>();
    configs.push((coincident, point));
    for (masses, config) in &configs {
        let degenerate = config.r == point.r;
        for &(lz, k, want) in &cases {
            let c = build(masses, config, lz, k);
            let want = if degenerate && want == Outcome::Feasible {
                Outcome::Degenerate
            } else {
                want
            };
            let at = format!("at (L_z, K*) = ({:e}, {:e}) ({})", lz.f(), k.f(), R::NAME);
            assert_eq!(c.outcome, want, "the tag {at}");
            assert!(
                c.p.iter().flatten().all(|x| x.f().is_finite()),
                "a non-finite momentum {at}: {:?}",
                c.p
            );
            if want != Outcome::Feasible {
                assert_eq!(c.a, R::zero(), "a is not 0 {at}");
                let still = [[R::zero(); 2]; 3];
                let rigid = assemble(masses.m, &config.r, lz, R::zero(), &still);
                assert_eq!(c.p, rigid, "the momenta are not the rigid rotation {at}");
            }
            if want == Outcome::Feasible
                && k == R::of(lz.f() * lz.f() / 2.0)
                && SHORT.contains(&lz.f())
            {
                assert_eq!(c.a, R::zero(), "a at K* = L_z²/2 {at}");
            }
        }
    }
    assert_eq!(
        infeasible_label(),
        (STATE_DECODE_FAILED, 3),
        "the infeasible label"
    );
}

#[test]
fn invariant_construction_infeasible_tagged() {
    check_tagged::<f64>(construct::<f64>);
    check_tagged::<f32>(construct::<f32>);
}

negative_control!(
    invariant_construction_infeasible_tagged,
    "a construction that tests K* ≤ L_z²/2 must fail at the boundary",
    expected = "the tag",
    check_tagged::<f64>(|m, c, lz, k| {
        let out = construct(m, c, lz, k);
        if k == lz * lz / 2.0 && out.outcome == Outcome::Feasible {
            Construction {
                outcome: Outcome::Infeasible,
                ..out
            }
        } else {
            out
        }
    })
);

mod tagged_nan {
    use super::*;

    negative_control!(
        invariant_construction_infeasible_tagged,
        "a construction that takes √(2(K* − K_min)) where infeasible must fail on its NaN",
        expected = "a non-finite momentum",
        check_tagged::<f64>(|m, c, lz, k| {
            let out = construct(m, c, lz, k);
            let a = f64::sqrt(2.0 * k - lz * lz);
            Construction {
                p: assemble(m.m, &c.r, lz, a, &out.w),
                ..out
            }
        })
    );
}

// ── invariant_construction_binade ───────────────────────────────────────────────────────────────────────────────

/// A binade reader under check.
type Reader<R> = fn(R) -> Binade<R>;

/// `read` gives each value's sign bit, zero flag and `|x| = s·2^e`, `s ∈ [1, 2)`, exactly, subnormals included; and
/// `s`'s limbs sum to `s`, each on its grid and below its bound (`w` = 8 at f32, 18 at f64).
fn check_binade<R: Q>(read: Reader<R>) {
    let w: i32 = if R::FRAC == 23 { 8 } else { 18 };
    let mut g = SplitMix::new(11);
    let mut xs = vec![
        R::zero(),
        R::raw(1),
        R::raw(0x15),
        R::one(),
        R::of(1.5),
        make::<R>(2 * R::BIAS, u64::MAX),
    ];
    xs.extend((0..2000).map(|_| R::raw(g.next_u64() & ((1u64 << (R::FRAC + 4)) - 1))));
    xs.extend((0..2000).map(|_| rand_pos::<R>(&mut g, 0, 2 * R::BIAS)));
    for x in xs.clone().into_iter().chain(xs.into_iter().map(|x| -x)) {
        let b = read(x);
        let at = format!("at {:e} ({})", x.f(), R::NAME);
        assert_eq!(b.negative, x.f().is_sign_negative(), "the sign flag {at}");
        assert_eq!(b.zero, x.f() == 0.0, "the zero flag {at}");
        if x.f() == 0.0 {
            continue;
        }
        assert!(
            (1.0..2.0).contains(&b.s.f()),
            "s = {} is not in [1, 2) {at}",
            b.s.f()
        );
        let scaled = b.s.f() * 2f64.powi(b.e / 2) * 2f64.powi(b.e - b.e / 2);
        assert_eq!(scaled, x.f().abs(), "s·2^e {at}");
        let l = b.s.limbs().map(|v| v.f());
        assert_eq!(l[0] + l[1] + l[2], b.s.f(), "the limbs' sum {at}");
        for (a, v) in l.iter().enumerate() {
            let grid = 2f64.powi(-(a as i32 + 1) * w + 1);
            assert_eq!((v / grid).fract(), 0.0, "limb {a} is off its grid {at}");
            assert!(
                *v >= 0.0 && *v < 2.0 * 2f64.powi(-(a as i32) * w),
                "limb {a} = {v:e} {at}"
            );
        }
    }
}

#[test]
fn invariant_construction_binade() {
    check_binade::<f64>(<f64 as ExactSign>::binade);
    check_binade::<f32>(<f32 as ExactSign>::binade);
}

negative_control!(
    invariant_construction_binade,
    "a reader whose sign flag is inverted must fail",
    expected = "the sign flag",
    check_binade::<f32>(|x| Binade {
        negative: !x.binade().negative,
        ..x.binade()
    })
);

// ── invariant_construction_entries ──────────────────────────────────────────────────────────────────────────────

/// The feasibility entry point.
type FeasibilityEntry = fn(UVec3, &[u32], &mut [u32]);

/// `feasibility` and `construct_defaults`, called natively, write the f32 test and construction at each sample and
/// nothing past the last.
fn check_entries(feasibility: FeasibilityEntry, construct_entry: FeasibilityEntry) {
    let ts: Vec<(f32, f32, f32)> = triples::<f32>(5).into_iter().step_by(97).collect();
    let input: Vec<u32> = ts
        .iter()
        .flat_map(|&(i, k, l)| [i.to_bits(), k.to_bits(), l.to_bits()])
        .collect();
    let mut out = vec![7u32; ts.len() + 1];
    for n in 0..=ts.len() as u32 {
        feasibility(UVec3::new(n, 0, 0), &input, &mut out);
    }
    for (n, &(i, k, l)) in ts.iter().enumerate() {
        assert_eq!(
            out[n],
            u32::from(infeasible(i, k, l)),
            "feasibility word {n}"
        );
    }
    assert_eq!(out[ts.len()], 7, "feasibility wrote past its input");

    let samples: [[f32; 6]; 4] = [
        [0.8, -0.4, 0.3, -0.7, 0.5, 0.2],
        [0.8, -0.4, 0.3, -0.7, 0.5, 0.1],
        [0.0, 0.0, 0.0, 0.0, -1.5, 3.0],
        [-1.3, 2.0, 1.1, 0.4, 0.0, 0.0],
    ];
    let input: Vec<u32> = samples.iter().flatten().map(|x| x.to_bits()).collect();
    let words = invariant::construct_out_words();
    assert_eq!(invariant::construct_in_words(), 6, "the input words");
    let mut out = vec![7u32; words * (samples.len() + 1)];
    for n in 0..=samples.len() as u32 {
        construct_entry(UVec3::new(n, 0, 0), &input, &mut out);
    }
    for (n, s) in samples.iter().enumerate() {
        let masses = mass::decode::<Defaults, f32>([s[0], s[1]]);
        let config = kernel::decode::config::decode::<Defaults, f32>([s[2], s[3]], &masses);
        let c = construct(&masses, &config, s[4], s[5]);
        let want: Vec<u32> =
            c.p.iter()
                .flatten()
                .map(|x| x.to_bits())
                .chain([
                    invariant::outcome_word(c.outcome),
                    invariant::seed_word(c.seed),
                ])
                .collect();
        assert_eq!(
            out[words * n..words * (n + 1)],
            want[..],
            "construction sample {n}"
        );
    }
    assert!(
        out[words * samples.len()..].iter().all(|&w| w == 7),
        "the construction wrote past its input"
    );
    let words_of = [
        (Outcome::Feasible, 0),
        (Outcome::Infeasible, 1),
        (Outcome::Degenerate, 2),
    ];
    for (o, w) in words_of {
        assert_eq!(invariant::outcome_word(o), w, "{o:?}'s word");
    }
    let seeds = [
        (Some(Seed::Rho), 0),
        (Some(Seed::Lambda), 1),
        (Some(Seed::JRho), 2),
        (Some(Seed::JLambda), 3),
        (None, 4),
    ];
    for (s, w) in seeds {
        assert_eq!(invariant::seed_word(s), w, "{s:?}'s word");
    }
}

#[test]
fn invariant_construction_entries() {
    check_entries(invariant::feasibility, invariant::construct_defaults);
}

negative_control!(
    invariant_construction_entries,
    "a feasibility entry that writes the comparison's negation must fail",
    expected = "feasibility word",
    check_entries(
        |id, i, o| {
            invariant::feasibility(id, i, o);
            let n = id.x as usize;
            if n < o.len() && n < i.len() / 3 {
                o[n] ^= 1;
            }
        },
        invariant::construct_defaults
    )
);

// ── Seed selection ──────────────────────────────────────────────────────────────────────────────────────────────

/// A seed rule under check.
type Chooser<R> = fn([R; 4]) -> Option<Seed>;

/// `ε_w` at `R`.
fn eps_w<R: Q>() -> R {
    R::EPS_W
}

/// The seeds, in order.
const SEEDS: [Seed; 4] = [Seed::Rho, Seed::Lambda, Seed::JRho, Seed::JLambda];

/// §2.2's `w⁽²⁾` norms, transcribed independently at f64 with the configuration's own `M` and `I`.
fn reference_norms(m: [f64; 3], r: [[f64; 2]; 3], rho: [f64; 2], lambda: [f64; 2]) -> [f64; 4] {
    let j = |v: [f64; 2]| [-v[1], v[0]];
    let big_m: f64 = m.iter().sum();
    let m01 = m[0] + m[1];
    let inertia: f64 = (0..3)
        .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
        .sum();
    let norm = |rd: [f64; 2], ld: [f64; 2]| {
        let v: [[f64; 2]; 3] = [
            [0, 1].map(|d| -(m[2] / big_m) * ld[d] - (m[1] / m01) * rd[d]),
            [0, 1].map(|d| -(m[2] / big_m) * ld[d] + (m[0] / m01) * rd[d]),
            [0, 1].map(|d| (m01 / big_m) * ld[d]),
        ];
        let c = [0, 1].map(|d| (0..3).map(|i| m[i] * v[i][d]).sum::<f64>());
        let w1: Vec<[f64; 2]> = v
            .iter()
            .map(|vi| [vi[0] - c[0] / big_m, vi[1] - c[1] / big_m])
            .collect();
        let l: f64 = (0..3)
            .map(|i| m[i] * (r[i][0] * w1[i][1] - r[i][1] * w1[i][0]))
            .sum();
        let beta = if inertia == 0.0 { 0.0 } else { l / inertia };
        (0..3)
            .map(|i| {
                let w = [w1[i][0] - beta * j(r[i])[0], w1[i][1] - beta * j(r[i])[1]];
                m[i] * (w[0] * w[0] + w[1] * w[1])
            })
            .sum::<f64>()
    };
    let z = [0.0; 2];
    [
        norm(rho, z),
        norm(z, lambda),
        norm(j(rho), z),
        norm(z, j(lambda)),
    ]
}

/// The seed the rule takes on `norms`, by its statement: the largest above `ε_w`, the first of equals.
fn rule(norms: [f64; 4], eps: f64) -> Option<Seed> {
    let mut best: Option<(usize, f64)> = None;
    for (n, &x) in norms.iter().enumerate() {
        if x > eps && best.is_none_or(|(_, b)| x > b) {
            best = Some((n, x));
        }
    }
    best.map(|(n, _)| SEEDS[n])
}

/// The norms of a fixture as the construction computes them, checked against the reference.
fn fixture_norms<R: Q>(masses: &Masses<R>, config: &Config<R>) -> [R; 4] {
    let c = construct(masses, config, R::zero(), R::one());
    let want = reference_norms(
        masses.m.map(|x| x.f()),
        config.r.map(|v| v.map(|x| x.f())),
        config.rho.map(|x| x.f()),
        config.lambda.map(|x| x.f()),
    );
    for n in 0..4 {
        let tol = 64.0 * R::eps() * want.iter().cloned().fold(0.0, f64::max).max(1e-300);
        assert!(
            (c.norms[n].f() - want[n]).abs() <= tol,
            "seed {n}'s norm {} is not §2.2's {} ({})",
            c.norms[n].f(),
            want[n],
            R::NAME
        );
    }
    assert_eq!(
        c.seed,
        choose(c.norms),
        "the construction's seed is the rule's on its norms"
    );
    c.norms
}

/// Where several seeds qualify, `chooser` takes the largest: a dyadic fixture whose second seed is larger than its
/// first, and decoded configurations, against the rule on the reference norms where they are apart.
fn check_largest<R: Q>(chooser: Chooser<R>) {
    let (masses, config) = dyadic::<R>([0.375, 0.125, 0.5], [2.0, 0.0], [1.5, 0.5]);
    let norms = fixture_norms(&masses, &config);
    assert_eq!(norms[0].f(), 0.375, "the fixture's primary norm");
    assert_eq!(norms[1].f(), 0.625, "the fixture's second norm");
    let want = rule(norms.map(|x| x.f()), eps_w::<R>().f());
    assert_ne!(
        want,
        Some(Seed::Rho),
        "the fixture's largest is not the first"
    );
    assert_eq!(
        chooser(norms),
        want,
        "the largest of {norms:?} ({})",
        R::NAME
    );
    for (masses, config) in decoded::<R>() {
        let norms = fixture_norms(&masses, &config);
        let mut sorted = norms.map(|x| x.f());
        sorted.sort_by(|a, b| b.total_cmp(a));
        if sorted[0] - sorted[1] > 1e-3 * sorted[0] {
            assert_eq!(
                chooser(norms),
                rule(norms.map(|x| x.f()), eps_w::<R>().f()),
                "the largest of {norms:?}"
            );
        }
    }
}

#[test]
fn seed_selection_largest() {
    check_largest::<f64>(choose::<f64>);
    check_largest::<f32>(choose::<f32>);
}

negative_control!(
    seed_selection_largest,
    "a rule that takes the first qualifying seed must fail",
    expected = "the largest of",
    check_largest::<f64>(|n| SEEDS
        .into_iter()
        .zip(n)
        .find(|(_, x)| *x > eps_w::<f64>())
        .map(|(s, _)| s))
);

/// An exact tie keeps the earlier seed: the dyadic fixture whose norms are `(½, ½, ¼, ¼)` exactly, and ties at every
/// pair of positions.
fn check_tie<R: Q>(chooser: Chooser<R>) {
    let (masses, config) = dyadic::<R>([0.25, 0.25, 0.5], [2.0, 0.0], [1.0, 1.0]);
    let norms = fixture_norms(&masses, &config);
    assert_eq!(
        norms.map(|x| x.f()),
        [0.5, 0.5, 0.25, 0.25],
        "the fixture's norms"
    );
    assert_eq!(chooser(norms), Some(Seed::Rho), "the tie of {norms:?}");
    let (lo, hi) = (R::of(0.25), R::of(0.5));
    for a in 0..4 {
        for b in a + 1..4 {
            let mut n = [lo; 4];
            n[a] = hi;
            n[b] = hi;
            assert_eq!(chooser(n), Some(SEEDS[a]), "the tie of {n:?}");
        }
    }
}

#[test]
fn seed_selection_tie() {
    check_tie::<f64>(choose::<f64>);
    check_tie::<f32>(choose::<f32>);
}

negative_control!(
    seed_selection_tie,
    "a rule that takes the later of equal norms must fail",
    expected = "the tie of",
    check_tie::<f64>(|n| {
        let mut best = None;
        for (s, x) in SEEDS.into_iter().zip(n) {
            if x > eps_w::<f64>() && best.is_none_or(|(_, b)| x >= b) {
                best = Some((s, x));
            }
        }
        best.map(|(s, _)| s)
    })
);

/// With `ρ = 0` the primary seed degenerates and the construction takes `(0, λ)`, feasible, not `DEGENERATE`.
fn check_fallback<R: Q>(chooser: Chooser<R>) {
    let (masses, config) = dyadic::<R>([0.25, 0.25, 0.5], [0.0, 0.0], [2.0, 0.0]);
    let norms = fixture_norms(&masses, &config);
    assert_eq!(
        norms.map(|x| x.f()),
        [0.0, 1.0, 0.0, 0.0],
        "the fixture's norms"
    );
    assert_eq!(
        chooser(norms),
        Some(Seed::Lambda),
        "the fallback at ρ = 0 ({})",
        R::NAME
    );
    let c = construct(&masses, &config, R::of(0.5), R::one());
    assert_eq!(
        (c.seed, c.outcome),
        (Some(Seed::Lambda), Outcome::Feasible),
        "the construction at ρ = 0"
    );
}

#[test]
fn seed_selection_fallback() {
    check_fallback::<f64>(choose::<f64>);
    check_fallback::<f32>(choose::<f32>);
}

negative_control!(
    seed_selection_fallback,
    "a rule with no fallback seed must fail at ρ = 0",
    expected = "the fallback at ρ = 0",
    check_fallback::<f64>(|n| (n[0] > eps_w::<f64>()).then_some(Seed::Rho))
);

/// `DEGENERATE` (no seed) only where no norm exceeds `ε_w`: `ε_w` itself does not qualify and the next float above it
/// does, at each position; random norms around `ε_w` follow the rule; the coincident configuration is `DEGENERATE`.
fn check_degenerate<R: Q>(chooser: Chooser<R>) {
    let eps = eps_w::<R>();
    let above = step(eps, 1);
    for a in 0..4 {
        let mut n = [R::zero(); 4];
        n[a] = eps;
        assert_eq!(
            chooser(n),
            None,
            "a norm of exactly ε_w qualifies, at seed {a} ({})",
            R::NAME
        );
        n[a] = above;
        assert_eq!(
            chooser(n),
            Some(SEEDS[a]),
            "the norm just above ε_w at seed {a} does not qualify"
        );
    }
    let mut g = SplitMix::new(3);
    for _ in 0..4000 {
        let n = [0; 4].map(|_| step(eps, (g.next_u64() % 9) as i64 - 4));
        assert_eq!(
            chooser(n),
            rule(n.map(|x| x.f()), eps.f()),
            "the rule on {n:?}"
        );
    }
    let (masses, config) = dyadic::<R>([0.25, 0.25, 0.5], [0.0, 0.0], [0.0, 0.0]);
    let norms = fixture_norms(&masses, &config);
    assert_eq!(chooser(norms), None, "the coincident configuration's seed");
}

#[test]
fn seed_selection_degenerate() {
    check_degenerate::<f64>(choose::<f64>);
    check_degenerate::<f32>(choose::<f32>);
}

negative_control!(
    seed_selection_degenerate,
    "a rule that lets a norm of exactly ε_w qualify must fail",
    expected = "a norm of exactly ε_w qualifies",
    check_degenerate::<f64>(|n| rule(n, eps_w::<f64>() * (1.0 - 1e-12)))
);
