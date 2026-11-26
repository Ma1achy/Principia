//! The momentum seed: the expansion `w = r/√I` in force, the largest-norm rule kept as a named legacy option
//! (chart_reference §2.2, dd_decoder §2, §3.4).
//!
//! **`seed_selection`**: under the seed in force the construction takes the expansion `w = r/√I`
//! ([`construct_expansion`]) on every configuration, and `NO_QUALIFYING_SEED` is unreachable for `I > 0`, at every
//! scale a float holds, subnormal positions included; under the largest-norm rule, the named legacy option, a fixture
//! where several seeds qualify takes the largest, not the first, an exact tie the earlier, `ρ = 0` a fallback, not
//! `DEGENERATE`, and `DEGENERATE` fires only where no seed has `‖w⁽²⁾‖²_m > ε_w`.
//!
//! **`seed_conversion`**: every chart image that runs the construction, the invariant charts, their linear option, the
//! lone energy axes beside `α` and the Burrau family's `K` and `L_z` charts, starts every pixel from a homographic
//! state `vᵢ = (ω·J + λ)·rᵢ`, `λ = a/√I ≥ 0`, which is `w = r/√I` read off the decoded `(m, r, p)` with no knowledge of
//! the chart: the conversion each golden of these charts records. The seam measurement, on the kernel's `(L_z, E)`
//! chart tilted toward `z_α` so the line `z_α = 0` crosses the window, is judged by the explanation identity, written
//! before any run: every neighbouring pair of pixels, the pair across the line included, differs in decoded velocity by
//! `‖Δv‖_m = √(2K*)·2 sin(|Δα|/2)`, the configuration's own motion, to a relative 10⁻⁹. The control is the same
//! measurement under the legacy largest-norm rule, which must fail the identity at the pair across the line on every
//! row (else the result is void); its jump there is predicted to be `2√(K* − K_min)` (printed beside it, no threshold
//! reading it). The criterion first written, no pair across the line differing by more than the largest neighbouring
//! difference within 8 pixels of it on either side, is still printed and judges nothing: it does not hold though there
//! is no seam, an explained divergence, the step across the line being `1 + h²/4` times its neighbours at `h = 1/64`
//! from the `α` link's own warp. And at the invariant charts' default pin, the equal-mass Lagrange triangle (`z = 0`),
//! every pixel starts a homographic orbit, whose break-up is set by rounding: labelled so.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory. Every loop is over a fixed grid.
//!
//! Each test has a registered negative control naming the panic it expects.

use std::f64::consts::FRAC_PI_4;

use kernel::chart::invariant::{self, Invariant};
use kernel::chart::mixed::Alpha;
use kernel::chart::physical::{AxisE, AxisK};
use kernel::chart::tilt::{shift_controls, Shift, Tiltable};
use kernel::chart::{
    Burrau, BurrauThetaK, BurrauThetaLz, Chart, InvariantLE, InvariantLK, Mapped, Mapping,
    MixedEnergy,
};
use kernel::decode::config::{self, Config};
use kernel::decode::invariant::{
    choose, construct_by_norms, construct_expansion, Construction, ExactSign, Outcome, Seed,
    Seeding,
};
use kernel::decode::mass::{self, Masses};
use kernel::decode::{Decoded, Ic, Latent};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, LinkParams, LinkReal};
use kernel::payload::PayloadReal;
use kernel::real::IeeeBits;
use validation::negative_control;

/// A float the tests run at, f32 or f64.
trait Q: LinkReal + PayloadReal + IeeeBits + ExactSign + invariant::WarpReal + Least + Copy {
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    /// The relative tolerance of a continuous value at this precision: a few hundred roundings.
    fn tol() -> f64;
}

impl Q for f64 {
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn tol() -> f64 {
        1e-13
    }
}

impl Q for f32 {
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        f64::from(self)
    }
    fn tol() -> f64 {
        5e-6
    }
}

/// The masses and canonical-frame configuration of `z = (z_α, z_β, z_μ₁, z_μ₂)` under the default links.
fn decoded<R: Q>(z: [f64; 4]) -> (Masses<R>, Config<R>) {
    let masses = mass::decode::<Defaults, R>([R::of(z[2]), R::of(z[3])]);
    let config = config::decode::<Defaults, R>([R::of(z[0]), R::of(z[1])], &masses);
    (masses, config)
}

/// The fixtures' controls: a grid over `z_α` through 0 and either side, with unequal masses (PIT-9) and several `β`.
fn controls() -> Vec<[f64; 4]> {
    let mut zs = Vec::new();
    for &za in &[-3.0, -0.9, -0.2, -1e-9, 0.0, 1e-9, 0.2, 0.9, 3.0] {
        for &zb in &[-2.0, -0.5, 0.0, 0.7, 2.5] {
            for &zm in &[[0.0, 0.0], [0.2, 0.1], [-1.3, 0.8], [2.0, -2.5]] {
                zs.push([za, zb, zm[0], zm[1]]);
            }
        }
    }
    zs
}

/// Feasible targets `(L_z, K*)` at `R`, `K* ≥ L_z²/2`: on the floor (the least float the exact test finds feasible,
/// [`invariant::k_floor`]), just above it and well above it; each as its `R` value read in f64.
fn targets<R: Q>() -> Vec<(f64, f64)> {
    let mut t = Vec::new();
    for &lz in &[0.0, 0.3, -0.75, 1.5] {
        let l = R::of(lz);
        for &extra in &[0.0, 1e-3, 0.4, 3.0] {
            let k = if extra == 0.0 {
                invariant::k_floor(l)
            } else {
                R::of(lz * lz / 2.0 + extra)
            };
            t.push((l.f(), k.f()));
        }
    }
    t
}

/// `I = Σ mᵢ‖rᵢ‖²`, in f64.
fn inertia(m: [f64; 3], r: [[f64; 2]; 3]) -> f64 {
    (0..3)
        .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
        .sum()
}

/// A construction: masses, configuration, `z_α`, `L_z`, `K*`.
type Build<R> = fn(&Masses<R>, &Config<R>, R, R, R) -> Construction<R>;

fn in_force<R: Q>(m: &Masses<R>, c: &Config<R>, _za: R, lz: R, k: R) -> Construction<R> {
    construct_expansion(m, c, lz, k)
}

#[cfg(feature = "controls")]
fn legacy<R: Q>(m: &Masses<R>, c: &Config<R>, za: R, lz: R, k: R) -> Construction<R> {
    kernel::decode::invariant::construct(m, c, za, lz, k)
}

// ── seed_selection_expansion ────────────────────────────────────────────────────────────────────────────────────

/// On every fixture and target, `build` takes the expansion: seed [`Seed::Expansion`], feasible, `w = r/√I` and
/// `pᵢ = mᵢ(L_z·J rᵢ + a·rᵢ/√I)`, `a = √(2(K* − L_z²/2))`, to the precision's rounding.
fn check_expansion<R: Q>(build: Build<R>) {
    let mut counted = 0;
    for z in controls() {
        let (masses, config) = decoded::<R>(z);
        let m = masses.m.map(Q::f);
        let r = config.r.map(|x| x.map(Q::f));
        let root = inertia(m, r).sqrt();
        for (lz, k) in targets::<R>() {
            let c = build(&masses, &config, R::of(z[0]), R::of(lz), R::of(k));
            let at = format!("at z = {z:?}, (L_z, K*) = ({lz}, {k})");
            assert_eq!(c.outcome, Outcome::Feasible, "the outcome {at}");
            let a = c.a.f();
            let floor = (lz * lz).max(f64::MIN_POSITIVE);
            assert!(
                (a * a - 2.0 * (k - lz * lz / 2.0)).abs() <= 50.0 * R::tol() * (2.0 * k).max(floor),
                "a² is not 2(K* − L_z²/2) {at}: a = {a:e}"
            );
            for i in 0..3 {
                for x in 0..2 {
                    let want = r[i][x] / root;
                    let got = c.w[i][x].f();
                    assert!(
                        (got - want).abs() <= 50.0 * R::tol(),
                        "w is not r/√I {at}: w[{i}][{x}] = {got:e}, r/√I = {want:e}"
                    );
                    let jr = if x == 0 { -r[i][1] } else { r[i][0] };
                    let p = m[i] * (lz * jr + a * want);
                    assert!(
                        (c.p[i][x].f() - p).abs() <= 50.0 * R::tol() * (1.0 + a + lz.abs()),
                        "p is not m(L_z·J r + a·r/√I) {at}: p[{i}][{x}] = {:e}, want {p:e}",
                        c.p[i][x].f()
                    );
                }
            }
            assert_eq!(c.seed, Some(Seed::Expansion), "the seed {at}");
            counted += usize::from(a > 1e-3);
        }
    }
    assert!(counted > 1000, "{counted} constructions with a > 0");
}

