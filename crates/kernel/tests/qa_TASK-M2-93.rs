//! QA tests for the momentum seed of the construction (the expansion seed `w = r/√I` in force, the largest-norm rule
//! kept as a named legacy option) and its conversion of every chart image, written from the requirements and the
//! sources they cite (chart_reference §2.2, dd_decoder §3.4), not from the implementation.
//!
//! - `qa_m293_seed_selection_in_force`: under the seed in force the construction's direction is
//!   `w = r/√I` and its momenta are `pᵢ = mᵢ(ω·J rᵢ + a·rᵢ/√I)`, `ω = L_z/I`, `a = √(2(K* − L_z²/(2I)))`, the whole
//!   prediction formed here from the configuration's own `I` and the targets (dd_decoder §3.4's formula), at f64 and
//!   f32, over decoded configurations on both sides of `z_α = 0`.
//! - `qa_m293_seed_selection_unreachable`: `NO_QUALIFYING_SEED` is unreachable for `I > 0`, at every scale a float
//!   holds (subnormal to near overflow, f64 and f32), and `w` stays `r/√I` there; the control is the legacy
//!   largest-norm rule on the same configurations, which does reach it at a small scale (its `ε_w` is absolute).
//! - `qa_m293_legacy_*` (the largest-norm rule, the named legacy option): several seeds qualify → the largest, not the
//!   first; an exact tie → the earlier; `ρ = 0` (bodies 0 and 1 coincident) → a fallback, not `DEGENERATE`;
//!   `DEGENERATE` exactly where no norm is strictly above `ε_w`.
//! - `qa_m293_seed_conversion_every_chart`: on every chart image that runs the construction (the
//!   invariant charts at three pins, f64 and f32, their linear mapping, the mixed-energy α × E and α × K, the Burrau
//!   `(ν, K)`, `θ × K` and `θ × L_z` charts), every untagged pixel is homographic and expanding, read off the decoded
//!   `(m, r, p)` with no knowledge of the chart: `v = ω·J r + λ·r`, `ω = L/I`, `λ = +√(2(K − L²/(2I))/I)`.
//! - `qa_m293_seed_conversion_seam`: the task's seam measurement, as written: rows of the `(L_z, E)` chart tilted
//!   toward `z_α` across `z_α = 0`, `‖Δv‖_m` between neighbouring pixels, the criterion "the pair across the line is no
//!   larger than the largest neighbouring difference within 8 pixels of it on either side". Its control, the legacy
//!   largest-norm rule, must fail it, else the result is void; that is asserted. That criterion under the seed in force
//!   is printed with its outcome, not asserted. What is asserted is the measurement's explanation, from outside the
//!   code: the pair across the line over its largest neighbour is exactly the ratio of the chord lengths of the
//!   decoded `α` steps (a seam-free start moves only with its configuration), so no seed term is present.
//! - `qa_m293_seed_conversion_lagrange`: at the invariant charts' default pin, `z = 0`, the masses are
//!   equal and the configuration equilateral; every pixel starts homographic and its accelerations are `−κ·rᵢ` with one
//!   `κ` for all three bodies, so the start is an exact homographic orbit and its break-up is set by rounding (labelled
//!   so in the output).
//!
//! **Tolerances.** Rounding bounds only: a few hundred roundings of the quantity's own size per precision (`Q::tol`);
//! where a value is formed as `√(K − K_min)` from rounded `K` and `K_min`, the bound is the square root of that
//! (`Q::root_tol`), as cancellation sets it. No threshold reads the legacy prediction `2√(K* − K_min)`.
//!
//! **Domain** (pitfalls §10): the stateless decode, its inputs held fixed; no trajectory.
//!
//! Each test registers the negative control that must make it fail.

use std::f64::consts::FRAC_PI_4;

use kernel::chart::invariant::{self as inv_chart, Invariant, InvariantChart};
use kernel::chart::mixed::Alpha;
use kernel::chart::physical::{AxisE, AxisK};
use kernel::chart::{
    Burrau, BurrauThetaK, BurrauThetaLz, Chart, InvariantLE, InvariantLK, Mapped, Mapping,
    MixedEnergy,
};
use kernel::decode::config::{self, Config};
use kernel::decode::invariant::{
    choose, construct_by_norms, construct_with, Construction, ExactSign, Outcome, Seed, Seeding,
};
use kernel::decode::mass::{self, Masses};
use kernel::decode::{Decoded, Ic, Latent};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, LinkParams, LinkReal};
use kernel::payload::PayloadReal;
use kernel::real::IeeeBits;
use validation::negative_control;

// ── Precisions and helpers ───────────────────────────────────────────────────────────────────────────────────────

trait Q: LinkReal + PayloadReal + IeeeBits + ExactSign + Copy {
    const LABEL: &'static str;
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    /// A few hundred roundings, relative.
    fn tol() -> f64;
    /// The square root of [`Q::tol`]: a quantity formed as `√(K − K_min)` from rounded inputs.
    fn root_tol() -> f64 {
        Self::tol().sqrt()
    }
    /// `2^e` at this precision, exact.
    fn two_to_the(e: i32) -> Self;
}

