//! The invariant charts `(L_z, E)` and `(L_z, K)` (`kernel::chart::invariant`; chart_reference §2, §2.1, §5.1, §5.2;
//! REQ-CHART-017), each check at the f64 and the f32 instantiation natively (R-118, R-265):
//! - `invariant_warp_feasible`: no `(u, v)` in `[0,1]²` gives `K* < K_min` under the warp, on both charts: the warp's
//!   targets pass the construction's exact test and the chart's decode is `Feasible`, over random `(u, v)`, warp
//!   parameters and frozen configurations, and over every edge `s ∈ {0, 1}` on a fine grid of `t`, where a rounded
//!   `√(2K)` above the exact root would put `L_z² > 2K` (chart_reference §5.2, "Feasibility").
//! - `invariant_warp_formula`: `K(t) = K_max·t^γ_K`, `L_max(t)` the largest float with `L_max² ≤ 2K` within the steps
//!   it takes from the rounded root, `L_z(s, t) = (2s − 1)·L_max(t)`; `E(t) = U + K(t)` on `(L_z, E)`.
//! - `invariant_chart_decode_targets`: the chart's decode is the shared decoder with the construction on the decoded
//!   masses and configuration, its seed decided on the very `z_α` the configuration came from, bit for bit; it meets
//!   `Σpᵢ = 0`, `L_z(p) = L_z` and `K(p) = K*` within REQ-DEC-046's proposed tolerances over random `(u, v)`, near
//!   `K* → K_min` included; the two charts decode alike (chart_reference §5.2).
//! - `invariant_chart_validate`: `Valid` on `[0,1]²` and above it; `t < 0` and `s ∉ [0, 1]` projected onto the
//!   feasible region, naming each chart's constraint; a non-finite coordinate rejected (R-26; chart_reference §5.1).
//! - `invariant_chart_forbids_energy_normalisation`: both charts set the flag and carry their names into the dump
//!   header (chart_reference §0.6, §5.1).
//! - `invariant_chart_stage`: the uniform round-trips the chart; the decode stage's compute entry points for both
//!   charts, called natively, write each sample's decode, the chart's own, and nothing past the last (REQ-CHART-001).
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.

mod decode_support;

use decode_support::*;
use kernel::chart::invariant::{self, warp, Invariant, Targets, WarpReal};
use kernel::chart::{
    write_dump_header, Chart, ChartDecode, Constraint, InvariantLE, InvariantLK, ValidationResult,
};
use kernel::decode::invariant::{construct, infeasible, Outcome};
use kernel::decode::{config, mass, Decoded};
use kernel::generated::links::*;
use kernel::stage::decode::{
    descriptor_words, flags, ic_words, invariant_le_defaults, invariant_lk_defaults,
    read_descriptor, read_ic, sample, untag, DecodeMode, DecodeUniforms, DecodeVariant,
    InvariantLEDefaults, InvariantLKDefaults, UniformChart,
};
use proptest::prelude::*;
use spirv_std::glam::UVec3;
use validation::decode_f32_eps::SplitMix;
use validation::{negative_control, prop, GAMMA_K, K_MAX};

/// `|Σpᵢ|` in units of `eps·Σ|pᵢ|`, at most (REQ-DEC-046, proposed; `invariant_construction.rs`).
const TOL_P: f64 = 4.0;
/// `|L_z(p) − L_z|` in units of `eps·(|L_z| + √(2K*))`, at most (REQ-DEC-046, proposed).
const TOL_L: f64 = 4.0;
/// `|K(p) − K*|` in units of `eps·K*`, at most (REQ-DEC-046, proposed).
const TOL_K: f64 = 8.0;

/// A precision under test.
trait Q: P + WarpReal {
    const LABEL: &'static str;
    /// The next float above a positive finite `self`.
    fn up(self) -> Self;
}

impl Q for f64 {
    const LABEL: &'static str = "f64";
    fn up(self) -> Self {
        f64::from_bits(self.to_bits() + 1)
    }
}

impl Q for f32 {
    const LABEL: &'static str = "f32";
    fn up(self) -> Self {
        f32::from_bits(self.to_bits() + 1)
    }
}