#[test]
fn seed_selection_expansion() {
    check_expansion::<f64>(in_force::<f64>);
    check_expansion::<f32>(in_force::<f32>);
}

negative_control!(
    seed_selection_expansion,
    "R-82's legacy rule, decided on z_α, takes (ρ, 0) or (0, λ), not r/√I, and must fail",
    expected = "w is not r/√I",
    check_expansion::<f64>(legacy::<f64>)
);

// ── seed_selection_unreachable ──────────────────────────────────────────────────────────────────────────────────

/// `config` with every position scaled by `s` (the Jacobi vectors with them): `I` scaled by `s²`.
fn scale<R: Q>(config: &Config<R>, s: R) -> Config<R> {
    let by = |v: [R; 2]| [v[0] * s, v[1] * s];
    Config {
        rho: by(config.rho),
        lambda: by(config.lambda),
        r: config.r.map(by),
        ..*config
    }
}

/// The powers of two the positions are scaled by: from the subnormal range to near the largest float.
fn scales<R: Q>() -> Vec<f64> {
    let (lo, hi) = if R::of(1e-300).f() == 0.0 {
        (-140, 120)
    } else {
        (-1060, 1015)
    };
    (lo..=hi)
        .step_by(5)
        .map(|e: i32| 2f64.powi(e / 2) * 2f64.powi(e - e / 2))
        .collect()
}

/// `build` at `I > 0`, at every scale, never `DEGENERATE`: its seed qualifies, `w` is finite and of unit mass norm; a
/// lone non-zero position the least subnormal float qualifies too; and at `I = 0`, every position 0, `DEGENERATE`, its
/// momenta finite.
fn check_unreachable<R: Q>(build: fn(&Masses<R>, &Config<R>, R, R) -> Construction<R>) {
    let (lz, k) = (R::of(0.25), R::of(1.0));
    let mut configs = Vec::new();
    for z in [[0.3, -0.5, 0.2, 0.1], [-2.0, 1.0, -1.3, 0.8], [0.0; 4]] {
        let (masses, config) = decoded::<R>(z);
        for s in scales::<R>() {
            configs.push((
                masses,
                scale(&config, R::of(s)),
                format!("z = {z:?} scaled by {s:e}"),
            ));
        }
        let mut lone = scale(&config, R::zero());
        lone.r[1][0] = R::from_bits_least();
        configs.push((
            masses,
            lone,
            format!("z = {z:?}'s masses, r₁ = the least subnormal"),
        ));
    }
    for (masses, config, at) in &configs {
        let c = build(masses, config, lz, k);
        assert_ne!(c.outcome, Outcome::Degenerate, "DEGENERATE at I > 0, {at}");
        assert_eq!(c.seed, Some(Seed::Expansion), "the seed at I > 0, {at}");
        let n: f64 = (0..3)
            .map(|i| masses.m[i].f() * (c.w[i][0].f().powi(2) + c.w[i][1].f().powi(2)))
            .sum();
        assert!(
            (n - 1.0).abs() <= 50.0 * R::tol(),
            "‖w‖²_m = {n} at I > 0, {at}"
        );
    }
    let (masses, config) = decoded::<R>([0.3, -0.5, 0.2, 0.1]);
    let c = build(&masses, &scale(&config, R::zero()), lz, k);
    assert_eq!(c.outcome, Outcome::Degenerate, "the outcome at I = 0");
    assert_eq!(c.seed, None, "the seed at I = 0");
    assert!(
        c.p.iter().flatten().all(|x| x.f().is_finite()),
        "a non-finite momentum at I = 0"
    );
}

/// The least positive subnormal float, from its bits.
trait Least {
    fn from_bits_least() -> Self;
}

impl Least for f64 {
    fn from_bits_least() -> Self {
        f64::from_bits(1)
    }
}

impl Least for f32 {
    fn from_bits_least() -> Self {
        f32::from_bits(1)
    }
}

#[test]
fn seed_selection_unreachable() {
    check_unreachable::<f64>(construct_expansion::<f64>);
    check_unreachable::<f32>(construct_expansion::<f32>);
}

negative_control!(
    seed_selection_unreachable,
    "R-82's legacy rule on computed norms reads ε_w, not the scale, and must degenerate at a small I > 0",
    expected = "DEGENERATE at I > 0",
    check_unreachable::<f64>(construct_by_norms::<f64>)
);

// ── seed_selection_zero_norm ────────────────────────────────────────────────────────────────────────────────────

/// At `I = 0` the decision reads a finite 0: `build`'s expansion norm, [`Construction::norms`]`[0]`, is exactly 0 where
/// every position is 0 and where every body sits at one point off the origin (`I` about the CoM 0), not NaN, and the
/// outcome is `DEGENERATE`.
fn check_zero_norm<R: Q>(build: fn(&Masses<R>, &Config<R>, R, R) -> Construction<R>) {
    let (masses, config) = decoded::<R>([0.3, -0.5, 0.2, 0.1]);
    let zero = scale(&config, R::zero());
    let point = Config {
        r: [[R::of(1.25), R::of(-0.75)]; 3],
        rho: [R::zero(); 2],
        lambda: [R::zero(); 2],
        ..config
    };
    let even = Masses {
        m: [R::of(0.25), R::of(0.25), R::of(0.5)],
        m01: R::of(0.5),
        degenerate: None,
    };
    for (m, c, at) in [
        (&masses, &zero, "every position 0"),
        (&even, &point, "every body at one point"),
    ] {
        let k = build(m, c, R::of(0.25), R::of(1.0));
        assert!(
            k.norms[0].f() == 0.0,
            "the expansion's norm is not a finite 0 at I = 0, {at}: {:e}",
            k.norms[0].f()
        );
        assert_eq!(k.outcome, Outcome::Degenerate, "the outcome at I = 0, {at}");
    }
}

#[test]
fn seed_selection_zero_norm() {
    check_zero_norm::<f64>(construct_expansion::<f64>);
    check_zero_norm::<f32>(construct_expansion::<f32>);
}

/// The construction with `β_L`'s divisor unguarded, `L(w⁽¹⁾)/I` at the scaled `I` even where it is 0: its norm NaN
/// where every position is 0.
#[cfg(feature = "controls")]
fn unguarded(m: &Masses<f64>, c: &Config<f64>, lz: f64, k: f64) -> Construction<f64> {
    use kernel::decode::invariant::{angular, mass_norm2, scaled};
    let s = scaled(&c.r);
    let w = m.m;
    let com = [
        w[0] * s[0][0] + w[1] * s[1][0] + w[2] * s[2][0],
        w[0] * s[0][1] + w[1] * s[1][1] + w[2] * s[2][1],
    ];
    let w1 = s.map(|v| [v[0] - com[0], v[1] - com[1]]);
    let beta = angular(w, &s, &w1) / mass_norm2(w, &s);
    let w2: Vec<[f64; 2]> = (0..3)
        .map(|i| [w1[i][0] + beta * s[i][1], w1[i][1] - beta * s[i][0]])
        .collect();
    let mut out = construct_expansion(m, c, lz, k);
    out.norms[0] = mass_norm2(w, &[w2[0], w2[1], w2[2]]);
    out
}

negative_control!(
    seed_selection_zero_norm,
    "β_L's divisor unguarded gives a NaN norm where every position is 0, and must fail",
    expected = "is not a finite 0 at I = 0",
    check_zero_norm::<f64>(unguarded)
);