impl Q for f64 {
    const LABEL: &'static str = "f64";
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn tol() -> f64 {
        256.0 * f64::EPSILON
    }
    fn two_to_the(e: i32) -> Self {
        // Two normal factors, so a subnormal result is the exact power of two.
        let h = e / 2;
        2f64.powi(h) * 2f64.powi(e - h)
    }
}

impl Q for f32 {
    const LABEL: &'static str = "f32";
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        f64::from(self)
    }
    fn tol() -> f64 {
        256.0 * f64::from(f32::EPSILON)
    }
    fn two_to_the(e: i32) -> Self {
        let h = e / 2;
        2f32.powi(h) * 2f32.powi(e - h)
    }
}

/// A small deterministic generator (xorshift64*), uniform on `[lo, hi)`.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

/// Masses and canonical-frame configuration of `(z_α, z_β, z_μ₁, z_μ₂)` under the default links.
fn decoded<R: Q>(z: [f64; 4]) -> (Masses<R>, Config<R>) {
    let masses = mass::decode::<Defaults, R>([R::of(z[2]), R::of(z[3])]);
    let config = config::decode::<Defaults, R>([R::of(z[0]), R::of(z[1])], &masses);
    (masses, config)
}

/// `I = Σ mᵢ‖rᵢ‖²` in f64.
fn inertia(m: [f64; 3], r: &[[f64; 2]; 3]) -> f64 {
    (0..3)
        .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
        .sum()
}

fn wide2<R: Q>(v: [[R; 2]; 3]) -> [[f64; 2]; 3] {
    v.map(|x| x.map(Q::f))
}

/// The fixtures' controls: random over `z_α ∈ [−4, 4]` (both seeds the legacy largest-norm rule picks), `z_β`, unequal
/// masses, plus `z_α` at and beside 0.
fn controls() -> Vec<[f64; 4]> {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut zs: Vec<[f64; 4]> = (0..160)
        .map(|_| {
            [
                rng.range(-4.0, 4.0),
                rng.range(-3.0, 3.0),
                rng.range(-2.5, 2.5),
                rng.range(-2.5, 2.5),
            ]
        })
        .collect();
    for za in [-1e-7, 0.0, 1e-7] {
        zs.push([za, 0.4, 0.3, -0.6]);
    }
    zs
}

// ── qa_m293_seed_selection_in_force ──────────────────────────────────────────────────────────────────────────────

/// A construction on a decoded configuration: masses, configuration, `z_α`, `L_z`, `K*`.
type Build<R> = fn(&Masses<R>, &Config<R>, R, R, R) -> Construction<R>;

fn in_force<R: Q>(m: &Masses<R>, c: &Config<R>, za: R, lz: R, k: R) -> Construction<R> {
    construct_with(m, c, Seeding::Expansion, za, lz, k)
}

#[cfg(feature = "controls")]
fn legacy_rule<R: Q>(m: &Masses<R>, c: &Config<R>, za: R, lz: R, k: R) -> Construction<R> {
    construct_with(m, c, Seeding::LargestNorm, za, lz, k)
}

/// dd_decoder §3.4 with the expansion seed: `w = r/√I`, `pᵢ = mᵢ(ω·J rᵢ + a·wᵢ)`, `ω = L_z/I`,
/// `a = √(2(K* − L_z²/(2I)))`, from the configuration's own `I`; seed the expansion, outcome feasible.
fn check_in_force<R: Q>(build: Build<R>) {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let mut checked = 0;
    for z in controls() {
        let (masses, config) = decoded::<R>(z);
        let m = masses.m.map(Q::f);
        let r = wide2(config.r);
        let i_ = inertia(m, &r);
        for _ in 0..4 {
            let lz = R::of(rng.range(-1.5, 1.5));
            let extra = rng.range(0.05, 3.0);
            let k = R::of(lz.f() * lz.f() / (2.0 * i_) + extra);
            let c = build(&masses, &config, R::of(z[0]), lz, k);
            let (lz, k) = (lz.f(), k.f());
            assert_eq!(
                c.seed,
                Some(Seed::Expansion),
                "z = {z:?} ({}): the seed taken is not the expansion seed w = r/√I",
                R::LABEL
            );
            assert_eq!(c.outcome, Outcome::Feasible, "z = {z:?}: not feasible");
            let omega = lz / i_;
            let a = (2.0 * (k - lz * lz / (2.0 * i_))).sqrt();
            let scale = lz.abs() + (2.0 * k).sqrt();
            for b in 0..3 {
                let w = [r[b][0] / i_.sqrt(), r[b][1] / i_.sqrt()];
                let got_w = c.w[b].map(Q::f);
                for x in 0..2 {
                    assert!(
                        (got_w[x] - w[x]).abs() <= R::tol(),
                        "z = {z:?} ({}): w[{b}] = {got_w:?} is not the expansion seed w = r/√I = {w:?}",
                        R::LABEL
                    );
                }
                let v = [-omega * r[b][1] + a * w[0], omega * r[b][0] + a * w[1]];
                let p = c.p[b].map(Q::f);
                for x in 0..2 {
                    assert!(
                        (p[x] - m[b] * v[x]).abs() <= R::tol() * scale,
                        "z = {z:?}, L_z = {lz}, K* = {k} ({}): p[{b}] = {p:?} is not mᵢ(ω·J rᵢ + a·rᵢ/√I), the \
                         expansion seed's",
                        R::LABEL
                    );
                }
            }
            checked += 1;
        }
    }
    assert!(checked >= 600);
}