/// The frozen configurations the checks run at: the golden IC's (`z = 0`, the seed rule's `z_α = 0`), unequal masses
/// (PIT-9), a hierarchical triple and a wide inner pair, each `(z_α, z_β, z_μ₁, z_μ₂)`.
const FROZEN: [[f64; 4]; 6] = [
    [0.0, 0.0, 0.0, 0.0],
    [0.3, -0.5, 0.7, -0.4],
    [1.8, 0.4, -1.0, 1.2],
    [-1.5, 1.0, 0.2, 0.9],
    [-0.25, -1.75, 1.5, 1.5],
    [-1e-30, 0.6, -2.0, 0.1],
];

/// The chart parameters `(z_α, z_β, z_μ₁, z_μ₂)` under `(K_max, γ_K)`, at `R`.
fn inv<R: Q>(z: [f64; 4], k_max: f64, gamma_k: f64) -> Invariant<R> {
    Invariant {
        z_cfg: [R::of(z[0]), R::of(z[1])],
        z_mass: [R::of(z[2]), R::of(z[3])],
        k_max: R::of(k_max),
        gamma_k: R::of(gamma_k),
    }
}

/// The register's warp defaults (REQ-CHART-044, proposed).
fn defaults() -> (f64, f64) {
    (K_MAX.number(), GAMMA_K.number())
}

/// A warp under check: `(K_max, γ_K, s, t)` to the targets.
type Warp<R> = fn(R, R, R, R) -> Targets<R>;

// ── invariant_warp_feasible (REQ-CHART-017) ──────────────────────────────────────────────────────────────────────

/// At `(s, t)` under `(K_max, γ_K)`: the targets `warp` gives pass the exact test `2·I·K* ≥ L_z²`, and both charts'
/// decodes at the frozen `z` are `Feasible` (the charts decode through the kernel's own warp).
fn check_point<R: Q>(warp: Warp<R>, z: [f64; 4], k_max: f64, gamma_k: f64, s: R, t: R) {
    let t_ = warp(R::of(k_max), R::of(gamma_k), s, t);
    assert!(
        !infeasible(R::one(), t_.k, t_.lz),
        "infeasible under the warp at (s, t) = ({:e}, {:e}), K_max = {k_max}, γ_K = {gamma_k} ({}): K* = {:e}, \
         L_z = {:e}",
        s.f(),
        t.f(),
        R::LABEL,
        t_.k.f(),
        t_.lz.f()
    );
    let c = inv::<R>(z, k_max, gamma_k);
    for (name, outcome) in [
        (
            "InvariantLE",
            invariant::decode::<Defaults, R, _>(&InvariantLE(c), s, t).1,
        ),
        (
            "InvariantLK",
            invariant::decode::<Defaults, R, _>(&InvariantLK(c), s, t).1,
        ),
    ] {
        assert_eq!(
            outcome,
            Outcome::Feasible,
            "{name}'s decode at (s, t) = ({:e}, {:e}) is not feasible ({})",
            s.f(),
            t.f(),
            R::LABEL
        );
    }
}

/// `n` values of `t` on `[0, 1]`: the evenly spaced grid `j/(n − 1)` and the floats next to 0 and 1.
fn ts<R: Q>(n: u32) -> Vec<R> {
    let mut out: Vec<R> = (0..n)
        .map(|j| R::of(f64::from(j) / f64::from(n - 1)))
        .collect();
    let tiny = R::of(f64::from(f32::MIN_POSITIVE));
    out.extend([R::zero(), tiny, R::one(), R::one() - R::of(R::eps() / 2.0)]);
    out
}