// ── seed_selection_coincident ───────────────────────────────────────────────────────────────────────────────────

/// The points the coincident fixtures put every body at.
const POINTS: [[f64; 2]; 4] = [[1.25, -0.75], [0.3, 0.7], [-1.1, 0.45], [0.01, -1.9]];

/// The latents whose decoded masses the coincident fixtures take: unequal, not summing to 1 exactly at every
/// precision.
const MASS_AT: [[f64; 4]; 3] = [
    [0.7, -0.4, 0.3, -0.6],
    [0.3, -0.5, 0.2, 0.1],
    [-2.1, 1.3, -1.2, 0.9],
];

/// Every body at one point, `I` about the CoM 0, with unequal decoded masses: `build`'s expansion norm,
/// [`Construction::norms`]`[0]`, is exactly 0 and the outcome `DEGENERATE`, at every point in [`POINTS`]. The fixture
/// is shown to reach the case: at some point the CoM `c = Σ mᵢrᵢ` at the scaled positions does not round back to the
/// point, so `rᵢ − c` is a residue, not 0.
fn check_coincident<R: Q>(build: fn(&Masses<R>, &Config<R>, R, R) -> Construction<R>) {
    let mut residue = 0;
    for z in MASS_AT {
        let (masses, config) = decoded::<R>(z);
        for p in POINTS {
            let at = [R::of(p[0]), R::of(p[1])];
            let point = Config {
                r: [at; 3],
                rho: [R::zero(); 2],
                lambda: [R::zero(); 2],
                ..config
            };
            let s = kernel::decode::invariant::scaled(&point.r);
            let m = masses.m;
            let c0 = m[0] * s[0][0] + m[1] * s[1][0] + m[2] * s[2][0];
            let c1 = m[0] * s[0][1] + m[1] * s[1][1] + m[2] * s[2][1];
            if c0 != s[0][0] || c1 != s[0][1] {
                residue += 1;
            }
            let k = build(&masses, &point, R::of(0.25), R::of(1.0));
            assert!(
                k.norms[0].f() == 0.0,
                "the expansion's norm is not an exact 0 with every body at one point ({}, masses at {z:?}, \
                 point {p:?}): {:e}",
                std::any::type_name::<R>(),
                k.norms[0].f()
            );
            assert_eq!(
                k.outcome,
                Outcome::Degenerate,
                "the outcome with every body at one point ({}, masses at {z:?}, point {p:?})",
                std::any::type_name::<R>()
            );
        }
    }
    assert!(
        residue > 0,
        "fixture: no point where the CoM rounds off it ({})",
        std::any::type_name::<R>()
    );
}

#[test]
fn seed_selection_coincident() {
    check_coincident::<f64>(construct_expansion::<f64>);
    check_coincident::<f32>(construct_expansion::<f32>);
}

/// The construction with `w⁽¹⁾ = rᵢ − c` taken as computed even where every body is at one point: its norm the
/// residue's.
#[cfg(feature = "controls")]
fn residual(m: &Masses<f64>, c: &Config<f64>, lz: f64, k: f64) -> Construction<f64> {
    use kernel::decode::invariant::{angular, mass_norm2, scaled};
    let s = scaled(&c.r);
    let w = m.m;
    let com = [
        w[0] * s[0][0] + w[1] * s[1][0] + w[2] * s[2][0],
        w[0] * s[0][1] + w[1] * s[1][1] + w[2] * s[2][1],
    ];
    let w1 = s.map(|v| [v[0] - com[0], v[1] - com[1]]);
    let inertia = mass_norm2(w, &s);
    let beta = angular(w, &s, &w1) / if inertia > 0.0 { inertia } else { 1.0 };
    let w2: Vec<[f64; 2]> = (0..3)
        .map(|i| [w1[i][0] + beta * s[i][1], w1[i][1] - beta * s[i][0]])
        .collect();
    let mut out = construct_expansion(m, c, lz, k);
    out.norms[0] = mass_norm2(w, &[w2[0], w2[1], w2[2]]);
    out
}

negative_control!(
    seed_selection_coincident,
    "rᵢ − c taken as computed leaves a rounding residue where every body is at one point, and must fail",
    expected = "is not an exact 0 with every body at one point",
    check_coincident::<f64>(residual)
);

// ── seed_selection_legacy ───────────────────────────────────────────────────────────────────────────────────────

/// The legacy largest-norm rule, the named option: `rule` on the seeds' norms. Several qualify: a decoded
/// configuration at `α > π/4` (`z_α = 1`), where every norm is above `ε_w` and `(0, λ)` is the largest, takes it, not
/// the first; an exact tie keeps the earlier seed; `ρ = 0` takes a fallback, not `DEGENERATE`; and over scales down
/// to all-zero positions, `DEGENERATE` fires exactly where no norm is above `ε_w`.
fn check_legacy(rule: fn([f64; 4]) -> Option<Seed>) {
    let eps = <f64 as ChartConstants>::EPS_W;
    let by_rule = |m: &Masses<f64>, c: &Config<f64>| {
        let out = construct_by_norms(m, c, 0.0, 0.5);
        let seed = rule(out.norms);
        let o = if seed.is_some() {
            Outcome::Feasible
        } else {
            Outcome::Degenerate
        };
        (out, seed, o)
    };
    let (masses, config) = decoded::<f64>([1.0, 0.4, 0.2, 0.1]);
    let (out, seed, _) = by_rule(&masses, &config);
    assert!(
        out.norms.iter().all(|n| *n > eps),
        "every seed qualifies: {:?}",
        out.norms
    );
    assert!(
        out.norms[1] > out.norms[0],
        "(0, λ) the largest: {:?}",
        out.norms
    );
    assert_eq!(seed, Some(Seed::Lambda), "several qualify: not the largest");
    assert_eq!(out.seed, seed, "construct_by_norms takes the rule's seed");

    assert_eq!(
        rule([0.5, 0.5, 0.25, 0.25]),
        Some(Seed::Rho),
        "a tie at the first: not the earlier"
    );
    assert_eq!(
        rule([0.1, 0.5, 0.5, 0.2]),
        Some(Seed::Lambda),
        "a tie at the second: not the earlier"
    );
    assert_eq!(
        rule([0.1, 0.2, 0.3, 0.3]),
        Some(Seed::JRho),
        "a tie at the third: not the earlier"
    );

    let equal = mass::decode::<Defaults, f64>([0.0, 0.0]);
    let (rho, lambda, r) = config::reconstruct(&equal, [0.0, 0.0], [0.0, 1.0]);
    let flat = Config {
        rho,
        lambda,
        r,
        ..config::decode::<Defaults, f64>([0.0, 0.0], &equal)
    };
    let (out, seed, o) = by_rule(&equal, &flat);
    assert!(
        out.norms[0] <= eps,
        "ρ = 0: the primary degenerates: {:?}",
        out.norms
    );
    assert_ne!(o, Outcome::Degenerate, "ρ = 0: DEGENERATE, not a fallback");
    assert_eq!(seed, Some(Seed::Lambda), "ρ = 0: the fallback");

    let (masses, config) = decoded::<f64>([0.3, -0.5, 0.2, 0.1]);
    let mut fired = [0, 0];
    for e in (-30..=0).chain([-600]) {
        let s = 2f64.powi(e);
        let c = scale(&config, s);
        let (out, _, o) = by_rule(&masses, &c);
        let none = out.norms.iter().all(|n| *n <= eps);
        assert_eq!(
            o == Outcome::Degenerate,
            none,
            "DEGENERATE fires only where no seed is above ε_w: {o:?} at scale {s:e}, norms {:?}",
            out.norms
        );
        fired[usize::from(none)] += 1;
    }
    let (out, _, o) = by_rule(&masses, &scale(&config, 0.0));
    assert_eq!(
        o,
        Outcome::Degenerate,
        "all positions 0: DEGENERATE fires {:?}",
        out.norms
    );
    assert!(
        fired[0] > 0 && fired[1] > 0,
        "both sides of ε_w reached: {fired:?}"
    );
}