#[test]
fn qa_m293_seed_selection_in_force() {
    check_in_force::<f64>(in_force);
    check_in_force::<f32>(in_force);
}

negative_control!(
    qa_m293_seed_selection_in_force,
    "R-82's legacy rule, the named option, is not the seed in force",
    expected = "is not the expansion seed",
    check_in_force::<f64>(legacy_rule)
);

// ── qa_m293_seed_selection_unreachable ───────────────────────────────────────────────────────────────────────────

/// The construction at L_z = 0 on a configuration: masses, configuration, `K*`.
type Plain<R> = fn(&Masses<R>, &Config<R>, R) -> Construction<R>;

fn plain_in_force<R: Q>(m: &Masses<R>, c: &Config<R>, k: R) -> Construction<R> {
    construct_with(m, c, Seeding::Expansion, R::zero(), R::zero(), k)
}

#[cfg(feature = "controls")]
fn plain_legacy<R: Q>(m: &Masses<R>, c: &Config<R>, k: R) -> Construction<R> {
    construct_by_norms(m, c, R::zero(), k)
}

/// `config` with every position multiplied by `s` (a power of two).
fn scaled<R: Q>(config: &Config<R>, s: R) -> Config<R> {
    let mul = |v: [R; 2]| [v[0] * s, v[1] * s];
    Config {
        alpha: config.alpha,
        beta: config.beta,
        rho_t: mul(config.rho_t),
        lambda_t: mul(config.lambda_t),
        rho: mul(config.rho),
        lambda: mul(config.lambda),
        r: config.r.map(mul),
    }
}

/// At every exponent in `exps`, on every fixture with a nonzero position (`I > 0`), the construction takes a seed,
/// is not `DEGENERATE`, and, where the scaling is exact, its `w` is `r/√I` of the fixture's positions.
fn check_unreachable<R: Q>(build: Plain<R>, exps: &[i32]) {
    let k = R::of(0.5);
    for z in controls().into_iter().step_by(4) {
        let (masses, config) = decoded::<R>(z);
        let m = masses.m.map(Q::f);
        for &e in exps {
            let c_s = scaled(&config, R::two_to_the(e));
            if c_s.r.iter().flatten().all(|x| x.f() == 0.0) {
                continue;
            }
            // The stored positions back at unit scale: exact, as both factors are powers of two in f64's range.
            let back = 2f64.powi(-e / 2) * 2f64.powi(-e + e / 2);
            let r: [[f64; 2]; 3] = c_s.r.map(|v| v.map(|x| x.f() * back));
            let i_ = inertia(m, &r);
            let c = build(&masses, &c_s, k);
            assert!(
                c.outcome != Outcome::Degenerate && c.seed.is_some(),
                "z = {z:?}, scale 2^{e} ({}): NO_QUALIFYING_SEED reached at I > 0",
                R::LABEL
            );
            // Where the scaling rounded (a subnormal position), the stored configuration is not the fixture's and its
            // CoM and L_z are not 0, so the projections move w off r/√I: only the seed is checked there.
            if r != wide2(config.r) {
                continue;
            }
            for (b, rb) in r.iter().enumerate() {
                let w = c.w[b].map(Q::f);
                for (x, &wx) in w.iter().enumerate() {
                    let want = rb[x] / i_.sqrt();
                    assert!(
                        (wx - want).abs() <= R::root_tol(),
                        "z = {z:?}, scale 2^{e} ({}): w[{b}] = {w:?} is not r/√I",
                        R::LABEL
                    );
                }
            }
        }
    }
}

const EXPS_64: [i32; 12] = [
    -1070, -1060, -1030, -600, -300, -40, -10, 0, 10, 40, 300, 1000,
];
const EXPS_32: [i32; 10] = [-148, -140, -130, -100, -40, -10, 0, 10, 40, 100];

#[test]
fn qa_m293_seed_selection_unreachable() {
    check_unreachable::<f64>(plain_in_force, &EXPS_64);
    check_unreachable::<f32>(plain_in_force, &EXPS_32);
}

negative_control!(
    qa_m293_seed_selection_unreachable,
    "R-82's rule, whose ε_w is absolute, reaches NO_QUALIFYING_SEED on a small configuration with I > 0",
    expected = "NO_QUALIFYING_SEED reached at I > 0",
    check_unreachable::<f64>(plain_legacy, &[-24])
);

// ── qa_m293_legacy_* (the largest-norm rule, the named legacy option) ─────────────────────────────────────────────

/// A seed rule on the four seeds' `‖w⁽²⁾‖²_m`, in seed order `(ρ,0), (0,λ), (Jρ,0), (0,Jλ)`.
type Rule = fn([f64; 4]) -> Option<Seed>;

fn eps_w() -> f64 {
    <f64 as ChartConstants>::EPS_W
}