/// The warp over its whole domain at `R`: random `(u, v)`, `(K_max, γ_K)` and frozen `z`, and both edges `s ∈ {0, 1}`
/// on a fine grid of `t`, at the defaults and at a spread of parameters.
fn check_feasible<R: Q>(warp: Warp<R>) {
    let (k0, g0) = defaults();
    let params = [
        (k0, g0),
        (1.0, 1.0),
        (0.37, 1.0),
        (40.0, 1.5),
        (1e-3, 3.25),
        (7.5, 8.0),
    ];
    for &(k_max, gamma_k) in &params {
        for t in ts::<R>(1 << 12) {
            for s in [R::zero(), R::one()] {
                check_point::<R>(warp, FROZEN[1], k_max, gamma_k, s, t);
            }
        }
    }
    let strategy = (
        0.0f64..=1.0,
        0.0f64..=1.0,
        prop_oneof![Just(k0), 1e-4f64..=1e3],
        prop_oneof![Just(g0), Just(1.0), 1.0f64..=8.0],
        0usize..FROZEN.len(),
        any::<bool>(),
    );
    prop::run(&strategy, |(u, v, k_max, gamma_k, z, edge)| {
        let s = if edge { u.round() } else { u };
        check_point::<R>(warp, FROZEN[z], k_max, gamma_k, R::of(s), R::of(v));
        Ok(())
    });
}

#[test]
fn invariant_warp_feasible() {
    check_feasible::<f64>(warp::<f64>);
    check_feasible::<f32>(warp::<f32>);
}

/// §2.1's warp with the rounded root unchecked, `L_max = √(2K)`: at an edge `s ∈ {0, 1}` a root rounded up gives
/// `L_z² > 2K`.
#[cfg(feature = "controls")]
fn rounded_root<R: Q>(k_max: R, gamma_k: R, s: R, t: R) -> Targets<R> {
    let k = k_max * t.powf(gamma_k);
    let l_max = (k + k).sqrt();
    Targets {
        k,
        l_max,
        lz: (s + s - R::one()) * l_max,
    }
}

negative_control!(
    invariant_warp_feasible,
    "a warp whose L_max is the rounded √(2K), unchecked, is infeasible at an edge",
    expected = "infeasible under the warp",
    check_feasible::<f32>(rounded_root::<f32>)
);

// ── invariant_warp_formula (REQ-CHART-017) ───────────────────────────────────────────────────────────────────────

/// `warp` at `R` is §2.1's: `K = K_max·t^γ_K` as written; `L_max` within four floats below the rounded root, with
/// `L_max² ≤ 2K` exactly and the next float up past it unless it is the rounded root itself; `L_z = (2s − 1)·L_max`;
/// and `E(t) = U + K(t)` on `(L_z, E)`.
fn check_formula<R: Q>(warp: Warp<R>) {
    let (k0, g0) = defaults();
    let mut g = SplitMix::new(7);
    let mut unit = || R::of((g.next_u64() >> 11) as f64 / (1u64 << 53) as f64);
    for (k_max, gamma_k) in [(k0, g0), (1.0, 1.0), (40.0, 1.5)] {
        let (km, gk) = (R::of(k_max), R::of(gamma_k));
        for _ in 0..4096 {
            let (s, t) = (unit(), unit());
            let got = warp(km, gk, s, t);
            let ctx = format!(
                "at (s, t) = ({:e}, {:e}), K_max = {k_max}, γ_K = {gamma_k} ({})",
                s.f(),
                t.f(),
                R::LABEL
            );
            assert_eq!(got.k, km * t.powf(gk), "K(t) is not K_max·t^γ_K {ctx}");
            let root = (got.k + got.k).sqrt();
            assert!(
                got.l_max <= root && R::ulps(got.l_max, root) <= 4,
                "L_max {:e} is not within four floats below √(2K) {:e} {ctx}",
                got.l_max.f(),
                root.f()
            );
            assert!(!infeasible(R::one(), got.k, got.l_max), "L_max² > 2K {ctx}");
            if got.l_max != root {
                assert!(
                    infeasible(R::one(), got.k, got.l_max.up()),
                    "L_max {:e} is stepped below a feasible float {ctx}",
                    got.l_max.f()
                );
            }
            assert_eq!(
                got.lz,
                (s + s - R::one()) * got.l_max,
                "L_z is not (2s − 1)·L_max {ctx}"
            );
            let le = InvariantLE(inv::<R>(FROZEN[0], k_max, gamma_k));
            let u = R::of(-0.25);
            assert_eq!(le.energy(u, t), u + got.k, "E(t) is not U + K(t) {ctx}");
        }
    }
}

#[test]
fn invariant_warp_formula() {
    check_formula::<f64>(warp::<f64>);
    check_formula::<f32>(warp::<f32>);
}