#[test]
fn seed_selection_legacy() {
    check_legacy(choose::<f64>);
}

/// The first seed above `ε_w`, in seed order: not the largest-norm rule.
#[cfg(feature = "controls")]
fn first(norms: [f64; 4]) -> Option<Seed> {
    let seeds = [Seed::Rho, Seed::Lambda, Seed::JRho, Seed::JLambda];
    (0..4)
        .find(|&i| norms[i] > <f64 as ChartConstants>::EPS_W)
        .map(|i| seeds[i])
}

negative_control!(
    seed_selection_legacy,
    "the first qualifying seed, not the largest, must fail",
    expected = "not the largest",
    check_legacy(first)
);

#[cfg(feature = "controls")]
mod legacy_tie {
    use super::*;

    /// The largest, an exact tie taking the later seed: not the legacy rule, which keeps the earlier.
    fn later(norms: [f64; 4]) -> Option<Seed> {
        let seeds = [Seed::Rho, Seed::Lambda, Seed::JRho, Seed::JLambda];
        let eps = <f64 as ChartConstants>::EPS_W;
        let mut best = None;
        for i in 0..4 {
            if norms[i] > eps && best.is_none_or(|b: usize| norms[i] >= norms[b]) {
                best = Some(i);
            }
        }
        best.map(|i| seeds[i])
    }

    negative_control!(
        seed_selection_legacy,
        "an exact tie broken toward the later seed must fail",
        expected = "not the earlier",
        check_legacy(later)
    );
}

#[cfg(feature = "controls")]
mod legacy_fallback {
    use super::*;

    /// The primary seed or none: not the largest-norm rule.
    fn primary(norms: [f64; 4]) -> Option<Seed> {
        let largest = choose(norms);
        (norms[0] > <f64 as ChartConstants>::EPS_W).then_some(largest?)
    }

    negative_control!(
        seed_selection_legacy,
        "a rule degenerating where the primary seed does must fail at ρ = 0",
        expected = "DEGENERATE, not a fallback",
        check_legacy(primary)
    );
}

#[cfg(feature = "controls")]
mod legacy_threshold {
    use super::*;

    /// The rule at a threshold of 0 in place of `ε_w`: not the largest-norm rule.
    fn any(norms: [f64; 4]) -> Option<Seed> {
        let shifted = norms.map(|n| n + <f64 as ChartConstants>::EPS_W);
        choose(shifted).filter(|_| norms.iter().any(|n| *n > 0.0))
    }

    negative_control!(
        seed_selection_legacy,
        "a rule that qualifies any non-zero norm must fail where every norm is at most ε_w",
        expected = "DEGENERATE fires only where",
        check_legacy(any)
    );
}

// ── seed_selection_scaled ───────────────────────────────────────────────────────────────────────────────────────

/// Six components of distinct binades, `0` down to `−5`, the largest `1.5` in slot `at` (row-major), the rest in turn
/// after it.
fn spread(at: usize) -> [[f64; 2]; 3] {
    let base = [1.5, -0.75, 0.3125, -0.15625, 0.0859375, -0.0390625];
    let mut v = [0.0; 6];
    for (k, b) in base.iter().enumerate() {
        v[(at + k) % 6] = *b;
    }
    [[v[0], v[1]], [v[2], v[3]], [v[4], v[5]]]
}

/// `2^e` in f64, as two normal powers of two.
fn pow2(e: i32) -> f64 {
    2f64.powi(e / 2) * 2f64.powi(e - e / 2)
}

/// `scale` scales by a power of two read from the bits, exactly: wherever the largest component sits and at every
/// scale where nothing underflows, the six components come back as [`spread`]'s, the largest in `[1, 2)`; a lone least
/// subnormal comes back scaled by `2^limit` ([`ExactSign::doubling_limit`]); all zero stays zero.
fn check_scaled<R: Q>(scale: fn(&[[R; 2]; 3]) -> [[R; 2]; 3]) {
    let range = if R::of(1e-300).f() == 0.0 {
        -100..=100
    } else {
        -1000..=1000
    };
    for at in 0..6 {
        let want = spread(at);
        for e in range.clone().step_by(50) {
            let r = want.map(|v| v.map(|x| R::of(x * pow2(e))));
            let got = scale(&r).map(|v| v.map(Q::f));
            assert_eq!(
                got, want,
                "the scaled positions, largest in slot {at}, at 2^{e}"
            );
        }
    }
    let zero = [[R::zero(); 2]; 3];
    let mut lone = zero;
    lone[1][0] = R::from_bits_least();
    let got = scale(&lone).map(|v| v.map(Q::f));
    let mut want = [[0.0; 2]; 3];
    want[1][0] = R::from_bits_least().f() * pow2(R::doubling_limit());
    assert_eq!(got, want, "the scaled least subnormal");
    assert_eq!(
        scale(&zero).map(|v| v.map(Q::f)),
        [[0.0; 2]; 3],
        "all zero scaled"
    );
}

#[test]
fn seed_selection_scaled() {
    check_scaled::<f64>(kernel::decode::invariant::scaled::<f64>);
    check_scaled::<f32>(kernel::decode::invariant::scaled::<f32>);
}

/// The scale read off the first component alone: not the largest binade.
#[cfg(feature = "controls")]
fn first_binade<R: Q>(r: &[[R; 2]; 3]) -> [[R; 2]; 3] {
    let s = pow2(-r[0][0].binade().e);
    r.map(|v| v.map(|x| R::of(x.f() * s)))
}

negative_control!(
    seed_selection_scaled,
    "a scale read off the first component, not the largest, must fail where the largest is elsewhere",
    expected = "the scaled positions",
    check_scaled::<f64>(first_binade::<f64>)
);

#[cfg(feature = "controls")]
mod scaled_limit {
    use super::*;

    /// The scale with no hold at the limit: a subnormal largest component scaled into `[1, 2)`.
    fn unheld<R: Q>(r: &[[R; 2]; 3]) -> [[R; 2]; 3] {
        let e = r
            .iter()
            .flatten()
            .filter(|x| x.f() != 0.0)
            .map(|x| x.binade().e)
            .max()
            .unwrap_or(0);
        let s = pow2(-e);
        r.map(|v| v.map(|x| R::of(x.f() * s)))
    }

    negative_control!(
        seed_selection_scaled,
        "a scale not held at the limit takes the least subnormal into [1, 2) and must fail",
        expected = "the scaled least subnormal",
        check_scaled::<f64>(unheld::<f64>)
    );
}

// ── seed_selection_centred ──────────────────────────────────────────────────────────────────────────────────────

/// The expansion's `w⁽²⁾` and `‖w⁽²⁾‖²_m` of `(m, r)`.
type Expansion = fn([f64; 3], &[[f64; 2]; 3]) -> ([[f64; 2]; 3], f64);

/// `expand` off the CoM: masses `(¼, ¼, ½)` and positions whose CoM is `(0.375, 0.125)`, the largest component in
/// `[1, 2)` so no scale applies: `w⁽²⁾ = r − c` to rounding (the rotation term is 0 in exact arithmetic), of zero mass
/// drift. And three bodies at one point off the origin, `I` about the CoM 0: the construction is `DEGENERATE`, its
/// momenta finite.
fn check_centred(expand: Expansion, build: Build<f64>) {
    let m = [0.25, 0.25, 0.5];
    let r = [[1.5, 0.25], [-0.5, 1.25], [0.25, -0.5]];
    let c = [0.375, 0.125];
    let (w, n2) = expand(m, &r);
    for i in 0..3 {
        for x in 0..2 {
            assert!(
                (w[i][x] - (r[i][x] - c[x])).abs() <= 1e-15,
                "w⁽²⁾ is not r − c off the CoM: w[{i}][{x}] = {:e}",
                w[i][x]
            );
        }
    }
    let drift: Vec<f64> = (0..2)
        .map(|x| (0..3).map(|i| m[i] * w[i][x]).sum::<f64>())
        .collect();
    assert!(
        drift.iter().all(|d| d.abs() <= 1e-15),
        "w⁽²⁾ is not centred: Σ mᵢwᵢ = {drift:?}"
    );
    let want = inertia(m, r.map(|v| [v[0] - c[0], v[1] - c[1]]));
    assert!(
        (n2 - want).abs() <= 1e-15,
        "‖w⁽²⁾‖²_m = {n2:e}, I = {want:e}"
    );

    let masses = Masses {
        m,
        m01: 0.5,
        degenerate: None,
    };
    let point = Config {
        r: [[1.25, -0.75]; 3],
        rho: [0.0; 2],
        lambda: [0.0; 2],
        ..decoded::<f64>([0.0; 4]).1
    };
    let k = build(&masses, &point, 0.0, 0.0, 1.0);
    assert_eq!(
        k.outcome,
        Outcome::Degenerate,
        "three bodies at one point, I = 0 about the CoM"
    );
    assert_eq!(k.seed, None, "the seed at one point");
    assert!(
        k.p.iter().flatten().all(|x| x.is_finite()),
        "a non-finite momentum at one point"
    );
}