/// Several seeds qualify: the largest is taken, not the first.
fn check_largest(rule: Rule) {
    for (norms, want) in [
        ([0.2, 0.9, 0.1, 0.05], Seed::Lambda),
        ([0.3, 0.1, 0.8, 0.2], Seed::JRho),
        ([0.5, 0.4, 0.3, 0.6], Seed::JLambda),
        ([0.6, 0.4, 0.3, 0.2], Seed::Rho),
        ([1e-9, 2e-9, 0.0, 3e-9], Seed::JLambda),
    ] {
        assert_eq!(
            rule(norms),
            Some(want),
            "norms {norms:?}: the seed taken is not the largest qualifying one"
        );
    }
}

#[test]
fn qa_m293_legacy_largest() {
    check_largest(choose::<f64>);
}

/// The first qualifying seed: the largest-norm rule mis-read.
#[cfg(feature = "controls")]
fn first(norms: [f64; 4]) -> Option<Seed> {
    let seeds = [Seed::Rho, Seed::Lambda, Seed::JRho, Seed::JLambda];
    (0..4).find(|&i| norms[i] > eps_w()).map(|i| seeds[i])
}

negative_control!(
    qa_m293_legacy_largest,
    "the first qualifying seed is not the largest",
    expected = "is not the largest qualifying one",
    check_largest(first)
);

/// An exact tie keeps the earlier seed in seed order.
fn check_tie(rule: Rule) {
    for (norms, want) in [
        ([0.7, 0.7, 0.1, 0.1], Seed::Rho),
        ([0.1, 0.6, 0.6, 0.6], Seed::Lambda),
        ([0.25; 4], Seed::Rho),
        ([0.0, 0.0, 0.4, 0.4], Seed::JRho),
    ] {
        assert_eq!(
            rule(norms),
            Some(want),
            "norms {norms:?}: an exact tie does not take the earlier seed"
        );
    }
}

#[test]
fn qa_m293_legacy_tie() {
    check_tie(choose::<f64>);
}

/// The largest, a tie taking the later seed.
#[cfg(feature = "controls")]
fn later_on_tie(norms: [f64; 4]) -> Option<Seed> {
    let seeds = [Seed::Rho, Seed::Lambda, Seed::JRho, Seed::JLambda];
    let mut best: Option<usize> = None;
    for i in 0..4 {
        if norms[i] > eps_w() && best.is_none_or(|b| norms[i] >= norms[b]) {
            best = Some(i);
        }
    }
    best.map(|i| seeds[i])
}

negative_control!(
    qa_m293_legacy_tie,
    "a rule that takes the later seed on a tie",
    expected = "does not take the earlier seed",
    check_tie(later_on_tie)
);

/// `ρ = 0`, the primary seed degenerate: the rule takes a fallback, not `DEGENERATE`; on norms and on the construction
/// of a configuration with bodies 0 and 1 coincident.
fn check_fallback(rule: Rule) {
    for norms in [
        [0.0, 0.6, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.3],
        [1e-12, 0.2, 0.0, 0.2],
    ] {
        let s = rule(norms);
        assert!(
            s.is_some() && s != Some(Seed::Rho),
            "norms {norms:?}: the primary degenerate gives {s:?}, not a fallback: DEGENERATE at ρ = 0"
        );
    }
    for zm in [[0.0, 0.0], [0.7, -1.1], [-1.5, 0.4]] {
        let masses = mass::decode::<Defaults, f64>(zm);
        for lambda in [[0.8, 0.3], [-0.2, 1.1], [0.0, 0.5]] {
            let config = config::from_jacobi(&masses, [0.0, 0.0], lambda);
            let c = construct_by_norms(&masses, &config, 0.2, 1.0);
            let s = rule(c.norms);
            assert!(
                s.is_some() && s != Some(Seed::Rho),
                "masses {:?}, ρ = 0, λ = {lambda:?}: the rule gives {s:?} on the norms {:?}, not a fallback: \
                 DEGENERATE at ρ = 0",
                masses.m,
                c.norms
            );
            assert!(
                c.outcome == Outcome::Feasible && c.seed.is_some() && c.seed != Some(Seed::Rho),
                "masses {:?}, ρ = 0, λ = {lambda:?}: the legacy construction gives {:?} / {:?}: DEGENERATE at ρ = 0",
                masses.m,
                c.outcome,
                c.seed
            );
        }
    }
}

#[test]
fn qa_m293_legacy_fallback() {
    check_fallback(choose::<f64>);
}

/// Only the primary seed is ever considered.
#[cfg(feature = "controls")]
fn primary_only(norms: [f64; 4]) -> Option<Seed> {
    (norms[0] > eps_w()).then_some(Seed::Rho)
}

negative_control!(
    qa_m293_legacy_fallback,
    "a rule with no fallback emits DEGENERATE at ρ = 0",
    expected = "DEGENERATE at ρ = 0",
    check_fallback(primary_only)
);