/// A warp with `K` linear in `t`, `γ_K` ignored.
#[cfg(feature = "controls")]
fn linear<R: Q>(k_max: R, _gamma_k: R, s: R, t: R) -> Targets<R> {
    warp(k_max, R::one(), s, t)
}

negative_control!(
    invariant_warp_formula,
    "a warp ignoring γ_K is not K_max·t^γ_K",
    expected = "K(t) is not K_max·t^γ_K",
    check_formula::<f64>(linear::<f64>)
);

// ── invariant_chart_decode_targets (chart_reference §5.2) ────────────────────────────────────────────────────────

/// A chart decode under check: the chart, `(u, v)`, to the decode and the construction's outcome.
type ChartDecodeFn<R> = fn(&InvariantLE<R>, R, R) -> (Decoded<R>, Outcome);

/// The kernel's.
fn kernel_decode<R: Q>(c: &InvariantLE<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    invariant::decode::<Defaults, R, _>(c, u, v)
}

/// The residuals of a decode's momenta against `(L_z, K*)`, each in its unit: `(Σp, L_z, K)`.
fn residuals<R: Q>(d: &Decoded<R>, lz: R, k: R) -> [f64; 3] {
    let m = d.ic.m.map(|x| x.f());
    let r = d.ic.r.map(|v| v.map(|x| x.f()));
    let p = d.ic.p.map(|v| v.map(|x| x.f()));
    let (lz, k) = (lz.f(), k.f());
    let sum = [p[0][0] + p[1][0] + p[2][0], p[0][1] + p[1][1] + p[2][1]];
    let size: f64 = p.iter().map(|v| v[0].abs() + v[1].abs()).sum();
    let l: f64 = (0..3).map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0]).sum();
    let kin: f64 = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum();
    let over = |d: f64, scale: f64| {
        if d == 0.0 {
            0.0
        } else {
            d / (R::eps() * scale)
        }
    };
    [
        over(sum[0].abs() + sum[1].abs(), size),
        over((l - lz).abs(), lz.abs() + (2.0 * k).sqrt()),
        over((kin - k).abs(), k),
    ]
}

/// The decode at `(u, v)` of the `(L_z, E)` chart at frozen `z`: the shared blocks and the construction on the
/// decoded `z_α`, bit for bit; `(L_z, K)` the same; and the targets met within the tolerances.
fn check_decode_point<R: Q>(decode: ChartDecodeFn<R>, z: [f64; 4], u: R, v: R, worst: &Worst) {
    let (k0, g0) = defaults();
    let c = inv::<R>(z, k0, g0);
    let (got, outcome) = decode(&InvariantLE(c), u, v);
    let t = c.targets(u, v);
    let masses = mass::decode::<Defaults, R>(c.z_mass);
    let cfg = config::decode::<Defaults, R>(c.z_cfg, &masses);
    let want = construct(&masses, &cfg, c.z_cfg[0], t.lz, t.k);
    let ctx = format!(
        "at (u, v) = ({:e}, {:e}), z = {z:?} ({})",
        u.f(),
        v.f(),
        R::LABEL
    );
    assert_eq!(
        (got.ic.p, outcome),
        (want.p, want.outcome),
        "the chart's decode is not the construction on the decoded z_α {ctx}"
    );
    assert_eq!(
        (got.masses, got.config, got.ic.m, got.ic.r),
        (masses, cfg, masses.m, cfg.r),
        "the chart's mass and configuration are not the shared blocks' {ctx}"
    );
    check_jacobi(&got, &ctx);
    let lk = invariant::decode::<Defaults, R, _>(&InvariantLK(c), u, v);
    assert_eq!(lk.0, got, "(L_z, K) and (L_z, E) decode differently {ctx}");
    assert_eq!(
        InvariantLE(c).decode_at::<Defaults, R>(u, v),
        got,
        "ChartDecode is not the chart's decode {ctx}"
    );
    let [rp, rl, rk] = residuals(&got, t.lz, t.k);
    worst.see(&format!("{} Σp", R::LABEL), rp);
    worst.see(&format!("{} L_z", R::LABEL), rl);
    worst.see(&format!("{} K", R::LABEL), rk);
    assert!(rp <= TOL_P, "Σp is {rp} eps of Σ|p| {ctx}");
    assert!(rl <= TOL_L, "L_z(p) misses L_z by {rl} eps {ctx}");
    assert!(rk <= TOL_K, "K(p) misses K* by {rk} eps {ctx}");
}