#[test]
fn seed_selection_centred() {
    check_centred(kernel::decode::invariant::expansion::<f64>, in_force::<f64>);
}

/// The expansion with the CoM drift left in.
#[cfg(feature = "controls")]
fn uncentred(m: [f64; 3], r: &[[f64; 2]; 3]) -> ([[f64; 2]; 3], f64) {
    (*r, kernel::decode::invariant::mass_norm2(m, r))
}

negative_control!(
    seed_selection_centred,
    "the expansion with its CoM drift left in must fail off the CoM",
    expected = "is not r − c off the CoM",
    check_centred(uncentred, in_force::<f64>)
);

#[cfg(feature = "controls")]
mod centred_point {
    use super::*;

    /// The construction taking the expansion wherever its norm is not negative, `I = 0` included.
    fn at_zero(m: &Masses<f64>, c: &Config<f64>, _za: f64, lz: f64, k: f64) -> Construction<f64> {
        let mut out = construct_expansion(m, c, lz, k);
        if out.norms[0] >= 0.0 && out.seed.is_none() {
            out.seed = Some(Seed::Expansion);
            out.outcome = Outcome::Feasible;
        }
        out
    }

    negative_control!(
        seed_selection_centred,
        "a seed taken at a zero norm must fail at one point",
        expected = "three bodies at one point",
        check_centred(kernel::decode::invariant::expansion::<f64>, at_zero)
    );
}

// ── seed_selection_rotation ─────────────────────────────────────────────────────────────────────────────────────

/// Fixed configurations, the largest component in `[1, 2)`, whose `L(w⁽¹⁾)` rounds to non-zero.
const ROUNDED: [([f64; 3], [[f64; 2]; 3]); 3] = [
    (
        [0.15686341393367842, 0.4279699277828045, 0.41516665828351706],
        [
            [1.122141276923135, -1.802897305297881],
            [0.8458746256789378, 1.81900676385371],
            [1.8936710159479202, 0.7647429708348676],
        ],
    ),
    (
        [0.23463629917483367, 0.6320793336091198, 0.13328436721604645],
        [
            [1.0098692060064807, -0.45904358749837404],
            [-1.6263850266970128, -1.6812672676876736],
            [0.28597168959487584, 0.9208829739273776],
        ],
    ),
    (
        [0.07079541544905885, 0.46417844949754605, 0.4650261350533951],
        [
            [-1.0532238724526979, 1.6701177414990638],
            [0.8228368883626889, -0.9994184775992525],
            [1.1521357749880012, -1.5506026882246315],
        ],
    ),
];

/// The rotation term removes `L(w⁽¹⁾)`'s rounding, `w⁽¹⁾ = r − c`: on each of [`ROUNDED`], `|L(w⁽²⁾)|` is at most half
/// `|L(w⁽¹⁾)|`, which is not 0.
fn check_rotation(expand: Expansion) {
    use kernel::decode::invariant::angular;
    for (m, r) in ROUNDED {
        let c = [
            m[0] * r[0][0] + m[1] * r[1][0] + m[2] * r[2][0],
            m[0] * r[0][1] + m[1] * r[1][1] + m[2] * r[2][1],
        ];
        let w1 = r.map(|v| [v[0] - c[0], v[1] - c[1]]);
        let l1 = angular(m, &r, &w1);
        assert!(l1 != 0.0, "L(w⁽¹⁾) is 0 at {r:?}");
        let l2 = angular(m, &r, &expand(m, &r).0);
        assert!(
            l2.abs() <= l1.abs() / 2.0,
            "the rotation term does not remove L(w⁽¹⁾): {l2:e} against {l1:e} at {r:?}"
        );
    }
}

#[test]
fn seed_selection_rotation() {
    check_rotation(kernel::decode::invariant::expansion::<f64>);
}

/// The expansion with no rotation term: `w⁽²⁾ = r − c`.
#[cfg(feature = "controls")]
fn unrotated(m: [f64; 3], r: &[[f64; 2]; 3]) -> ([[f64; 2]; 3], f64) {
    let c = [
        m[0] * r[0][0] + m[1] * r[1][0] + m[2] * r[2][0],
        m[0] * r[0][1] + m[1] * r[1][1] + m[2] * r[2][1],
    ];
    let w = r.map(|v| [v[0] - c[0], v[1] - c[1]]);
    (w, kernel::decode::invariant::mass_norm2(m, &w))
}

negative_control!(
    seed_selection_rotation,
    "the expansion with no rotation term keeps L(w⁽¹⁾) and must fail",
    expected = "the rotation term does not remove",
    check_rotation(unrotated)
);

// ── seed_selection_legacy_seeds ─────────────────────────────────────────────────────────────────────────────────

/// A construction with its seed named: masses, configuration, seed, `L_z`, `K*`.
type Named = fn(&Masses<f64>, &Config<f64>, Seed, f64, f64) -> Construction<f64>;

/// The legacy largest-norm option, each seed named: `build` at each of the four takes that seed, feasible, and the four
/// `w` differ pairwise, on a decoded configuration where all four qualify.
fn check_legacy_seeds(build: Named) {
    let (masses, config) = decoded::<f64>([0.3, -0.5, 0.2, 0.1]);
    let seeds = [Seed::Rho, Seed::Lambda, Seed::JRho, Seed::JLambda];
    let out: Vec<Construction<f64>> = seeds
        .iter()
        .map(|s| build(&masses, &config, *s, 0.25, 1.0))
        .collect();
    for a in 0..4 {
        for b in a + 1..4 {
            let gap = (0..3)
                .flat_map(|i| (0..2).map(move |x| (i, x)))
                .map(|(i, x)| (out[a].w[i][x] - out[b].w[i][x]).abs())
                .fold(0.0, f64::max);
            assert!(
                gap > 1e-3,
                "the seeds {:?} and {:?} give one w",
                seeds[a],
                seeds[b]
            );
        }
    }
    for (s, c) in seeds.iter().zip(&out) {
        assert_eq!(c.seed, Some(*s), "the seed taken");
        assert_eq!(c.outcome, Outcome::Feasible, "the outcome at {s:?}");
    }
}

#[test]
fn seed_selection_legacy_seeds() {
    check_legacy_seeds(kernel::decode::invariant::construct_seeded::<f64>);
}

/// `(Jρ, 0)` taken as `(ρ, 0)`: a seed's field not its own.
#[cfg(feature = "controls")]
fn rho_for_jrho(m: &Masses<f64>, c: &Config<f64>, s: Seed, lz: f64, k: f64) -> Construction<f64> {
    let s = if s == Seed::JRho { Seed::Rho } else { s };
    kernel::decode::invariant::construct_seeded(m, c, s, lz, k)
}

negative_control!(
    seed_selection_legacy_seeds,
    "(Jρ, 0) built from (ρ, 0)'s field must fail",
    expected = "give one w",
    check_legacy_seeds(rho_for_jrho)
);

// ── seed_selection_legacy_chart ─────────────────────────────────────────────────────────────────────────────────