/// `DEGENERATE` fires exactly where no seed's norm is strictly above `ε_w`; on norms, and on a construction whose
/// configuration is all at the origin.
fn check_threshold(rule: Rule) {
    let e = eps_w();
    let above = e * (1.0 + 1e-9);
    for (norms, want) in [
        ([e; 4], None),
        ([0.0; 4], None),
        ([0.5 * e, e, 0.0, e], None),
        ([0.0, 0.0, 0.0, above], Some(Seed::JLambda)),
        ([e, above, e, 0.0], Some(Seed::Lambda)),
        ([above, 0.0, 0.0, 0.0], Some(Seed::Rho)),
    ] {
        assert_eq!(
            rule(norms),
            want,
            "norms {norms:?} (ε_w = {e:e}): DEGENERATE is not exactly where no norm exceeds ε_w"
        );
    }
    let masses = mass::decode::<Defaults, f64>([0.2, -0.3]);
    let config = config::from_jacobi(&masses, [0.0, 0.0], [0.0, 0.0]);
    let c = construct_by_norms(&masses, &config, 0.0, 1.0);
    assert_eq!(
        (c.outcome, c.seed),
        (Outcome::Degenerate, None),
        "every position 0: DEGENERATE is not exactly where no norm exceeds ε_w"
    );
}

#[test]
fn qa_m293_legacy_threshold() {
    check_threshold(choose::<f64>);
}

/// `≥ ε_w` in place of `> ε_w`.
#[cfg(feature = "controls")]
fn at_least(norms: [f64; 4]) -> Option<Seed> {
    let seeds = [Seed::Rho, Seed::Lambda, Seed::JRho, Seed::JLambda];
    let mut best: Option<usize> = None;
    for i in 0..4 {
        if norms[i] >= eps_w() && best.is_none_or(|b| norms[i] > norms[b]) {
            best = Some(i);
        }
    }
    best.map(|i| seeds[i])
}

negative_control!(
    qa_m293_legacy_threshold,
    "a norm equal to ε_w taken as qualifying",
    expected = "DEGENERATE is not exactly where no norm exceeds ε_w",
    check_threshold(at_least)
);

// ── qa_m293_seed_conversion_every_chart ──────────────────────────────────────────────────────────────────────────

/// `v = p/m`.
fn velocities(ic: &Ic<f64>) -> [[f64; 2]; 3] {
    std::array::from_fn(|i| [ic.p[i][0] / ic.m[i], ic.p[i][1] / ic.m[i]])
}

/// The residual `‖v − ω·J r − λ·r‖_m / ‖v‖_m` with `ω = L/I` and `λ = +√(2(K − L²/(2I))/I)`, all read off the state;
/// and `λ·√I / ‖v‖_m`, the share of the motion in the dilation.
fn expansion_residual(ic: &Ic<f64>) -> (f64, f64) {
    let (m, r) = (ic.m, ic.r);
    let v = velocities(ic);
    let i_ = inertia(m, &r);
    let l: f64 = (0..3)
        .map(|i| m[i] * (r[i][0] * v[i][1] - r[i][1] * v[i][0]))
        .sum();
    let k2: f64 = (0..3)
        .map(|i| m[i] * (v[i][0] * v[i][0] + v[i][1] * v[i][1]))
        .sum();
    let omega = l / i_;
    let lambda = ((k2 - l * l / i_).max(0.0) / i_).sqrt();
    let off: f64 = (0..3)
        .map(|i| {
            let e = [
                v[i][0] - (-omega * r[i][1] + lambda * r[i][0]),
                v[i][1] - (omega * r[i][0] + lambda * r[i][1]),
            ];
            m[i] * (e[0] * e[0] + e[1] * e[1])
        })
        .sum();
    let size = k2.sqrt().max(f64::MIN_POSITIVE);
    (off.sqrt() / size, lambda * i_.sqrt() / size)
}

fn state<R: PayloadReal + Q>(d: &Decoded<R>) -> Option<Ic<f64>> {
    d.degenerate.is_none().then(|| Ic {
        m: d.ic.m.map(Q::f),
        r: wide2(d.ic.r),
        p: wide2(d.ic.p),
    })
}

/// An image: name, residual tolerance, decode at `(u, v)` (`None` where tagged).
type Image = (String, f64, Box<dyn Fn(f64, f64) -> Option<Ic<f64>>>);

/// Every untagged pixel of a 25 × 25 grid over `[0, 1]²` of each image starts homographic and expanding, to the
/// image's tolerance; at least a quarter of each image's pixels carry real dilation (`K* > K_min`).
fn check_every_chart(images: &[Image]) {
    for (name, tol, decode) in images {
        let (mut systems, mut dilating) = (0, 0);
        for i in 0..=24 {
            for j in 0..=24 {
                let (u, v) = (f64::from(i) / 24.0, f64::from(j) / 24.0);
                let Some(ic) = decode(u, v) else { continue };
                systems += 1;
                let (off, share) = expansion_residual(&ic);
                assert!(
                    off <= *tol,
                    "{name}: the pixel ({u}, {v}) does not start homographic and expanding: residual {off:e}"
                );
                dilating += usize::from(share > 0.01);
            }
        }
        println!("{name}: {systems} untagged pixels, {dilating} with K* > K_min, each homographic and expanding");
        assert!(
            4 * dilating >= 25 * 25,
            "{name}: only {dilating} pixels with K* > K_min; the image does not test the seed"
        );
    }
}

const PINS: [[f64; 4]; 3] = [
    [0.0, 0.0, 0.0, 0.0],
    [0.45, -0.8, 0.6, -0.2],
    [-1.2, 1.7, -0.9, 0.5],
];