/// The momentum block's record of the construction: its particle momenta are the IC's, `p_λ = p₂` and `q = (p_ρ,
/// p_λ)` exactly, and §0.3's map takes `(p_ρ, p_λ)` back to `p` within rounding.
fn check_jacobi<R: Q>(d: &Decoded<R>, ctx: &str) {
    let mo = &d.momentum;
    assert_eq!(mo.p, d.ic.p, "the momentum block's p is not the IC's {ctx}");
    assert_eq!(mo.p_lambda, d.ic.p[2], "p_λ is not p₂ {ctx}");
    assert_eq!(
        mo.q,
        [mo.p_rho[0], mo.p_rho[1], mo.p_lambda[0], mo.p_lambda[1]],
        "q is not (p_ρ, p_λ) {ctx}"
    );
    let back = kernel::decode::momentum::to_particles(&d.masses, mo.p_rho, mo.p_lambda);
    let size: f64 = d.ic.p.iter().flatten().map(|x| x.f().abs()).sum();
    for (b, p) in back.iter().flatten().zip(d.ic.p.iter().flatten()) {
        assert!(
            (b.f() - p.f()).abs() <= 4.0 * R::eps() * size.max(f64::MIN_POSITIVE),
            "(p_ρ, p_λ) do not map back to p: {:e} for {:e} {ctx}",
            b.f(),
            p.f()
        );
    }
}

fn check_decode<R: Q>(decode: ChartDecodeFn<R>, worst: &Worst) {
    for z in FROZEN {
        for s in [0.0, 1e-6, 0.25, 0.5, 0.75, 1.0 - 1e-6, 1.0] {
            for t in [0.0, 1e-3, 0.1, 0.5, 0.9, 1.0] {
                check_decode_point(decode, z, R::of(s), R::of(t), worst);
            }
        }
    }
    let strategy = (0.0f64..=1.0, 0.0f64..=1.0, 0usize..FROZEN.len());
    prop::run(&strategy, |(u, v, z)| {
        check_decode_point(decode, FROZEN[z], R::of(u), R::of(v), worst);
        Ok(())
    });
}

#[test]
fn invariant_chart_decode_targets() {
    let worst = Worst::default();
    check_decode::<f64>(kernel_decode::<f64>, &worst);
    check_decode::<f32>(kernel_decode::<f32>, &worst);
    worst.print("invariant_chart_decode_targets");
}

/// A decode whose construction reads `z_β` for `z_α`: the seed differs wherever their signs do.
#[cfg(feature = "controls")]
fn z_beta_seed<R: Q>(c: &InvariantLE<R>, u: R, v: R) -> (Decoded<R>, Outcome) {
    let (mut d, outcome) = kernel_decode(c, u, v);
    let t = c.0.targets(u, v);
    d.ic.p = construct(&d.masses, &d.config, c.0.z_cfg[1], t.lz, t.k).p;
    (d, outcome)
}

negative_control!(
    invariant_chart_decode_targets,
    "a decode whose seed reads z_β is not the construction on the decoded z_α",
    expected = "is not the construction on the decoded z_α",
    check_decode::<f64>(z_beta_seed::<f64>, &Worst::default())
);

// ── invariant_chart_validate (R-26) ──────────────────────────────────────────────────────────────────────────────

/// A chart's `validate` under check.
type Validate = fn(&dyn Fn(f64, f64) -> ValidationResult, f64, f64) -> ValidationResult;

/// The chart's own `validate`.
fn own(v: &dyn Fn(f64, f64) -> ValidationResult, x: f64, y: f64) -> ValidationResult {
    v(x, y)
}