/// The `(L_z, E)` chart's decode under `seeding` at the decode preset's pin is the legacy largest-norm construction,
/// decided on the pin's `z_α`, on a grid where `K* > K_min`.
fn check_legacy_chart(seeding: Seeding) {
    let le = InvariantLE(invariant_at::<f64>(PINS[1]));
    let mut counted = 0;
    for i in 1..8 {
        for j in 1..8 {
            let (u, v) = (f64::from(i) / 8.0, f64::from(j) / 8.0);
            let z = le.map::<f64>(u, v).z;
            let (d, outcome) = invariant::decode_seeded::<Defaults, f64, _>(
                &le,
                u,
                v,
                &z,
                &LinkParams::DEFAULT,
                seeding,
            );
            if outcome != Outcome::Feasible {
                continue;
            }
            let t = le.0.targets::<f64>(u, v);
            let want =
                kernel::decode::invariant::construct(&d.masses, &d.config, PINS[1][0], t.lz, t.k);
            if want.a <= 1e-3 {
                continue;
            }
            counted += 1;
            for b in 0..3 {
                for x in 0..2 {
                    assert!(
                        (d.ic.p[b][x] - want.p[b][x]).abs() <= 1e-12,
                        "the decode at ({u}, {v}) is not R-82's legacy construction: p[{b}][{x}] = {:e}, want {:e}",
                        d.ic.p[b][x],
                        want.p[b][x]
                    );
                }
            }
        }
    }
    assert!(counted > 10, "{counted} pixels with K* > K_min");
}

#[test]
fn seed_selection_legacy_chart() {
    check_legacy_chart(Seeding::LargestNorm);
}

negative_control!(
    seed_selection_legacy_chart,
    "the decode under the seed in force is not R-82's legacy construction and must fail",
    expected = "is not R-82's legacy construction",
    check_legacy_chart(Seeding::Expansion)
);

// ── seed_conversion_charts ──────────────────────────────────────────────────────────────────────────────────────

/// How far a decoded `(m, r, p)` is from homographic: `v = p/m`, `ω = L/I`, `λ = D/I` (`D = Σ mᵢ rᵢ·vᵢ`), and the
/// residual `‖v − ω·J r − λ·r‖_m` over `‖v‖_m`; returns `(residual, λ, ‖v‖_m)`.
fn homographic(ic: &Ic<f64>) -> (f64, f64, f64) {
    let (m, r) = (ic.m, ic.r);
    let v: [[f64; 2]; 3] = std::array::from_fn(|i| [ic.p[i][0] / m[i], ic.p[i][1] / m[i]]);
    let i_ = inertia(m, r);
    let l: f64 = (0..3)
        .map(|i| m[i] * (r[i][0] * v[i][1] - r[i][1] * v[i][0]))
        .sum();
    let d: f64 = (0..3)
        .map(|i| m[i] * (r[i][0] * v[i][0] + r[i][1] * v[i][1]))
        .sum();
    let (omega, lambda) = (l / i_, d / i_);
    let off: f64 = (0..3)
        .map(|i| {
            let e = [
                v[i][0] - (-omega * r[i][1] + lambda * r[i][0]),
                v[i][1] - (omega * r[i][0] + lambda * r[i][1]),
            ];
            m[i] * (e[0] * e[0] + e[1] * e[1])
        })
        .sum();
    let size: f64 = (0..3)
        .map(|i| m[i] * (v[i][0] * v[i][0] + v[i][1] * v[i][1]))
        .sum();
    (
        off.sqrt() / size.sqrt().max(f64::MIN_POSITIVE),
        lambda,
        size.sqrt(),
    )
}

/// The decoded state in f64, or `None` where the decode is tagged (not a system to convert).
fn state<R: Q>(d: &Decoded<R>) -> Option<Ic<f64>> {
    d.degenerate.is_none().then(|| Ic {
        m: d.ic.m.map(Q::f),
        r: d.ic.r.map(|x| x.map(Q::f)),
        p: d.ic.p.map(|x| x.map(Q::f)),
    })
}

/// A chart image to convert: its name, its precision's tolerance and its decode at `(u, v)`.
type Image = (String, f64, Box<dyn Fn(f64, f64) -> Option<Ic<f64>>>);

/// Every pixel of a 33 × 33 grid over `[0,1]²` of each image is homographic, `λ ≥ 0` (the expanding sheet), to the
/// image's tolerance; and the pixels with `K* > K_min`, `λ > 0`, number at least a quarter of each image.
fn check_homographic(images: &[Image]) {
    for (name, tol, decode) in images {
        let (mut moving, mut systems) = (0, 0);
        for i in 0..=32 {
            for j in 0..=32 {
                let (u, v) = (f64::from(i) / 32.0, f64::from(j) / 32.0);
                let Some(ic) = decode(u, v) else { continue };
                systems += 1;
                let (off, lambda, size) = homographic(&ic);
                assert!(
                    off <= *tol,
                    "{name}: the pixel ({u}, {v}) is not homographic: residual {off:e} of ‖v‖_m = {size:e}"
                );
                assert!(
                    lambda >= -tol * size,
                    "{name}: the pixel ({u}, {v}) contracts, λ = {lambda:e}"
                );
                moving += usize::from(lambda > 1e3 * tol * size);
            }
        }
        println!("{name}: {systems} systems, {moving} with K* > K_min, every one homographic");
        assert!(
            4 * moving >= 33 * 33,
            "{name}: {moving} pixels with K* > K_min"
        );
    }
}

/// The invariant charts' pins, `(z_α, z_β, z_μ₁, z_μ₂)`: the default (the equal-mass Lagrange triangle), the decode
/// preset's and one at `z_α < 0`.
const PINS: [[f64; 4]; 3] = [
    [0.0, 0.0, 0.0, 0.0],
    [0.3, -0.5, 0.2, 0.1],
    [-0.7, 1.1, -0.4, 0.6],
];

fn invariant_at<R: Q>(pin: [f64; 4]) -> Invariant<R> {
    Invariant {
        z_cfg: [R::of(pin[0]), R::of(pin[1])],
        z_mass: [R::of(pin[2]), R::of(pin[3])],
        k_max: R::of(1.0),
        gamma_k: R::of(2.0),
    }
}

/// The images under the seed in force: every chart that runs the construction.
fn images() -> Vec<Image> {
    let mut out: Vec<Image> = Vec::new();
    for pin in PINS {
        let le = InvariantLE(invariant_at::<f64>(pin));
        let lk = InvariantLK(invariant_at::<f64>(pin));
        let le32 = InvariantLE(invariant_at::<f32>(pin));
        out.push((
            format!("(L_z, E) at {pin:?}"),
            1e-12,
            Box::new(move |u, v| state(&le.decode::<Defaults, f64>(u, v))),
        ));
        out.push((
            format!("(L_z, K) at {pin:?}"),
            1e-12,
            Box::new(move |u, v| state(&lk.decode::<Defaults, f64>(u, v))),
        ));
        out.push((
            format!("(L_z, E) at {pin:?}, f32"),
            1e-5,
            Box::new(move |u, v| state(&le32.decode::<Defaults, f32>(u as f32, v as f32))),
        ));
        let linear = Mapped::<_, Defaults>::new(
            InvariantLE(invariant_at::<f64>(pin)),
            [Mapping::Linear, Mapping::Linear],
        );
        out.push((
            format!("(L_z, E) at {pin:?}, linear"),
            1e-12,
            Box::new(move |u, v| state(&linear.decode::<Defaults, f64>(u, v))),
        ));
    }
    let z0 = [0.3, -0.5, 0.0, 0.0, 0.0, 0.0, 0.2, 0.1];
    for held in [0.0, 0.3] {
        let e: MixedEnergy<Alpha, AxisE, Defaults, f64> =
            MixedEnergy::new(z0, [0.2, 1.3], 1.0, 2.0, held, Mapping::Warped);
        let k: MixedEnergy<Alpha, AxisK, Defaults, f64> =
            MixedEnergy::new(z0, [0.2, 1.3], 1.0, 2.0, held, Mapping::Warped);
        out.push((
            format!("α × E, held L_z = {held}"),
            1e-12,
            Box::new(move |u, v| state(&e.decode::<Defaults, f64>(u, v))),
        ));
        out.push((
            format!("α × K, held L_z = {held}"),
            1e-12,
            Box::new(move |u, v| state(&k.decode::<Defaults, f64>(u, v))),
        ));
    }
    let nu_k = Burrau {
        nu_range: [1.0 / 32.0, 31.0 / 32.0],
        k_max: 1.0,
        gamma_k: 2.0,
    };
    out.push((
        "(ν, K)".into(),
        1e-12,
        Box::new(move |u, v| state(&nu_k.decode::<Defaults, f64>(u, v))),
    ));
    let theta_k = BurrauThetaK {
        theta_range: [0.1, FRAC_PI_4],
        k_max: 1.0,
        gamma_k: 2.0,
    };
    out.push((
        "Φ_θ,K".into(),
        1e-12,
        Box::new(move |u, v| state(&theta_k.decode::<Defaults, f64>(u, v))),
    ));
    let theta_lz = BurrauThetaLz {
        theta_range: [0.1, FRAC_PI_4],
        k: 0.125,
        lz_fraction: 1.0,
    };
    out.push((
        "Φ_θ,L_z".into(),
        1e-12,
        Box::new(move |u, v| state(&theta_lz.decode::<Defaults, f64>(u, v))),
    ));
    out
}