fn invariant_at<R: Q>(pin: [f64; 4]) -> Invariant<R> {
    Invariant {
        z_cfg: [R::of(pin[0]), R::of(pin[1])],
        z_mass: [R::of(pin[2]), R::of(pin[3])],
        k_max: R::of(1.0),
        gamma_k: R::of(2.0),
    }
}

/// The invariant `(L_z, E)` chart's decode under `seeding`, f64.
fn le_seeded(
    chart: InvariantLE<f64>,
    seeding: Seeding,
) -> Box<dyn Fn(f64, f64) -> Option<Ic<f64>>> {
    Box::new(move |u, v| {
        let z = chart.map::<f64>(u, v).z;
        let d = inv_chart::decode_seeded::<Defaults, f64, _>(
            &chart,
            u,
            v,
            &z,
            &LinkParams::DEFAULT,
            seeding,
        );
        state(&d.0)
    })
}

fn every_chart() -> Vec<Image> {
    let root64 = <f64 as Q>::root_tol();
    let root32 = <f32 as Q>::root_tol();
    let mut out: Vec<Image> = Vec::new();
    for pin in PINS {
        let le = InvariantLE(invariant_at::<f64>(pin));
        let lk = InvariantLK(invariant_at::<f64>(pin));
        let le32 = InvariantLE(invariant_at::<f32>(pin));
        let lk32 = InvariantLK(invariant_at::<f32>(pin));
        let linear = Mapped::<_, Defaults>::new(le, [Mapping::Linear, Mapping::Linear]);
        out.push((
            format!("(L_z, E) at {pin:?}"),
            root64,
            Box::new(move |u, v| state(&le.decode::<Defaults, f64>(u, v))),
        ));
        out.push((
            format!("(L_z, K) at {pin:?}"),
            root64,
            Box::new(move |u, v| state(&lk.decode::<Defaults, f64>(u, v))),
        ));
        out.push((
            format!("(L_z, E) at {pin:?}, f32"),
            root32,
            Box::new(move |u, v| state(&le32.decode::<Defaults, f32>(u as f32, v as f32))),
        ));
        out.push((
            format!("(L_z, K) at {pin:?}, f32"),
            root32,
            Box::new(move |u, v| state(&lk32.decode::<Defaults, f32>(u as f32, v as f32))),
        ));
        out.push((
            format!("(L_z, E) at {pin:?}, linear"),
            root64,
            Box::new(move |u, v| state(&linear.decode::<Defaults, f64>(u, v))),
        ));
    }
    let z0 = [-0.4, 0.9, 0.0, 0.0, 0.0, 0.0, -0.3, 0.5];
    for held in [0.0, -0.25] {
        let e: MixedEnergy<Alpha, AxisE, Defaults, f64> =
            MixedEnergy::new(z0, [0.15, 1.35], 1.0, 2.0, held, Mapping::Warped);
        let k: MixedEnergy<Alpha, AxisK, Defaults, f64> =
            MixedEnergy::new(z0, [0.15, 1.35], 1.0, 2.0, held, Mapping::Warped);
        out.push((
            format!("α × E, held L_z = {held}"),
            root64,
            Box::new(move |u, v| state(&e.decode::<Defaults, f64>(u, v))),
        ));
        out.push((
            format!("α × K, held L_z = {held}"),
            root64,
            Box::new(move |u, v| state(&k.decode::<Defaults, f64>(u, v))),
        ));
    }
    let nu_k = Burrau {
        nu_range: [1.0 / 32.0, 31.0 / 32.0],
        k_max: 1.0,
        gamma_k: 2.0,
    };
    out.push((
        "Burrau (ν, K)".into(),
        root64,
        Box::new(move |u, v| state(&nu_k.decode::<Defaults, f64>(u, v))),
    ));
    let theta_k = BurrauThetaK {
        theta_range: [0.1, FRAC_PI_4],
        k_max: 1.0,
        gamma_k: 2.0,
    };
    out.push((
        "Burrau θ × K".into(),
        root64,
        Box::new(move |u, v| state(&theta_k.decode::<Defaults, f64>(u, v))),
    ));
    let theta_lz = BurrauThetaLz {
        theta_range: [0.1, FRAC_PI_4],
        k: 0.3,
        lz_fraction: 0.6,
    };
    out.push((
        "Burrau θ × L_z".into(),
        root64,
        Box::new(move |u, v| state(&theta_lz.decode::<Defaults, f64>(u, v))),
    ));
    out
}

#[test]
fn qa_m293_seed_conversion_every_chart() {
    check_every_chart(&every_chart());
}

negative_control!(
    qa_m293_seed_conversion_every_chart,
    "the (L_z, E) image under R-82's legacy rule, at a pin with z_α > 0, does not start homographic",
    expected = "does not start homographic and expanding",
    check_every_chart(&[(
        "(L_z, E), legacy rule".into(),
        <f64 as Q>::root_tol(),
        le_seeded(InvariantLE(invariant_at::<f64>(PINS[1])), Seeding::LargestNorm),
    )])
);

// ── qa_m293_seed_conversion_seam ─────────────────────────────────────────────────────────────────────────────────

/// Pixels per row and the crossing pair's index: pixel `j` sits at `z_α = (j − 31.5)·h`, so `z_α = 0` lies between
/// pixels 31 and 32.
const COLUMNS: usize = 64;
const H: f64 = 1.0 / 64.0;
const LINE: usize = COLUMNS / 2 - 1;
/// The criterion's reach, the task's 8 pixels either side.
const SIDE: usize = 8;