/// `validate` of a chart whose constraints are `low` (for `t < 0`) and `wide` (for `s ∉ [0, 1]`).
fn check_validate(
    validate: Validate,
    chart: &dyn Fn(f64, f64) -> ValidationResult,
    low: Constraint,
    wide: Constraint,
) {
    let v = |x: f64, y: f64| validate(chart, x, y);
    for s in [0.0, 1e-300, 0.3, 0.5, 1.0 - 1e-16, 1.0] {
        for t in [0.0, 1e-300, 0.4, 1.0, 1.5, 1e30] {
            assert_eq!(v(s, t), ValidationResult::Valid, "({s}, {t}) is not valid");
        }
    }
    for (s, t, want_s) in [
        (-0.1, 0.5, 0.0),
        (1.5, 0.5, 1.0),
        (1.0 + f64::EPSILON, 0.5, 1.0),
        (-1e-300, 0.0, 0.0),
        (1e30, 2.0, 1.0),
    ] {
        assert_eq!(
            v(s, t),
            ValidationResult::Project {
                u: want_s,
                v: t,
                failed: wide
            },
            "({s}, {t}): |L_z| > L_max is not projected onto the parabola"
        );
    }
    for (s, t, want_s) in [
        (0.4, -1e-300, 0.4),
        (0.4, -0.5, 0.4),
        (-2.0, -1.0, 0.0),
        (3.0, -1.0, 1.0),
    ] {
        assert_eq!(
            v(s, t),
            ValidationResult::Project {
                u: want_s,
                v: 0.0,
                failed: low
            },
            "({s}, {t}): t < 0 is not projected onto the apex"
        );
    }
    for (s, t, failed) in [
        (f64::NAN, 0.5, wide),
        (f64::INFINITY, 0.5, wide),
        (0.5, f64::NAN, low),
        (0.5, f64::NEG_INFINITY, low),
        (0.5, f64::INFINITY, low),
        (f64::NAN, f64::NAN, low),
    ] {
        assert_eq!(
            v(s, t),
            ValidationResult::Reject { failed },
            "({s}, {t}): a non-finite coordinate is not rejected"
        );
    }
}

fn check_validate_both(validate: Validate) {
    let (k0, g0) = defaults();
    let c = inv::<f64>(FROZEN[1], k0, g0);
    check_validate(
        validate,
        &|x, y| InvariantLE(c).validate(x, y),
        Constraint::LzEParabola,
        Constraint::LzEParabola,
    );
    check_validate(
        validate,
        &|x, y| InvariantLK(c).validate(x, y),
        Constraint::KNonNegative,
        Constraint::KAboveLzBound,
    );
}

#[test]
fn invariant_chart_validate() {
    check_validate_both(own);
}

/// A `validate` that projects `t > 1` down to the top edge, as if the axis's bound were a constraint.
#[cfg(feature = "controls")]
fn top_clamped(v: &dyn Fn(f64, f64) -> ValidationResult, x: f64, y: f64) -> ValidationResult {
    if y > 1.0 && y.is_finite() && (0.0..=1.0).contains(&x) {
        ValidationResult::Project {
            u: x,
            v: 1.0,
            failed: Constraint::LzEParabola,
        }
    } else {
        v(x, y)
    }
}

negative_control!(
    invariant_chart_validate,
    "a validate refusing the feasible t > 1 side fails",
    expected = "is not valid",
    check_validate_both(top_clamped)
);

// ── invariant_chart_forbids_energy_normalisation (chart_reference §0.6) ──────────────────────────────────────────

/// Each chart's flag, name and dump header, as `(flag, name, header)`.
fn check_flags(charts: &[(bool, &str, String)]) {
    for (want, (flag, name, header)) in ["InvariantLE", "InvariantLK"].iter().zip(charts) {
        assert!(*flag, "{name} does not forbid energy normalisation");
        assert_eq!(name, want, "the chart's name");
        assert_eq!(header, &format!("chart: {want}\n"), "{want}'s dump header");
    }
}

fn header<C: Chart>(c: &C) -> String {
    let mut s = String::new();
    write_dump_header(&mut s, c).expect("the header writes");
    s
}