#[test]
fn seed_conversion_charts() {
    check_homographic(&images());
}

#[cfg(feature = "controls")]
/// The images under the legacy largest-norm rule, the named option: the `(L_z, E)` chart at the decode preset's pin and
/// the `(ν, K)` chart across `ν = 1/3`.
fn legacy_images() -> Vec<Image> {
    let le = InvariantLE(invariant_at::<f64>(PINS[1]));
    let nu_k = Burrau {
        nu_range: [1.0 / 32.0, 31.0 / 32.0],
        k_max: 1.0,
        gamma_k: 2.0,
    };
    vec![
        (
            "(L_z, E), R-82's legacy rule".into(),
            1e-12,
            Box::new(move |u, v| {
                let z = le.map::<f64>(u, v).z;
                let (d, _) = invariant::decode_seeded::<Defaults, f64, _>(
                    &le,
                    u,
                    v,
                    &z,
                    &LinkParams::DEFAULT,
                    Seeding::LargestNorm,
                );
                state(&d)
            }),
        ),
        (
            "(ν, K), R-82's legacy rule".into(),
            1e-12,
            Box::new(move |u, v| {
                let k = nu_k.kinetic(v);
                state(
                    &kernel::chart::burrau::decode_lz_seeded(
                        &nu_k,
                        u,
                        0.0,
                        k,
                        Seeding::LargestNorm,
                    )
                    .0,
                )
            }),
        ),
    ]
}

negative_control!(
    seed_conversion_charts,
    "the invariant chart under R-82's legacy rule is not homographic where K* > K_min, and must fail",
    expected = "is not homographic",
    check_homographic(&legacy_images()[..1])
);

#[cfg(feature = "controls")]
mod conversion_burrau {
    use super::*;

    negative_control!(
        seed_conversion_charts,
        "the (ν, K) chart under R-82's legacy rule is not homographic, and must fail",
        expected = "is not homographic",
        check_homographic(&legacy_images()[1..])
    );
}

// ── seed_conversion_seam ────────────────────────────────────────────────────────────────────────────────────────

/// The seam image's columns: `z_α = (j − 31.5)/64`, `j = 0 … 63`, so the line `z_α = 0` lies between columns 31 and 32.
const COLUMNS: usize = 64;
/// The neighbouring pair across the line: columns 31 and 32.
const LINE: usize = COLUMNS / 2 - 1;
/// The neighbouring pairs read on either side of the line (the first criterion's 8 pixels).
const SIDE: usize = 8;

/// The seam image's rows: chart points `(u, v)` of the `(L_z, E)` chart, `K* > K_min` at each.
const ROWS: [(f64, f64); 6] = [
    (0.5, 0.4),
    (0.5, 0.9),
    (0.25, 0.6),
    (0.8, 0.7),
    (0.1, 1.0),
    (0.65, 0.3),
];

/// One row's measurement: the jump across the line, the largest neighbouring difference within [`SIDE`] pixels of it on
/// either side, and `2√(K* − K_min)`, the legacy rule's predicted jump; and each neighbouring pair's `‖Δv‖_m` beside
/// `√(2K*)·2 sin(|Δα|/2)`, the difference the configuration's own motion gives a homographic start.
#[derive(Debug)]
struct Seam {
    across: f64,
    beside: f64,
    predicted: f64,
    pairs: Vec<(f64, f64)>,
}

/// The decode at `(u, v)` of `chart` tilted toward `z_α` by `dz`, under `seeding`: the state and the decoded `α`.
fn tilted(chart: &InvariantLE<f64>, u: f64, v: f64, dz: f64, seeding: Seeding) -> (Ic<f64>, f64) {
    let shift = Shift {
        x: 0,
        dx: dz,
        y: 0,
        dy: 0.0,
        two: false, ..Shift::none() };
    let z = Latent::new(shift_controls(chart.0.controls::<f64>(), &shift));
    let d = match seeding {
        Seeding::Expansion => {
            chart.decode_shifted::<Defaults, f64>(u, v, &shift, &LinkParams::DEFAULT)
        }
        Seeding::LargestNorm => {
            invariant::decode_seeded::<Defaults, f64, _>(
                chart,
                u,
                v,
                &z,
                &LinkParams::DEFAULT,
                seeding,
            )
            .0
        }
    };
    (state(&d).expect("a system"), d.config.alpha)
}

/// The seam measurement on every row under `seeding` (the module's note): the `(L_z, E)` chart at the decode preset's
/// `z_β` and masses, `z_α = 0` its pin, tilted toward `z_α` across the window.
fn measure(seeding: Seeding) -> Vec<Seam> {
    let chart = InvariantLE(Invariant {
        z_cfg: [0.0, PINS[1][1]],
        z_mass: [PINS[1][2], PINS[1][3]],
        k_max: 1.0,
        gamma_k: 2.0,
    });
    ROWS.iter()
        .map(|&(u, v)| {
            let row: Vec<(Ic<f64>, f64)> = (0..COLUMNS)
                .map(|j| tilted(&chart, u, v, (j as f64 - 31.5) / 64.0, seeding))
                .collect();
            let gap = |a: &Ic<f64>, b: &Ic<f64>| {
                (0..3)
                    .map(|i| {
                        let d = [
                            b.p[i][0] / b.m[i] - a.p[i][0] / a.m[i],
                            b.p[i][1] / b.m[i] - a.p[i][1] / a.m[i],
                        ];
                        a.m[i] * (d[0] * d[0] + d[1] * d[1])
                    })
                    .sum::<f64>()
                    .sqrt()
            };
            let t = chart.0.targets::<f64>(u, v);
            let pairs: Vec<(f64, f64)> = row
                .windows(2)
                .map(|w| {
                    let chord = 2.0 * ((w[1].1 - w[0].1).abs() / 2.0).sin();
                    (gap(&w[0].0, &w[1].0), (2.0 * t.k).sqrt() * chord)
                })
                .collect();
            let beside = (LINE - SIDE..=LINE + SIDE)
                .filter(|&j| j != LINE)
                .map(|j| pairs[j].0)
                .fold(0.0, f64::max);
            Seam {
                across: pairs[LINE].0,
                beside,
                predicted: 2.0 * (t.k - t.lz * t.lz / 2.0).max(0.0).sqrt(),
                pairs,
            }
        })
        .collect()
}

/// The explanation identity's tolerance: each pair's `‖Δv‖_m` against `√(2K*)·2 sin(|Δα|/2)`, relative.
const EXPLAINED: f64 = 1e-9;

impl Seam {
    /// Pair `j`'s relative residual from the identity, `|‖Δv‖_m − √(2K*)·2 sin(|Δα|/2)|` over the latter.
    fn residual(&self, j: usize) -> f64 {
        let (dv, motion) = self.pairs[j];
        (dv - motion).abs() / motion
    }