/// The rows of the `(L_z, E)` chart measured, each with `K* > K_min` (asserted).
const ROWS: [(f64, f64); 5] = [
    (0.5, 0.5),
    (0.3, 0.7),
    (0.62, 0.85),
    (0.2, 0.95),
    (0.75, 0.6),
];

struct Row {
    /// `‖Δv‖_m` of each neighbouring pair.
    steps: Vec<f64>,
    /// The chord `2 sin(|Δα|/2)` of each pair's decoded `α` step.
    chords: Vec<f64>,
    /// `2√(K* − K_min)`, the legacy prediction, printed only.
    predicted: f64,
}

impl Row {
    fn across(&self) -> f64 {
        self.steps[LINE]
    }
    /// The largest neighbouring difference within [`SIDE`] pixels of the line on either side.
    fn beside(&self) -> f64 {
        (LINE - SIDE..=LINE + SIDE)
            .filter(|&j| j != LINE)
            .map(|j| self.steps[j])
            .fold(0.0, f64::max)
    }
    fn holds(&self) -> bool {
        self.across() <= self.beside()
    }
    fn beside_chord(&self) -> f64 {
        (LINE - SIDE..=LINE + SIDE)
            .filter(|&j| j != LINE)
            .map(|j| self.chords[j])
            .fold(0.0, f64::max)
    }
}

/// The measurement under `seeding`: the `(L_z, E)` chart at an unequal-mass pin with `z_α = 0`, tilted toward `z_α`
/// (the hidden coordinate 0) across each row.
fn measure(seeding: Seeding) -> Vec<Row> {
    let chart = InvariantLE(Invariant {
        z_cfg: [0.0, -0.8],
        z_mass: [0.6, -0.2],
        k_max: 1.0,
        gamma_k: 2.0,
    });
    ROWS.iter()
        .map(|&(u, v)| {
            let t = chart.invariant().targets::<f64>(u, v);
            assert!(
                t.k - t.lz * t.lz / 2.0 > 1e-3,
                "the row ({u}, {v}) has K* = K_min: no seam to see"
            );
            let pixels: Vec<(Ic<f64>, f64)> = (0..COLUMNS)
                .map(|j| {
                    let mut z = chart.map::<f64>(u, v).z.z;
                    z[0] += (j as f64 - 31.5) * H;
                    let (d, outcome) = inv_chart::decode_seeded::<Defaults, f64, _>(
                        &chart,
                        u,
                        v,
                        &Latent::new(z),
                        &LinkParams::DEFAULT,
                        seeding,
                    );
                    assert_eq!(outcome, Outcome::Feasible);
                    (state(&d).expect("a system"), d.config.alpha)
                })
                .collect();
            let steps = pixels
                .windows(2)
                .map(|w| {
                    let (a, b) = (velocities(&w[0].0), velocities(&w[1].0));
                    (0..3)
                        .map(|i| {
                            let d = [b[i][0] - a[i][0], b[i][1] - a[i][1]];
                            w[0].0.m[i] * (d[0] * d[0] + d[1] * d[1])
                        })
                        .sum::<f64>()
                        .sqrt()
                })
                .collect();
            let chords = pixels
                .windows(2)
                .map(|w| 2.0 * ((w[1].1 - w[0].1).abs() / 2.0).sin())
                .collect();
            Row {
                steps,
                chords,
                predicted: 2.0 * (t.k - t.lz * t.lz / 2.0).sqrt(),
            }
        })
        .collect()
}

fn report(rows: &[Row], what: &str) {
    for (row, (u, v)) in rows.iter().zip(ROWS) {
        println!(
            "{what}, row ({u}, {v}): across {:.9e}, largest beside {:.9e}, ratio {:.7}, criterion {}; the decoded α \
             chords' ratio {:.7}; legacy prediction 2√(K* − K_min) = {:.6}",
            row.across(),
            row.beside(),
            row.across() / row.beside(),
            if row.holds() { "holds" } else { "does not hold" },
            row.chords[LINE] / row.beside_chord(),
            row.predicted
        );
    }
}

/// The task's criterion's control: under `seeding` the criterion must fail on every row, else it cannot see a seam.
fn check_control_fires(rows: &[Row]) {
    assert!(
        rows.iter().all(|r| !r.holds()),
        "the control passes the criterion on a row: the criterion cannot see the seam, the result is void"
    );
}

/// No seed term: on every row the pair across the line over its largest neighbour is the ratio of the decoded `α`
/// chords, to rounding (a seam-free start moves with its configuration only: `‖Δv‖_m ∝ ‖Δr‖_m`).
fn check_no_seed_term(rows: &[Row], what: &str) {
    for (row, (u, v)) in rows.iter().zip(ROWS) {
        let got = row.across() / row.beside();
        let want = row.chords[LINE] / row.beside_chord();
        assert!(
            (got - want).abs() <= 1e-9,
            "{what}, row ({u}, {v}): the jump across z_α = 0 over its neighbours, {got:.9}, is not the configuration's \
             own α step ratio {want:.9}: a seed term crosses the line"
        );
    }
}