#[test]
fn invariant_chart_forbids_energy_normalisation() {
    let c = inv::<f32>(FROZEN[0], 1.0, 2.0);
    let (le, lk) = (InvariantLE(c), InvariantLK(c));
    check_flags(&[
        (le.forbids_energy_normalisation(), le.name(), header(&le)),
        (lk.forbids_energy_normalisation(), lk.name(), header(&lk)),
    ]);
}

negative_control!(
    invariant_chart_forbids_energy_normalisation,
    "the latent chart's flag, false, in the (L_z, K) chart's place fails",
    expected = "does not forbid energy normalisation",
    {
        let c = inv::<f32>(FROZEN[0], 1.0, 2.0);
        let le = InvariantLE(c);
        let latent = kernel::chart::Latent::<f32> {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        };
        check_flags(&[
            (le.forbids_energy_normalisation(), le.name(), header(&le)),
            (
                latent.forbids_energy_normalisation(),
                "InvariantLK",
                "chart: InvariantLK\n".to_owned(),
            ),
        ])
    }
);

// ── invariant_chart_stage (REQ-CHART-001) ────────────────────────────────────────────────────────────────────────

/// A decode stage compute entry point, called natively.
type Entry = fn(UVec3, &DecodeUniforms, &[f32], &mut [u32], &mut [u32], &mut [u32]);

/// Runs `entry` natively over `uv`, three invocations past the last sample among them, with `chart`'s uniform, and checks each sample against the stage's `sample` for the
/// chart `V::Chart`, and the uniform's round trip.
fn check_stage<V: DecodeVariant>(entry: Entry, chart: V::Chart)
where
    V::Chart: PartialEq + core::fmt::Debug,
{
    let rows = chart.uniform();
    assert_eq!(
        V::Chart::from_uniform(&rows),
        chart,
        "the uniform does not round-trip the chart"
    );
    let uv: Vec<f32> = [
        [0.0, 0.0],
        [1.0, 1.0],
        [0.0, 0.7],
        [1.0, 0.3],
        [0.25, 0.9],
        [0.5, 0.5],
    ]
    .into_iter()
    .flatten()
    .collect();
    let n = uv.len() / 2;
    let uniforms = DecodeUniforms {
        chart: rows,
        flags: [flags(DecodeMode::Full), 0, 0, 0].into(),
    };
    let mut ic = vec![0; n * ic_words()];
    let mut tags = vec![u32::MAX; n];
    let mut descriptors = vec![0; n * descriptor_words()];
    for i in 0..n as u32 + 3 {
        entry(
            UVec3::new(i, 0, 0),
            &uniforms,
            &uv,
            &mut ic,
            &mut tags,
            &mut descriptors,
        );
    }
    for i in 0..n {
        let want = sample::<V::Chart, V::Links, f32>(&chart, uv[2 * i], uv[2 * i + 1]);
        let got = (
            read_ic(&ic, i),
            untag(tags[i]),
            read_descriptor(&descriptors, i),
        );
        assert!(
            got.0 == want.ic && got.1 == Ok(want.degenerate) && got.2 == want.descriptor,
            "sample {i}: the entry point's decode is not the chart's"
        );
    }
}

/// A chart of the defaults at the frozen `z`, as the kernel holds it.
fn stage_chart(z: [f64; 4]) -> Invariant<f32> {
    let (k0, g0) = defaults();
    inv::<f32>(z, k0, g0)
}

#[test]
fn invariant_chart_stage() {
    for z in FROZEN {
        check_stage::<InvariantLEDefaults>(invariant_le_defaults, InvariantLE(stage_chart(z)));
        check_stage::<InvariantLKDefaults>(invariant_lk_defaults, InvariantLK(stage_chart(z)));
    }
    assert_eq!(
        InvariantLEDefaults::ENTRY,
        "stage_decode_invariant_le_defaults"
    );
    assert_eq!(
        InvariantLKDefaults::ENTRY,
        "stage_decode_invariant_lk_defaults"
    );
}

negative_control!(
    invariant_chart_stage,
    "the latent chart's entry point on the (L_z, E) uniform is not the chart's decode",
    expected = "the entry point's decode is not the chart's",
    check_stage::<InvariantLEDefaults>(
        kernel::stage::decode::latent_defaults,
        InvariantLE(stage_chart(FROZEN[1]))
    )
);