    /// The largest residual over the row's neighbouring pairs away from the line.
    fn worst_away(&self) -> f64 {
        (0..self.pairs.len())
            .filter(|&j| j != LINE)
            .map(|j| self.residual(j))
            .fold(0.0, f64::max)
    }
}

/// Prints the first criterion's numbers on each row of `seams`, and returns whether it holds (no row's jump across the
/// line exceeds its largest neighbouring difference within [`SIDE`] pixels). It judges nothing: a report only.
fn first_criterion(seams: &[Seam], what: &str) -> bool {
    for (s, (u, v)) in seams.iter().zip(ROWS) {
        println!(
            "{what}, row (u, v) = ({u}, {v}): across {:.9e}, largest beside {:.9e}, ratio {:.6}, \
             legacy prediction 2√(K* − K_min) = {:.6}",
            s.across,
            s.beside,
            s.across / s.beside,
            s.predicted
        );
    }
    seams.iter().all(|s| s.across <= s.beside)
}

/// Prints the identity's residuals on each row of `seams`: the pair across the line and the largest of the other 62.
fn report_explained(seams: &[Seam], what: &str) {
    for (s, (u, v)) in seams.iter().zip(ROWS) {
        println!(
            "{what}, row (u, v) = ({u}, {v}): the identity's relative residual across the line {:.3e}, the largest \
             of the other {} pairs {:.3e} (criterion: each ≤ {EXPLAINED:e})",
            s.residual(LINE),
            s.pairs.len() - 1,
            s.worst_away()
        );
    }
}

/// The criterion. Every neighbouring pair of every row, the pair across the line included, differs in decoded velocity
/// by exactly what the configuration's own motion gives a homographic start, `‖Δv‖_m = √(2K*)·2 sin(|Δα|/2)`
/// (`v = (ω·J + λ)·r` at fixed `ω`, `λ`, `ω² + λ² = 2K*` at `I = 1`, and `‖Δr‖_m` the chord of `Δα` on the unit
/// hypersphere at fixed `β` and masses), to a relative 10⁻⁹: no seed term, so no seam. Read on the pairs `pairs` of
/// each row.
fn check_explained(seams: &[Seam], pairs: std::ops::Range<usize>, what: &str) {
    for (s, (u, v)) in seams.iter().zip(ROWS) {
        for j in pairs.clone() {
            let (dv, motion) = s.pairs[j];
            assert!(
                s.residual(j) <= EXPLAINED,
                "{what}: the row (u, v) = ({u}, {v}), pair {j}: ‖Δv‖_m = {dv:e} is not the configuration's own \
                 motion, {motion:e}"
            );
        }
    }
}

/// The control's reading: under the legacy rule the pair across the line fails the identity on every row, and by more
/// than any other pair of its row. A row where either fails means the identity cannot see the seam there, and the
/// result is void.
///
/// The plan is unclear here; chose to require the line's residual to exceed every other pair's on the row, not only the
/// tolerance, because the legacy rule misses the identity away from the line too, on every row: its seed is one fixed
/// vector on each side and does not turn with `α`, so each pair there is short by a relative `1 − |L_z|/√(2K*)`, at
/// most 1. Failing the identity at the line would then hold with no jump across it; the jump is what makes the line's
/// residual the row's largest. No threshold is added.
fn check_seam_seen(control: &[Seam]) {
    for (s, (u, v)) in control.iter().zip(ROWS) {
        assert!(
            s.residual(LINE) > EXPLAINED,
            "the control satisfies the identity across the line on the row (u, v) = ({u}, {v}): the identity cannot \
             see the seam, the result is void"
        );
        assert!(
            s.residual(LINE) > s.worst_away(),
            "the control's residual across the line, {:e}, is not above every other pair's on the row (u, v) = ({u}, \
             {v}), {:e}: no jump is seen at the line, the result is void",
            s.residual(LINE),
            s.worst_away()
        );
    }
}

#[test]
fn seed_conversion_seam() {
    let now = measure(Seeding::Expansion);
    let before = measure(Seeding::LargestNorm);
    let held = first_criterion(&now, "the seed in force");
    let held_before = first_criterion(&before, "R-82's legacy rule");
    println!(
        "seed_conversion_seam: the first criterion (across ≤ largest beside), judging nothing: under the seed in force \
         it {} (an explained divergence where it does not: the α link's warp, 1 + h²/4 at h = 1/64, no seam); under \
         R-82's legacy rule it {}",
        if held { "holds" } else { "does not hold" },
        if held_before {
            "holds"
        } else {
            "does not hold"
        }
    );
    report_explained(&now, "the seed in force");
    report_explained(&before, "R-82's legacy rule (the control)");
    check_seam_seen(&before);
    check_explained(&now, 0..COLUMNS - 1, "the seed in force");
    println!(
        "seed_conversion_seam: PASS: every neighbouring difference under the seed in force, the pair across the line \
         included, is the configuration's own motion to a relative {EXPLAINED:e}, no seed term; the control fails the \
         identity across the line on every row"
    );
}

// The control arm: the criterion under the legacy rule, read at the pair across the line on every row. It panics with
// the criterion's own message only when every row fails there; a row that passes panics as void instead, which is not
// the message the control expects.
negative_control!(
    seed_conversion_seam,
    "the pair across z_α = 0 under R-82's legacy rule carries the seed's jump, not the configuration's motion, on \
     every row",
    expected = "R-82's legacy rule, across the line: the row (u, v) = (0.5, 0.4), pair 31: ‖Δv‖_m",
    {
        let before = measure(Seeding::LargestNorm);
        check_seam_seen(&before);
        check_explained(&before, LINE..LINE + 1, "R-82's legacy rule, across the line");
    }
);

// ── seed_conversion_lagrange ────────────────────────────────────────────────────────────────────────────────────

/// At the invariant charts' default pin, `z = 0`: the masses equal and the configuration the equilateral triangle, and
/// every pixel of `images` homographic, so each starts a homographic orbit, whose break-up is set by rounding.
fn check_lagrange(images: &[Image]) {
    let (masses, config) = decoded::<f64>([0.0; 4]);
    for m in masses.m {
        assert!(
            (m - 1.0 / 3.0).abs() <= 1e-15,
            "the default pin's masses: {:?}",
            masses.m
        );
    }
    let r = config.r;
    let side =
        |a: usize, b: usize| ((r[a][0] - r[b][0]).powi(2) + (r[a][1] - r[b][1]).powi(2)).sqrt();
    let sides = [side(0, 1), side(1, 2), side(2, 0)];
    for s in sides {
        assert!(
            (s - sides[0]).abs() <= 1e-14,
            "the default pin's sides: {sides:?}"
        );
    }
    check_homographic(images);
    println!(
        "the invariant charts' default pin, the equal-mass Lagrange triangle: every pixel starts a homographic orbit, \
         its break-up set by rounding (R-934)"
    );
}

fn lagrange_images(seeding: Seeding) -> Vec<Image> {
    let le = InvariantLE(invariant_at::<f64>(PINS[0]));
    let lk = InvariantLK(invariant_at::<f64>(PINS[0]));
    vec![
        (
            "(L_z, E) at the default pin".into(),
            1e-12,
            Box::new(move |u, v| {
                state(
                    &invariant::decode_seeded::<Defaults, f64, _>(
                        &le,
                        u,
                        v,
                        &le.map::<f64>(u, v).z,
                        &LinkParams::DEFAULT,
                        seeding,
                    )
                    .0,
                )
            }),
        ),
        (
            "(L_z, K) at the default pin".into(),
            1e-12,
            Box::new(move |u, v| {
                state(
                    &invariant::decode_seeded::<Defaults, f64, _>(
                        &lk,
                        u,
                        v,
                        &lk.map::<f64>(u, v).z,
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
fn seed_conversion_lagrange() {
    check_lagrange(&lagrange_images(Seeding::Expansion));
}

negative_control!(
    seed_conversion_lagrange,
    "R-82's legacy rule at the Lagrange triangle takes (ρ, 0), not homographic, and must fail",
    expected = "is not homographic",
    check_lagrange(&lagrange_images(Seeding::LargestNorm))
);