#[test]
fn qa_m293_seed_conversion_seam() {
    let now = measure(Seeding::Expansion);
    let before = measure(Seeding::LargestNorm);
    report(&now, "the seed in force");
    report(&before, "R-82's legacy rule (the control)");
    check_control_fires(&before);
    check_no_seed_term(&now, "the seed in force");
    let held = now.iter().all(Row::holds);
    println!(
        "qa_m293_seed_conversion_seam: the task's criterion {} under the seed in force (its reading is RQ-820's \
         (closed by R-1022), queued for the owner); the control fails it on every row; the step across the line is the \
         configuration's own α step, no seed term",
        if held { "holds" } else { "does not hold" }
    );
}

negative_control!(
    qa_m293_seed_conversion_seam,
    "under R-82's legacy rule a seed term crosses the line",
    expected = "a seed term crosses the line",
    check_no_seed_term(&measure(Seeding::LargestNorm), "R-82's legacy rule")
);

// ── qa_m293_seed_conversion_lagrange ─────────────────────────────────────────────────────────────────────────────

/// At `z = 0`: masses equal, configuration equilateral; every pixel of `images` starts homographic and expanding,
/// and its Newtonian accelerations (`G = 1`) are `−κ·rᵢ`, one `κ` for all bodies, so the motion stays homographic
/// in exact arithmetic and only rounding breaks it.
fn check_lagrange(images: &[Image]) {
    let (masses, config) = decoded::<f64>([0.0; 4]);
    for m in masses.m {
        assert!(
            (m - 1.0 / 3.0).abs() <= 4.0 * f64::EPSILON,
            "the default pin's masses {:?}",
            masses.m
        );
    }
    let r = config.r;
    let d = |a: usize, b: usize| ((r[a][0] - r[b][0]).powi(2) + (r[a][1] - r[b][1]).powi(2)).sqrt();
    let sides = [d(0, 1), d(1, 2), d(2, 0)];
    for s in sides {
        assert!(
            (s - sides[0]).abs() <= 8.0 * f64::EPSILON,
            "the default pin's sides {sides:?}"
        );
    }
    let mut count = 0;
    for (name, tol, decode) in images {
        for i in 0..=16 {
            for j in 0..=16 {
                let (u, v) = (f64::from(i) / 16.0, f64::from(j) / 16.0);
                let Some(ic) = decode(u, v) else { continue };
                let (off, _) = expansion_residual(&ic);
                assert!(
                    off <= *tol,
                    "{name}: the pixel ({u}, {v}) does not start homographic: residual {off:e}"
                );
                let acc: [[f64; 2]; 3] = std::array::from_fn(|a| {
                    let mut s = [0.0; 2];
                    for b in (0..3).filter(|&b| b != a) {
                        let dx = [ic.r[b][0] - ic.r[a][0], ic.r[b][1] - ic.r[a][1]];
                        let q = (dx[0] * dx[0] + dx[1] * dx[1]).powf(1.5);
                        s[0] += ic.m[b] * dx[0] / q;
                        s[1] += ic.m[b] * dx[1] / q;
                    }
                    s
                });
                let kappa = -(acc[0][0] * ic.r[0][0] + acc[0][1] * ic.r[0][1])
                    / (ic.r[0][0] * ic.r[0][0] + ic.r[0][1] * ic.r[0][1]);
                for (a, (acc_a, r_a)) in acc.iter().zip(ic.r).enumerate() {
                    for (ax, rx) in acc_a.iter().zip(r_a) {
                        assert!(
                            (ax + kappa * rx).abs() <= 1e-12 * kappa.abs(),
                            "{name}: at ({u}, {v}) body {a}'s acceleration is not −κ·r: no homographic orbit"
                        );
                    }
                }
                count += 1;
            }
        }
    }
    assert!(count > 0);
    println!(
        "qa_m293_seed_conversion_lagrange: at the invariant charts' default pin, the equal-mass Lagrange triangle, \
         every pixel ({count}) starts a homographic orbit, whose break-up is set by rounding (R-934)"
    );
}

fn lagrange_images(seeding: Seeding) -> Vec<Image> {
    let lk = InvariantLK(invariant_at::<f64>(PINS[0]));
    vec![
        (
            "(L_z, E) at the default pin".into(),
            <f64 as Q>::root_tol(),
            le_seeded(InvariantLE(invariant_at::<f64>(PINS[0])), seeding),
        ),
        (
            "(L_z, K) at the default pin".into(),
            <f64 as Q>::root_tol(),
            Box::new(move |u, v| {
                let z = lk.map::<f64>(u, v).z;
                state(
                    &inv_chart::decode_seeded::<Defaults, f64, _>(
                        &lk,
                        u,
                        v,
                        &z,
                        &LinkParams::DEFAULT,
                        seeding,
                    )
                    .0,
                )
            }),
        ),
    ]
}

#[test]
fn qa_m293_seed_conversion_lagrange() {
    check_lagrange(&lagrange_images(Seeding::Expansion));
}

negative_control!(
    qa_m293_seed_conversion_lagrange,
    "R-82's legacy rule at the Lagrange triangle takes (ρ, 0), not homographic",
    expected = "does not start homographic",
    check_lagrange(&lagrange_images(Seeding::LargestNorm))
);
