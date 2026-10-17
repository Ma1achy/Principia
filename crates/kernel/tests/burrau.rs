//! The Burrau family (`kernel::burrau`, `kernel::chart::burrau`; chart_reference §4.1–§4.5, §5.2; chart_decoder_contract
//! Part 5; R-22), each check at the f64 and the f32 instantiation natively (R-118, R-265):
//! - `burrau_euclid_*`: Euclid's parametrisation gives `(3, 4, 5)` at `(2, 1)` and the §4.1 table's rows, a triple only
//!   at a primitive `(m, n)`; the normalised Burrau convention's masses `(c, b, a)/(a+b+c)`, `(5, 4, 3)/12` at `(2, 1)`,
//!   positions and Jacobi vectors `ρ = (a/c, 0)`, `λ = (−ab/(c(b+c)), b/c)`; the table's angles and `a/b`; the charts
//!   state the normalised convention (REQ-CHART-021).
//! - `burrau_nu_continuous_*`: the embed's masses, positions and Jacobi vectors, and the decoded IC, are continuous in
//!   `ν`; the primitive triples' `ν = n/m` reproduce the discrete family; the `(ν, K)` chart's momenta are §2's
//!   construction at `L_z = 0`, `K(v)`, its seed R-82's decided exactly on `ν` (REQ-CHART-022).
//! - `euclid_plane_*`: varying `u` at fixed `v` gives identical ICs; the overlay marks exactly the primitive-triple
//!   lattice points; `m(u)` and `ν(v)` are §4.5's maps (REQ-CHART-023).
//! - `annotation_axis_*`: the Euclid plane's `m` axis is annotation-only and the decoder ignores it; the chart is a
//!   1-swept family; a 2D density across it is refused (REQ-CHART-013).
//! - `body_indices_*`: the collision of bodies 0 and 1 is pair id 2 by the payload's pair-id map; the family's 1-based
//!   numbering translates to 0-based; `ICDescriptor`'s fields are `m0 m1 m2`; `(3, 4, 5)` loads with masses
//!   `(5, 4, 3)/12` (REQ-SYS-016).
//! - `burrau_nu_half_*`: the `ν = 1/2` decode against the `(2, 1)` construction and the classical configuration within
//!   REQ-VAL-118's proposed tolerance, and the checked-in measurement, `fixtures/gates/burrau_nu_half/` (REQ-VAL-016).
//! - `burrau_chart_*`: validation (R-26) and the charts' flags and names.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.

mod decode_support;

use std::path::Path;

use decode_support::*;
use kernel::burrau::{
    euclid, from_literature, gcd, literature_pair_id, pair_id, primitive, Convention, System,
    Triple, CONVENTION, CONVENTION_STATEMENT,
};
use kernel::canon;
use kernel::chart::burrau::{
    acute_angle, decode, decode_plane, embed, seed, LatticePoint, RestBlocks,
};
use kernel::chart::invariant::WarpReal;
use kernel::chart::Axis;
use kernel::chart::{
    admit_statistic, family_label, swept, write_dump_header, AxisMeta, BlockTouch, Burrau, Chart,
    ChartDecode, Constraint, EuclidPlane, Statistic, StatisticRefused, ValidationResult,
};
use kernel::decode::config::{from_jacobi, Config};
use kernel::decode::invariant::{choose, construct_seeded, Outcome, Seed};
use kernel::decode::mass::{from_masses, Masses};
use kernel::decode::{decode_with, Decoded, Ic, Latent};
use kernel::generated::links::*;
use validation::{
    negative_control, pair_bodies, prop, BURRAU_NU_HALF_TOL, EUCLID_M_MAX, EUCLID_M_MIN,
    EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
};

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
    /// The next float below a positive finite `self`.
    fn down(self) -> Self;
}

impl Q for f64 {
    const LABEL: &'static str = "f64";
    fn up(self) -> Self {
        f64::from_bits(self.to_bits() + 1)
    }
    fn down(self) -> Self {
        f64::from_bits(self.to_bits() - 1)
    }
}

impl Q for f32 {
    const LABEL: &'static str = "f32";
    fn up(self) -> Self {
        f32::from_bits(self.to_bits() + 1)
    }
    fn down(self) -> Self {
        f32::from_bits(self.to_bits() - 1)
    }
}

/// The Euclid plane at the register's defaults (chart_reference §4.5), at `R`.
fn plane<R: Q>() -> EuclidPlane<R> {
    EuclidPlane {
        m_range: [R::of(EUCLID_M_MIN.number()), R::of(EUCLID_M_MAX.number())],
        nu_range: [R::of(EUCLID_NU_MIN.number()), R::of(EUCLID_NU_MAX.number())],
    }
}

/// The `(ν, K)` chart on the Euclid plane's `ν` range, under the register's warp defaults (REQ-CHART-044, proposed).
fn nu_k<R: Q>() -> Burrau<R> {
    Burrau {
        nu_range: [R::of(EUCLID_NU_MIN.number()), R::of(EUCLID_NU_MAX.number())],
        k_max: R::of(K_MAX.number()),
        gamma_k: R::of(GAMMA_K.number()),
    }
}

/// The system's quantities as one list: masses, positions, `ρ`, `λ`.
fn flat<R: P>(s: &System<R>) -> Vec<f64> {
    let mut v: Vec<f64> = s.m.iter().map(|x| x.f()).collect();
    v.extend(s.r.iter().flatten().map(|x| x.f()));
    v.extend(s.rho.iter().chain(&s.lambda).map(|x| x.f()));
    v
}

/// The canonical `(m, r, p)` as one list.
fn flat_ic<R: P>(ic: &Ic<R>) -> Vec<f64> {
    let mut v: Vec<f64> = ic.m.iter().map(|x| x.f()).collect();
    v.extend(ic.r.iter().flatten().map(|x| x.f()));
    v.extend(ic.p.iter().flatten().map(|x| x.f()));
    v
}

/// The largest componentwise `|a − b|`.
fn largest(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// The decode of `system` at rest, through the shared blocks.
fn at_rest<R: Q>(system: System<R>) -> Decoded<R> {
    decode_with(&mut RestBlocks::new(system), &Latent::new([R::zero(); 8]))
}

// ── burrau_euclid (REQ-CHART-021) ───────────────────────────────────────────────────────────────────────────────

/// A row of §4.1's table.
type Row = ((u32, u32), (u64, u64, u64), (f64, f64), f64);

/// §4.1's table: `(m, n)`, the triple, the two acute angles in degrees, and `a/b`.
const TABLE: [Row; 4] = [
    ((2, 1), (3, 4, 5), (36.9, 53.1), 0.75),
    ((3, 2), (5, 12, 13), (22.6, 67.4), 0.42),
    ((4, 1), (15, 8, 17), (61.9, 28.1), 1.88),
    ((4, 3), (7, 24, 25), (16.3, 73.7), 0.29),
];

/// A Euclid map under check.
type EuclidFn = fn(u32, u32) -> Option<Triple>;

/// The table's rows reproduce, and `(m, n)` gives a triple only where it is primitive, against a brute force over
/// `m, n ≤ 40`: `m > n > 0`, `gcd = 1`, `m − n` odd, and then `a² + b² = c²` with `a = m² − n²`, `b = 2mn`.
fn check_euclid(f: EuclidFn) {
    for ((m, n), (a, b, c), _, _) in TABLE {
        assert_eq!(
            f(m, n),
            Some(Triple { a, b, c }),
            "Euclid's triple at ({m}, {n}) is not ({a}, {b}, {c})"
        );
    }
    for m in 0..=40u32 {
        for n in 0..=40u32 {
            let want = m > n
                && n > 0
                && (1..=n).filter(|d| m % d == 0 && n % d == 0).max() == Some(1)
                && (m - n) % 2 == 1;
            let got = f(m, n);
            assert_eq!(
                got.is_some(),
                want,
                "Euclid's triple at ({m}, {n}): {got:?}"
            );
            if let Some(t) = got {
                let (m, n) = (m as u64, n as u64);
                assert_eq!(
                    (t.a, t.b, t.c),
                    (m * m - n * n, 2 * m * n, m * m + n * n),
                    "Euclid's triple at ({m}, {n})"
                );
                assert_eq!(t.a * t.a + t.b * t.b, t.c * t.c, "not Pythagorean: {t:?}");
            }
        }
    }
}

#[test]
fn burrau_euclid_triples() {
    check_euclid(euclid);
    assert_eq!((gcd(12, 18), gcd(7, 0), gcd(0, 7), gcd(1, 1)), (6, 7, 7, 1));
    assert!(primitive(2, 1) && !primitive(3, 1) && !primitive(4, 2) && !primitive(1, 1));
    // A `c` past u64 is no triple.
    assert_eq!(euclid(u32::MAX, u32::MAX - 1), None);
    assert!(euclid(1 << 31, (1 << 31) - 1).is_some());
}

negative_control!(
    burrau_euclid_triples,
    "a map that also takes m − n even must fail the check",
    expected = "Euclid's triple at (3, 1)",
    check_euclid(|m, n| {
        (m > n && n > 0 && gcd(m, n) == 1).then(|| {
            let (m, n) = (m as u64, n as u64);
            Triple {
                a: m * m - n * n,
                b: 2 * m * n,
                c: m * m + n * n,
            }
        })
    })
);

/// The system of the triple `t` under check.
type SystemFn<R> = fn(&Triple) -> System<R>;

/// At every primitive `(m, n)`, `m ≤ 32`: the masses are `(c, b, a)/(a+b+c)` and the positions `(0, 0)`, `(a/c, 0)`,
/// `(0, b/c)`, each the correctly rounded rational; `ρ = (a/c, 0)` and `λ = (−ab/(c(b+c)), b/c)` likewise, and they are
/// the Jacobi vectors of those positions and masses, `r₁ − r₀` and `r₂ − (m₀r₀ + m₁r₁)/M₀₁`, within rounding.
fn check_system<R: Q>(f: SystemFn<R>) {
    let q = |num: u64, den: u64| R::of(num as f64) / R::of(den as f64);
    for m in 2..=32u32 {
        for n in 1..m {
            let Some(t) = euclid(m, n) else { continue };
            let s = f(&t);
            let (a, b, c) = (t.a, t.b, t.c);
            let p = a + b + c;
            let want = System {
                m: [q(c, p), q(b, p), q(a, p)],
                r: [
                    [R::zero(), R::zero()],
                    [q(a, c), R::zero()],
                    [R::zero(), q(b, c)],
                ],
                rho: [q(a, c), R::zero()],
                lambda: [-q(a * b, c * (b + c)), q(b, c)],
            };
            assert_eq!(s, want, "the system of ({a}, {b}, {c}) at {}", R::LABEL);
            let m01 = s.m[0] + s.m[1];
            let r01 = [
                (s.m[0].f() * s.r[0][0].f() + s.m[1].f() * s.r[1][0].f()) / m01.f(),
                (s.m[0].f() * s.r[0][1].f() + s.m[1].f() * s.r[1][1].f()) / m01.f(),
            ];
            let jac = [
                s.r[1][0].f() - s.r[0][0].f(),
                s.r[1][1].f() - s.r[0][1].f(),
                s.r[2][0].f() - r01[0],
                s.r[2][1].f() - r01[1],
            ];
            let got = [s.rho[0].f(), s.rho[1].f(), s.lambda[0].f(), s.lambda[1].f()];
            assert!(
                largest(&jac, &got) <= 4.0 * R::eps(),
                "the Jacobi vectors of ({a}, {b}, {c}) at {}: {got:?}, from the positions {jac:?}",
                R::LABEL
            );
        }
    }
    let classical = f(&euclid(2, 1).unwrap());
    let twelfths = [
        R::of(5.0) / R::of(12.0),
        R::of(4.0) / R::of(12.0),
        R::of(3.0) / R::of(12.0),
    ];
    assert_eq!(
        classical.m,
        twelfths,
        "(2, 1) does not give masses (5, 4, 3)/12 at {}",
        R::LABEL
    );
}

#[test]
fn burrau_euclid_masses() {
    check_system::<f64>(Triple::burrau);
    check_system::<f32>(Triple::burrau);
}

negative_control!(
    burrau_euclid_masses,
    "masses in the order (a, b, c), the historical labelling, must fail the check",
    expected = "the system of (3, 4, 5) at f64",
    check_system::<f64>(|t| {
        let mut s = t.burrau::<f64>();
        s.m.reverse();
        s
    })
);

/// The table's acute angles, `atan2(a, b)` and `atan2(b, a)` in degrees, to its 0.1°, and `a/b` to its 0.01; the
/// `ν` axis's `θ(n/m)` is the first.
fn check_angles(theta: fn(f64) -> f64) {
    for ((m, n), (a, b, _), (t1, t2), ratio) in TABLE {
        let nu = n as f64 / m as f64;
        let got = theta(nu).to_degrees();
        let (a, b) = (a as f64, b as f64);
        assert!(
            (got - t1).abs() <= 0.05 && (90.0 - got - t2).abs() <= 0.05,
            "θ(ν) at ({m}, {n}) is {got}°, not the table's {t1}°"
        );
        assert!(
            (a.atan2(b).to_degrees() - got).abs() <= 1e-9,
            "θ(ν) at ({m}, {n}) is not atan(a/b)"
        );
        assert!((a / b - ratio).abs() <= 0.005, "a/b at ({m}, {n})");
    }
}

#[test]
fn burrau_euclid_angles() {
    check_angles(acute_angle::<f64>);
    assert!((acute_angle(0.5f32).to_degrees() - 36.869_9).abs() < 1e-3);
}

negative_control!(
    burrau_euclid_angles,
    "the complementary angle must fail the check",
    expected = "θ(ν) at (2, 1)",
    check_angles(|nu| std::f64::consts::FRAC_PI_2 - acute_angle(nu))
);

/// The convention stated: the crate's and both charts' are the normalised one, and its statement says so, with no
/// digit (so no body number, R-449).
fn check_convention(stated: Convention, statement: &str) {
    assert_eq!(
        stated,
        Convention::Normalised,
        "the convention stated is not the normalised one"
    );
    assert!(
        statement.contains("normalised"),
        "the statement does not say normalised: {statement}"
    );
    assert!(
        !statement.chars().any(|c| c.is_ascii_digit()),
        "the statement holds a digit: {statement}"
    );
}

#[test]
fn burrau_euclid_convention() {
    check_convention(CONVENTION, CONVENTION_STATEMENT);
    check_convention(
        plane::<f64>().convention(),
        plane::<f64>().convention_statement(),
    );
    check_convention(
        nu_k::<f32>().convention(),
        nu_k::<f32>().convention_statement(),
    );
}

negative_control!(
    burrau_euclid_convention,
    "the historical convention must fail the check",
    expected = "the convention stated is not the normalised one",
    check_convention(Convention::Historical, CONVENTION_STATEMENT)
);

// ── burrau_nu_continuous (REQ-CHART-022) ────────────────────────────────────────────────────────────────────────

/// A map of `ν` to a list of quantities, under check for continuity.
type Curve = fn(f64) -> Vec<f64>;

/// `f` is Lipschitz on `[lo, hi]` with constant `lip`: at 2001 points `ν` and steps `h` from 2⁻¹⁰ to 2⁻³⁰,
/// `|f(ν + h) − f(ν)| ≤ lip·h + 8·eps` componentwise, `eps` the precision's, the rounding of the two evaluations.
fn check_lipschitz(name: &str, f: Curve, lo: f64, hi: f64, lip: f64, eps: f64) {
    for j in 0..=2000 {
        let nu = lo + (hi - lo) * j as f64 / 2000.0;
        let at = f(nu);
        for k in [10, 20, 30] {
            let h = (2f64).powi(-k);
            if nu + h > hi {
                continue;
            }
            let jump = largest(&f(nu + h), &at);
            assert!(
                jump <= lip * h + 8.0 * eps,
                "{name} jumps by {jump:e} over [{nu}, {nu} + 2^-{k}]"
            );
        }
    }
}

/// The embed at `R` as one list.
fn embed_at<R: Q>(nu: f64) -> Vec<f64> {
    flat(&embed(R::of(nu)))
}

/// The decoded IC at rest at `R` as one list.
fn decoded_at<R: Q>(nu: f64) -> Vec<f64> {
    flat_ic(&at_rest(embed(R::of(nu))).ic)
}

#[test]
fn burrau_nu_continuous_embed() {
    // The embed's masses, positions and Jacobi vectors have derivatives of magnitude at most 2 on (0, 1): `a/c`'s is
    // 4ν/(1+ν²)², `b/c`'s 2(1−ν²)/(1+ν²)², `λ_x`'s and the masses' below those; 4 leaves a margin.
    check_lipschitz(
        "the f64 embed",
        embed_at::<f64>,
        1e-6,
        1.0 - 1e-6,
        4.0,
        f64::EPSILON,
    );
    check_lipschitz(
        "the f32 embed",
        embed_at::<f32>,
        1e-3,
        1.0 - 1e-3,
        4.0,
        f32::EPSILON as f64,
    );
    // The decoded IC on the Euclid plane's range: the scale gauge grows as the triangle flattens toward either end, and
    // the decode's derivative there is at most 130.1 (measured at ν = 31/32 by central differences on a grid of 2·10⁵,
    // f64); 256 leaves a margin.
    let (lo, hi) = (EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number());
    check_lipschitz(
        "the f64 decode",
        decoded_at::<f64>,
        lo,
        hi,
        256.0,
        f64::EPSILON,
    );
    check_lipschitz(
        "the f32 decode",
        decoded_at::<f32>,
        lo,
        hi,
        256.0,
        f32::EPSILON as f64,
    );
}

negative_control!(
    burrau_nu_continuous_embed,
    "an embed with a jump at ν = 1/2 must fail the check",
    expected = "jumps by",
    check_lipschitz(
        "a broken embed",
        |nu| embed_at::<f64>(if nu > 0.5 { nu + 1e-3 } else { nu }),
        0.25,
        0.75,
        4.0,
        f64::EPSILON
    )
);

/// At every primitive `(m, n)`, `m ≤ 32`, the embed at `ν = n/m` is the triple's system within `tol·eps` per
/// component (`ν` is rounded once, so not bit for bit), and so is its decode.
fn check_primitive_nu<R: Q>(at: fn(R) -> System<R>, tol: f64) {
    for m in 2..=32u32 {
        for n in 1..m {
            let Some(t) = euclid(m, n) else { continue };
            let nu = R::of(n as f64) / R::of(m as f64);
            let (got, want) = (at(nu), t.burrau::<R>());
            let d = largest(&flat(&got), &flat(&want)) / R::eps();
            assert!(
                d <= tol,
                "ν = {n}/{m} is {d:.2} eps from the triple ({}, {}, {}) at {}",
                t.a,
                t.b,
                t.c,
                R::LABEL
            );
            let d = largest(&flat_ic(&at_rest(got).ic), &flat_ic(&at_rest(want).ic)) / R::eps();
            assert!(
                d <= 16.0 * tol,
                "ν = {n}/{m}'s decode is {d:.2} eps from the triple's at {}",
                R::LABEL
            );
        }
    }
}

#[test]
fn burrau_nu_continuous_primitive() {
    check_primitive_nu::<f64>(embed, 8.0);
    check_primitive_nu::<f32>(embed, 8.0);
}

negative_control!(
    burrau_nu_continuous_primitive,
    "an embed with its legs a and b swapped must fail the check",
    expected = "ν = 1/2 is",
    check_primitive_nu::<f64>(
        |nu| System::of_sides(nu + nu, 1.0 - nu * nu, 1.0 + nu * nu),
        8.0
    )
);

/// A `(ν, K)` decode under check.
type NuKDecode<R> = fn(&Burrau<R>, R, R) -> (Decoded<R>, Outcome);

/// The `(ν, K)` chart's decode at `(u, v)` is the shared blocks with §2's construction: its momenta are
/// [`construct_seeded`]'s on the decoded masses and configuration at `L_z = 0`, `K* = K(v)`, the seed [`seed`]`(ν(u))`,
/// bit for bit, its outcome feasible; they meet `Σpᵢ = 0`, `L_z(p) = 0` and `K(p) = K*` within REQ-DEC-046's
/// tolerances; and away from `ν = 1/3` the seed is the one R-82's rule takes on the computed norms ([`choose`]).
fn check_construction<R: Q>(dec: NuKDecode<R>, u: f64, v: f64) {
    let chart = nu_k::<R>();
    let (u, v) = (R::of(u), R::of(v));
    let (d, outcome) = dec(&chart, u, v);
    let nu = chart.nu(u);
    let k = chart.kinetic(v);
    let c = construct_seeded(&d.masses, &d.config, seed(nu), R::zero(), k);
    let at = format!("(u, v) = ({:e}, {:e}) at {}", u.f(), v.f(), R::LABEL);
    assert_eq!(
        d.ic.p, c.p,
        "the momenta are not the construction's at {at}"
    );
    assert_eq!(outcome, Outcome::Feasible, "the outcome at {at}");
    assert_eq!(
        d.descriptor,
        kernel::decode::descriptor::describe(&d.ic),
        "the descriptor is not the decoded IC's at {at}"
    );
    assert_eq!(
        d.ic.r, d.config.r,
        "the positions are not the configuration's at {at}"
    );
    let (m, r, p) = (d.ic.m, d.ic.r, d.ic.p);
    let sum = [p[0][0] + p[1][0] + p[2][0], p[0][1] + p[1][1] + p[2][1]];
    let scale: f64 = p.iter().flatten().map(|x| x.f().abs()).sum();
    let lz: f64 = (0..3)
        .map(|i| r[i][0].f() * p[i][1].f() - r[i][1].f() * p[i][0].f())
        .sum();
    let kin: f64 = (0..3)
        .map(|i| (p[i][0].f().powi(2) + p[i][1].f().powi(2)) / (2.0 * m[i].f()))
        .sum();
    let eps = R::eps();
    assert!(
        sum[0].f().abs().max(sum[1].f().abs()) <= TOL_P * eps * scale.max(f64::MIN_POSITIVE),
        "Σpᵢ = {sum:?} at {at}"
    );
    assert!(
        lz.abs() <= TOL_L * eps * (2.0 * k.f()).sqrt().max(f64::MIN_POSITIVE),
        "L_z(p) = {lz:e} at {at}"
    );
    assert!(
        (kin - k.f()).abs() <= TOL_K * eps * k.f(),
        "K(p) = {kin:e}, K* = {:e} at {at}",
        k.f()
    );
    if (nu.f() - 1.0 / 3.0).abs() > 1e-4 && k.f() > 0.0 {
        assert_eq!(
            Some(seed(nu)),
            choose(c.norms),
            "the seed at {at} is not R-82's"
        );
    }
}

#[test]
fn burrau_nu_continuous_construction() {
    let edges = [0.0, 1.0, 0.5, 1.0 / 3.0, 0.31, 0.32];
    for &u in &edges {
        for &v in &edges {
            check_construction::<f64>(decode, u, v);
            check_construction::<f32>(decode, u, v);
        }
    }
    prop::run(&(0.0f64..=1.0, 0.0f64..=1.0), |(u, v)| {
        check_construction::<f64>(decode, u, v);
        check_construction::<f32>(decode, u, v);
        Ok(())
    });
    // At K = 0 the construction gives the rest start: the (ν, K) chart's bottom row is the Euclid plane's system.
    let (d, _) = decode(&nu_k::<f64>(), 0.5, 0.0);
    assert_eq!(d.ic, decode_plane(&plane::<f64>(), 0.5).ic);
}

negative_control!(
    burrau_nu_continuous_construction,
    "momenta built at twice K* must fail the check",
    expected = "the momenta are not the construction's",
    check_construction::<f64>(
        |chart, u, v| {
            let doubled = Burrau {
                k_max: chart.k_max * 2.0,
                ..*chart
            };
            decode(&doubled, u, v)
        },
        0.4,
        0.7
    )
);

/// The shared configuration entry under check.
type FromJacobi = fn(&Masses<f64>, [f64; 2], [f64; 2]) -> Config<f64>;

/// `config::from_jacobi` on explicit `(ρ, λ)`, off the canonical frame too: `ρ̃ = √μ_ρ·ρ/ℓ`, `λ̃ = √μ_λ·λ/ℓ`,
/// `ℓ = √(μ_ρ‖ρ‖² + μ_λ‖λ‖²)`, so `‖ρ̃‖² + ‖λ̃‖² = 1`; `cos α = ‖ρ̃‖`, `sin α = ‖λ̃‖`, `β = atan2(λ_y, λ_x)`; the
/// positions are the decoder's reconstruction, `r₁ − r₀ = ρ/ℓ`, `r₂ − r₀₁ = λ/ℓ`, the CoM at the origin, `I = 1`; each
/// within 8 eps.
fn check_from_jacobi(f: FromJacobi) {
    let cases = [
        (
            [5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0],
            [0.6, 0.0],
            [-4.0 / 15.0, 0.8],
        ),
        ([0.5, 0.3, 0.2], [0.3, 0.4], [-0.5, 0.2]),
        ([0.2, 0.1, 0.7], [-0.7, -0.2], [0.1, -0.9]),
    ];
    let tol = 8.0 * f64::EPSILON;
    for (m, rho, lambda) in cases {
        let masses = from_masses(m);
        let c = f(&masses, rho, lambda);
        let m01 = m[0] + m[1];
        let (mu_rho, mu_lambda) = (m[0] * m[1] / m01, m[2] * m01);
        let ell = (mu_rho * (rho[0] * rho[0] + rho[1] * rho[1])
            + mu_lambda * (lambda[0] * lambda[0] + lambda[1] * lambda[1]))
            .sqrt();
        let want = [
            mu_rho.sqrt() * rho[0] / ell,
            mu_rho.sqrt() * rho[1] / ell,
            mu_lambda.sqrt() * lambda[0] / ell,
            mu_lambda.sqrt() * lambda[1] / ell,
        ];
        let got = [c.rho_t[0], c.rho_t[1], c.lambda_t[0], c.lambda_t[1]];
        assert!(
            largest(&got, &want) <= tol,
            "ρ̃, λ̃ of {rho:?}, {lambda:?}: {got:?}, not {want:?}"
        );
        let (nr, nl) = (want[0].hypot(want[1]), want[2].hypot(want[3]));
        let angles = [c.alpha.cos(), c.alpha.sin(), c.beta];
        let want_angles = [nr, nl, lambda[1].atan2(lambda[0])];
        assert!(
            largest(&angles, &want_angles) <= tol,
            "α, β of {rho:?}, {lambda:?}: {angles:?}"
        );
        let r = c.r;
        let r01 = [
            (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
            (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
        ];
        let jac = [
            r[1][0] - r[0][0],
            r[1][1] - r[0][1],
            r[2][0] - r01[0],
            r[2][1] - r01[1],
        ];
        let scaled = [rho[0] / ell, rho[1] / ell, lambda[0] / ell, lambda[1] / ell];
        assert!(
            largest(&jac, &scaled) <= tol,
            "the positions of {rho:?}, {lambda:?}: {jac:?}"
        );
        assert!(largest(&[c.rho[0], c.rho[1], c.lambda[0], c.lambda[1]], &scaled) <= tol);
        let com = [
            m[0] * r[0][0] + m[1] * r[1][0] + m[2] * r[2][0],
            m[0] * r[0][1] + m[1] * r[1][1] + m[2] * r[2][1],
        ];
        let inertia: f64 = (0..3)
            .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
            .sum();
        assert!(
            largest(&[com[0], com[1], inertia], &[0.0, 0.0, 1.0]) <= tol,
            "CoM {com:?}, I {inertia}"
        );
    }
}

#[test]
fn burrau_nu_continuous_from_jacobi() {
    check_from_jacobi(from_jacobi);
}

negative_control!(
    burrau_nu_continuous_from_jacobi,
    "an entry that drops ρ_y must fail the check",
    expected = "ρ̃, λ̃ of [0.3, 0.4]",
    check_from_jacobi(|m, rho, lambda| from_jacobi(m, [rho[0], 0.0], lambda))
);

/// Whether `3x ≤ 1` in exact arithmetic, for a positive normal `x` (an f32 widens to f64 exactly): with
/// `x = M·2^E`, `M` the integer significand, that is `3M ≤ 2^−E`.
fn third_at_most(x: f64) -> bool {
    let bits = x.to_bits();
    let m = (bits & ((1 << 52) - 1)) | (1 << 52);
    let e = ((bits >> 52) & 0x7FF) as i32 - 1075;
    3 * (m as u128) <= 1u128 << (-e)
}

/// The largest float `ν` at `R` with `3ν ≤ 1` in exact arithmetic.
fn third_below<R: Q>() -> R {
    let mut nu = R::of(1.0 / 3.0);
    while !third_at_most(nu.f()) {
        nu = nu.down();
    }
    while third_at_most(nu.up().f()) {
        nu = nu.up();
    }
    nu
}

/// The seed is `(ρ, 0)` at the largest float with `3ν ≤ 1` and `(0, λ)` at the next one up, and so across a grid.
fn check_seed<R: Q>(f: fn(R) -> Seed) {
    let t = third_below::<R>();
    assert_eq!(
        f(t),
        Seed::Rho,
        "the seed at ν = {:e} ({}), 3ν ≤ 1",
        t.f(),
        R::LABEL
    );
    assert_eq!(
        f(t.up()),
        Seed::Lambda,
        "the seed at ν = {:e} ({}), 3ν > 1",
        t.up().f(),
        R::LABEL
    );
    for j in 1..1000 {
        let nu = R::of(j as f64 / 1000.0);
        let want = if third_at_most(nu.f()) {
            Seed::Rho
        } else {
            Seed::Lambda
        };
        assert_eq!(f(nu), want, "the seed at ν = {:e} ({})", nu.f(), R::LABEL);
    }
}

#[test]
fn burrau_nu_continuous_seed() {
    check_seed::<f64>(seed);
    check_seed::<f32>(seed);
    // The f64 and f32 roundings of 1/3 sit on opposite sides of it, so both branches of the exact sum are reached.
    assert!(third_at_most(1.0f64 / 3.0) && seed(1.0f64 / 3.0) == Seed::Rho);
    assert!(!third_at_most((1.0f32 / 3.0) as f64));
    assert_eq!(seed(1.0f32 / 3.0), Seed::Lambda);
}

negative_control!(
    burrau_nu_continuous_seed,
    "a seed decided on the rounded 3ν must fail the check",
    expected = "the seed at ν =",
    check_seed::<f32>(|nu| if nu * 3.0 <= 1.0 {
        Seed::Rho
    } else {
        Seed::Lambda
    })
);

// ── euclid_plane (REQ-CHART-023) ────────────────────────────────────────────────────────────────────────────────

/// A decode of the Euclid plane at `(u, v)` under check.
type PlaneDecode<R> = fn(&EuclidPlane<R>, R, R) -> Decoded<R>;

/// The chart's own decode, through [`ChartDecode`].
fn plane_decode<R: Q>(c: &EuclidPlane<R>, u: R, v: R) -> Decoded<R>
where
    f64: From<R>,
{
    c.decode_at::<Defaults, R>(u, v)
}

/// At each `v` of a grid, every `u` (inside `[0,1]` and past it) gives the same decode, `==`, as `u = 0`, and that is
/// [`decode_plane`] at `v`.
fn check_identical<R: Q>(dec: PlaneDecode<R>) {
    let c = plane::<R>();
    for j in 0..=64 {
        let v = R::of(j as f64 / 64.0);
        let base = dec(&c, R::zero(), v);
        assert_eq!(
            base,
            decode_plane(&c, v),
            "the decode at v = {:e} ({})",
            v.f(),
            R::LABEL
        );
        for u in [0.25, 0.5, 1.0, 1e-30, -3.0, 7.5, 0.123_456_789] {
            assert_eq!(
                dec(&c, R::of(u), v),
                base,
                "the decode at (u, v) = ({u}, {:e}) differs from u = 0 ({})",
                v.f(),
                R::LABEL
            );
        }
    }
}

#[test]
fn euclid_plane_identical_ics() {
    check_identical::<f64>(plane_decode);
    check_identical::<f32>(plane_decode);
}

negative_control!(
    euclid_plane_identical_ics,
    "a decode that reads u into ν must fail the check",
    expected = "differs from u = 0",
    check_identical::<f64>(|c, u, v| decode_plane(c, v + 1e-3 * u))
);

/// `m(u)` and `ν(v)` are §4.5's at the defaults `[1, 32]` and `[1/32, 31/32]`: `m(0) = 1`, `m(1) = 32`, `m(½) = √32`,
/// increasing; `ν` linear, `ν(0) = 1/32`, `ν(1) = 31/32`, `ν(½) = ½`, `ν(¼) = 1/32 + ¼·30/32`.
fn check_maps(c: &EuclidPlane<f64>) {
    let e = 4.0 * f64::EPSILON * 32.0;
    assert_eq!(c.m(0.0f64), 1.0, "m(0) is not m_min");
    assert!(
        (c.m(1.0f64) - 32.0).abs() <= e,
        "m(1) is not m_max: {}",
        c.m(1.0f64)
    );
    assert!(
        (c.m(0.5f64) - 32f64.sqrt()).abs() <= e,
        "m(1/2) is not √32: {}",
        c.m(0.5f64)
    );
    assert!(
        (0..100).all(|j| c.m(j as f64 / 100.0) < c.m((j + 1) as f64 / 100.0)),
        "m(u) does not increase"
    );
    assert_eq!(
        (c.nu(0.0f64), c.nu(1.0f64), c.nu(0.5f64), c.nu(0.25f64)),
        (
            1.0 / 32.0,
            31.0 / 32.0,
            0.5,
            1.0 / 32.0 + 0.25 * 30.0 / 32.0
        ),
        "ν(v) is not linear on [1/32, 31/32]"
    );
}

#[test]
fn euclid_plane_maps() {
    assert_eq!(
        [EUCLID_M_MIN, EUCLID_M_MAX, EUCLID_NU_MIN, EUCLID_NU_MAX].map(|k| k.number()),
        [1.0, 32.0, 1.0 / 32.0, 31.0 / 32.0],
        "the register's defaults are not §4.5's"
    );
    check_maps(&plane::<f64>());
}

negative_control!(
    euclid_plane_maps,
    "a plane on m ∈ [1, 33] must fail the check",
    expected = "m(1) is not m_max",
    check_maps(&EuclidPlane {
        m_range: [1.0, 33.0],
        ..plane::<f64>()
    })
);

/// The overlay under check: the chart's lattice points.
type Overlay = fn(&EuclidPlane<f64>) -> Vec<LatticePoint>;

/// The primitive overlay marks exactly the primitive-triple lattice points `(m, n)` of the chart, by brute force over
/// every integer pair in its range, each at the `(u, v)` where `m(u) = m` and `ν(v) = n/m`; the integer overlay holds
/// every lattice point.
fn check_overlay(c: &EuclidPlane<f64>, overlay: Overlay) {
    let points = overlay(c);
    let mut lattice = Vec::new();
    for m in 1..=40u32 {
        for n in 1..m {
            let nu = n as f64 / m as f64;
            let inside = m as f64 >= c.m_range[0]
                && m as f64 <= c.m_range[1]
                && nu >= c.nu_range[0]
                && nu <= c.nu_range[1];
            if inside {
                let g = (1..=n).filter(|d| m % d == 0 && n % d == 0).max().unwrap();
                lattice.push((m, n, g == 1 && (m - n) % 2 == 1));
            }
        }
    }
    let got: Vec<(u32, u32, bool)> = points.iter().map(|p| (p.m, p.n, p.primitive)).collect();
    assert_eq!(
        got, lattice,
        "the overlay's points are not the chart's lattice"
    );
    for p in &points {
        assert!(
            (c.m(p.u) - p.m as f64).abs() <= 1e-12 * p.m as f64
                && (c.nu(p.v) - p.n as f64 / p.m as f64).abs() <= 1e-12,
            "the point ({}, {}) sits at (u, v) = ({}, {})",
            p.m,
            p.n,
            p.u,
            p.v
        );
    }
}

#[test]
fn euclid_plane_overlay() {
    let c = plane::<f64>();
    check_overlay(&c, |c| c.lattice().collect());
    let primitive: Vec<(u32, u32)> = c
        .lattice()
        .filter(|p| p.primitive)
        .map(|p| (p.m, p.n))
        .collect();
    for want in [(2, 1), (3, 2), (4, 1), (4, 3), (32, 1), (32, 31)] {
        assert!(primitive.contains(&want), "the overlay misses {want:?}");
    }
    for not in [(3, 1), (4, 2), (6, 3), (5, 3)] {
        assert!(!primitive.contains(&not), "the overlay marks {not:?}");
    }
    // A plane whose ranges are not integers or dyadic: the lattice is cut at its edges.
    let narrow = EuclidPlane {
        m_range: [2.5, 9.5],
        nu_range: [0.3, 0.7],
    };
    check_overlay(&narrow, |c| c.lattice().collect());
    assert_eq!(narrow.lattice().next().map(|p| (p.m, p.n)), Some((3, 1)));
    let f32_plane = plane::<f32>();
    assert_eq!(f32_plane.lattice().count(), c.lattice().count());
}

negative_control!(
    euclid_plane_overlay,
    "an overlay that marks every coprime pair must fail the check",
    expected = "the overlay's points are not the chart's lattice",
    check_overlay(&plane::<f64>(), |c| {
        c.lattice()
            .map(|p| LatticePoint {
                primitive: gcd(p.m, p.n) == 1,
                ..p
            })
            .collect()
    })
);

// ── annotation_axis (REQ-CHART-013) ─────────────────────────────────────────────────────────────────────────────

/// The Euclid plane's metadata: `m` annotation-only, touching no block; `ν` swept, writing the configuration and the
/// masses; the chart a 1-swept family; per-value statistics along `ν` admitted, along `m` and a 2D density refused.
fn check_annotation(axes: [AxisMeta; 2]) {
    assert!(
        axes[0].annotation && axes[0].touches.is_empty(),
        "m is not annotation-only: {:?}",
        axes[0]
    );
    assert!(
        !axes[1].annotation
            && axes[1].touches
                == BlockTouch {
                    config: true,
                    momentum: false,
                    mass: true
                },
        "ν is not the swept axis writing the configuration and masses: {:?}",
        axes[1]
    );
    assert_eq!(swept(axes), 1, "the chart is not 1-swept");
    assert_eq!(family_label(axes), "1-swept family rendered in 2D");
    assert_eq!(
        admit_statistic(axes, Statistic::Density2D),
        Err(StatisticRefused::DensityAcrossAnnotation),
        "a 2D density is admitted"
    );
    assert_eq!(
        admit_statistic(axes, Statistic::PerValue(Axis::Horizontal)),
        Err(StatisticRefused::AlongAnnotation),
        "a per-value statistic along m is admitted"
    );
    assert_eq!(
        admit_statistic(axes, Statistic::PerValue(Axis::Vertical)),
        Ok(())
    );
}

#[test]
fn annotation_axis_guard() {
    check_annotation(plane::<f64>().axes());
    check_annotation(plane::<f32>().axes());
    // The (ν, K) chart has no annotation axis: 2-swept, every statistic admitted.
    let axes = nu_k::<f64>().axes();
    assert_eq!((swept(axes), family_label(axes)), (2, "2-swept chart"));
    for s in [
        Statistic::Density2D,
        Statistic::PerValue(Axis::Horizontal),
        Statistic::PerValue(Axis::Vertical),
    ] {
        assert_eq!(admit_statistic(axes, s), Ok(()), "{s:?} refused on (ν, K)");
    }
    assert!(axes[1].touches.momentum && !axes[1].touches.is_empty());
    // An annotation axis on the vertical: the guard reads each axis, not a fixed one.
    let flipped = [plane::<f64>().axes()[1], plane::<f64>().axes()[0]];
    assert_eq!(
        admit_statistic(flipped, Statistic::PerValue(Axis::Vertical)),
        Err(StatisticRefused::AlongAnnotation)
    );
    assert_eq!(
        admit_statistic(flipped, Statistic::PerValue(Axis::Horizontal)),
        Ok(())
    );
    assert_eq!(
        admit_statistic(flipped, Statistic::Density2D),
        Err(StatisticRefused::DensityAcrossAnnotation)
    );
    let none = AxisMeta {
        touches: BlockTouch {
            config: false,
            momentum: false,
            mass: false,
        },
        annotation: true,
    };
    assert_eq!(
        admit_statistic([none, none], Statistic::Density2D),
        Err(StatisticRefused::DensityAcrossAnnotation)
    );
    assert_eq!(
        (swept([none, none]), family_label([none, none])),
        (0, "no swept axis")
    );
    for (t, empty) in [
        ((true, false, false), false),
        ((false, true, false), false),
        ((false, false, true), false),
        ((true, true, false), false),
        ((false, true, true), false),
        ((true, true, true), false),
        ((false, false, false), true),
    ] {
        let b = BlockTouch {
            config: t.0,
            momentum: t.1,
            mass: t.2,
        };
        assert_eq!(b.is_empty(), empty, "{b:?}");
    }
}

negative_control!(
    annotation_axis_guard,
    "a plane whose m axis is declared swept must fail the check",
    expected = "m is not annotation-only",
    check_annotation({
        let mut a = plane::<f64>().axes();
        a[0].annotation = false;
        a
    })
);

/// Varying `m` at fixed `ν` gives identical ICs: at each `ν` on a grid, every `u` whose `m(u)` spans the annotation
/// axis decodes, through the chart, to the decode at the lattice's first column.
fn check_ignores_m(dec: PlaneDecode<f64>) {
    let c = plane::<f64>();
    for j in 0..=32 {
        let v = j as f64 / 32.0;
        let want = dec(&c, 0.0, v).ic;
        for m in 1..=32 {
            let u = (m as f64).ln() / 32f64.ln();
            assert_eq!(
                dec(&c, u, v).ic,
                want,
                "m = {m} at ν = {} changes the IC",
                c.nu(v)
            );
        }
    }
}

#[test]
fn annotation_axis_ignored() {
    check_ignores_m(plane_decode);
    // `chart::decode`, through the chart's own `Chart::decode`, is the plane's decode too.
    check_ignores_m(kernel::chart::decode::<Defaults, f64, EuclidPlane<f64>>);
    assert_eq!(
        kernel::chart::decode::<Defaults, f64, _>(&plane::<f64>(), 0.7, 0.5),
        decode_plane(&plane::<f64>(), 0.5)
    );
    assert_eq!(plane::<f64>().map(0.1, 0.2), plane::<f64>().map(0.9, 0.2));
}

negative_control!(
    annotation_axis_ignored,
    "a decode whose masses follow m must fail the check",
    expected = "changes the IC",
    check_ignores_m(|c, u, v| {
        let mut d = decode_plane(c, v);
        d.ic.m[0] += 1e-9 * c.m(u);
        d
    })
);

// ── body_indices (REQ-SYS-016) ──────────────────────────────────────────────────────────────────────────────────

/// `pair_id` is the payload's pair-id map: pair `k` is `pair_bodies()[k]`, either order, and no pair of a body with
/// itself or past body 2; the collision of bodies 0 and 1 is pair 2.
fn check_pairs(f: fn(u32, u32) -> Option<u32>) {
    assert_eq!(
        f(0, 1),
        Some(2),
        "the collision of bodies 0 and 1 is not pair id 2"
    );
    for (k, [i, j]) in pair_bodies().into_iter().enumerate() {
        assert_eq!(
            f(i, j),
            Some(k as u32),
            "pair ({i}, {j}) is not pair id {k}"
        );
        assert_eq!(
            f(j, i),
            Some(k as u32),
            "pair ({j}, {i}) is not pair id {k}"
        );
    }
    for i in 0..5 {
        assert_eq!(f(i, i), None, "a body paired with itself");
        assert_eq!(f(i, 3), None, "body 3 does not exist");
        assert_eq!(f(3, i), None, "body 3 does not exist");
    }
}

#[test]
fn body_indices_pairs() {
    check_pairs(pair_id);
    // The family's 1-based form: body k is body k − 1, its inner pair 1–2 is pair id 2.
    assert_eq!(
        (0..=4).map(from_literature).collect::<Vec<_>>(),
        [None, Some(0), Some(1), Some(2), None]
    );
    assert_eq!(literature_pair_id(1, 2), Some(2));
    assert_eq!(literature_pair_id(2, 3), Some(0));
    assert_eq!(literature_pair_id(3, 1), Some(1));
    assert_eq!(
        (literature_pair_id(0, 1), literature_pair_id(1, 4)),
        (None, None)
    );
}

negative_control!(
    body_indices_pairs,
    "a pair id naming the side by its smaller body must fail the check",
    expected = "the collision of bodies 0 and 1 is not pair id 2",
    check_pairs(|i, j| (i < 3 && j < 3 && i != j).then(|| i.min(j)))
);

/// `(3, 4, 5)` loads with 0-based masses `(5, 4, 3)/12`: the triple's system and its decode's `ICDescriptor`, whose
/// fields are `m0 m1 m2` by name, and the Euclid plane's decode at `ν = 1/2`.
fn check_loads<R: Q>(d: Decoded<R>) {
    let twelfths = [5.0, 4.0, 3.0].map(|x| R::of(x) / R::of(12.0));
    let named = [d.descriptor.m0, d.descriptor.m1, d.descriptor.m2];
    assert_eq!(
        named,
        twelfths,
        "m0 m1 m2 of (3, 4, 5) are not (5, 4, 3)/12 at {}",
        R::LABEL
    );
    assert_eq!(
        d.ic.m,
        twelfths,
        "the masses of (3, 4, 5) are not (5, 4, 3)/12 at {}",
        R::LABEL
    );
    let names = ["m0", "m1", "m2"].map(|n| field(&d.descriptor, n));
    assert_eq!(names, named);
}

#[test]
fn body_indices_burrau_loads() {
    let t = euclid(2, 1).unwrap();
    assert_eq!((t.a, t.b, t.c), (3, 4, 5));
    check_loads::<f64>(at_rest(t.burrau()));
    check_loads::<f32>(at_rest(t.burrau()));
    check_loads::<f64>(decode_plane(&plane::<f64>(), 0.5));
    check_loads::<f32>(decode_plane(&plane::<f32>(), 0.5));
}

negative_control!(
    body_indices_burrau_loads,
    "the historical 1-based order, masses (3, 4, 5)/12, must fail the check",
    expected = "m0 m1 m2 of (3, 4, 5) are not (5, 4, 3)/12",
    check_loads::<f64>({
        let mut d = at_rest(euclid(2, 1).unwrap().burrau::<f64>());
        d.descriptor.m0 = 3.0 / 12.0;
        d.descriptor.m2 = 5.0 / 12.0;
        d
    })
);

// ── burrau_nu_half (REQ-VAL-016, REQ-VAL-118) ───────────────────────────────────────────────────────────────────

/// The classical configuration through `C` (chart_reference §4.2, §0.4, §0.5): the masses `(5, 4, 3)` in units of
/// their total (`M = 1`), each its opposite side, on the 3-4-5 triangle with the right angle at the origin, scaled by
/// `scale` (`1/5`, the normalised convention's; 1, the historical triangle's), at rest.
fn classical<R: Q>(scale: f64) -> Ic<R> {
    let x = |v: f64| R::of(v * scale);
    let m = [5.0, 4.0, 3.0].map(|v| R::of(v) / R::of(12.0));
    let r = [[x(0.0), x(0.0)], [x(3.0), x(0.0)], [x(0.0), x(4.0)]];
    canon::canonicalise(&Ic {
        m,
        r,
        p: [[R::zero(); 2]; 3],
    })
    .ic
}

/// The `ν = 1/2` decodes, the Euclid plane's at `v = 1/2` and the `(ν, K)` chart's at `K = 0`, against the `(2, 1)`
/// construction and the classical configuration, normalised and historical: each largest componentwise difference of
/// the canonical `(m, r, p)`, in units of `eps`.
fn residuals<R: Q>(nu_half: fn() -> Decoded<R>) -> [(&'static str, f64); 3] {
    let d = nu_half();
    let half = flat_ic(&d.ic);
    let triple = flat_ic(&at_rest(euclid(2, 1).unwrap().burrau::<R>()).ic);
    let normalised = flat_ic(&classical::<R>(1.0 / 5.0));
    let historical = flat_ic(&classical::<R>(1.0));
    [
        ("triple", largest(&half, &triple) / R::eps()),
        (
            "classical_normalised",
            largest(&half, &normalised) / R::eps(),
        ),
        (
            "classical_historical",
            largest(&half, &historical) / R::eps(),
        ),
    ]
}

/// The plane's `ν = 1/2` decode at `R`.
fn plane_half<R: Q>() -> Decoded<R> {
    let c = plane::<R>();
    assert_eq!(c.nu(R::of(0.5)), R::of(0.5), "ν(1/2) is not 1/2");
    decode_plane(&c, R::of(0.5))
}

/// Each residual within REQ-VAL-118's proposed tolerance (the register's `burrau_nu_half_tol`, provisional until M2).
fn check_nu_half<R: Q>(nu_half: fn() -> Decoded<R>) {
    let tol = BURRAU_NU_HALF_TOL.number();
    for (what, d) in residuals(nu_half) {
        eprintln!(
            "burrau_nu_half ({}): ν = 1/2 against {what}: {d:.3} eps",
            R::LABEL
        );
        assert!(
            d <= tol,
            "ν = 1/2 is {d:.3} eps from the {what} at {}, past the tolerance {tol} (REQ-VAL-118, provisional)",
            R::LABEL
        );
    }
}

#[test]
fn burrau_nu_half_reproduces() {
    check_nu_half::<f64>(plane_half);
    check_nu_half::<f32>(plane_half);
    check_nu_half::<f64>(|| decode(&nu_k::<f64>(), 0.5, 0.0).0);
    check_nu_half::<f32>(|| decode(&nu_k::<f32>(), 0.5, 0.0).0);
    // The triple: (3, 4, 5) itself, and its decode at f64 bit for bit.
    assert_eq!(euclid(2, 1).map(|t| (t.a, t.b, t.c)), Some((3, 4, 5)));
    assert_eq!(plane_half::<f64>(), at_rest(euclid(2, 1).unwrap().burrau()));
    assert_eq!(BURRAU_NU_HALF_TOL.provisional, Some("M2"));
}

negative_control!(
    burrau_nu_half_reproduces,
    "the decode at ν = 1/2 + 1e-6 must fail the check",
    expected = "past the tolerance",
    check_nu_half::<f64>(|| at_rest(embed(0.5 + 1e-6)))
);

/// The measurement file, relative to the workspace root.
const MEASUREMENT: &str = "fixtures/gates/burrau_nu_half/measurement.toml";

/// The calibration's evidence, each value measured, as `(key, value, what)`.
fn measure() -> Vec<(&'static str, f64, &'static str)> {
    let r64 = residuals::<f64>(plane_half);
    let r32 = residuals::<f32>(plane_half);
    vec![
        (
            "tolerance",
            BURRAU_NU_HALF_TOL.number(),
            "the proposed tolerance, eps",
        ),
        (
            "f64_triple",
            r64[0].1,
            "ν = 1/2 against the (2, 1) construction, f64, eps",
        ),
        (
            "f32_triple",
            r32[0].1,
            "ν = 1/2 against the (2, 1) construction, f32, eps",
        ),
        (
            "f64_classical_normalised",
            r64[1].1,
            "against the normalised classical configuration through C, f64, eps",
        ),
        (
            "f32_classical_normalised",
            r32[1].1,
            "against the normalised classical configuration through C, f32, eps",
        ),
        (
            "f64_classical_historical",
            r64[2].1,
            "against the historical classical configuration through C, f64, eps",
        ),
        (
            "f32_classical_historical",
            r32[2].1,
            "against the historical classical configuration through C, f32, eps",
        ),
    ]
}

/// The measurement file's text for `values`.
fn measurement_text(values: &[(&str, f64, &str)]) -> String {
    let mut s = String::from(
        "# The Burrau ν = 1/2 tolerance's evidence (REQ-VAL-118; R-71): the largest componentwise difference of the\n\
         # canonical (m, r, p) between the ν = 1/2 decode and the (m, n) = (2, 1) construction, and the classical\n\
         # configuration brought to the canonical section by C, in units of the precision's eps.\n\
         # Measured by crates/kernel/tests/burrau.rs, burrau_nu_half_measurement; regenerate with\n\
         # PRIN_WRITE_BURRAU_MEASUREMENT=1. Compared at an absolute 1e-9 eps.\n",
    );
    for (k, v, what) in values {
        s.push_str(&format!("{k} = {v:e}  # {what}\n"));
    }
    s
}

/// Each key of `text` holds its measured value, within an absolute 10⁻⁹.
fn check_measurement(text: &str, values: &[(&str, f64, &str)]) {
    for (k, v, _) in values {
        let line = text
            .lines()
            .find(|l| l.split('=').next().is_some_and(|key| key.trim() == *k))
            .unwrap_or_else(|| panic!("{MEASUREMENT}: no `{k}`"));
        let held: f64 = line
            .split('=')
            .nth(1)
            .and_then(|r| r.split('#').next())
            .map(str::trim)
            .and_then(|r| r.parse().ok())
            .unwrap_or_else(|| panic!("{MEASUREMENT}: `{k}` holds no number"));
        assert!(
            (held - v).abs() <= 1e-9,
            "{MEASUREMENT}: `{k}` holds {held:e}, measured {v:e}"
        );
    }
}

#[test]
fn burrau_nu_half_measurement() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(MEASUREMENT);
    let values = measure();
    if std::env::var_os("PRIN_WRITE_BURRAU_MEASUREMENT").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).expect("the fixture directory");
        std::fs::write(&path, measurement_text(&values)).expect("the measurement writes");
    }
    let text = std::fs::read_to_string(&path).expect("the measurement reads");
    check_measurement(&text, &values);
}

negative_control!(
    burrau_nu_half_measurement,
    "a measurement file whose f64 classical residual is off must fail the check",
    expected = "`f64_classical_normalised` holds",
    check_measurement(
        &measurement_text(
            &measure()
                .into_iter()
                .map(|(k, v, w)| (
                    k,
                    if k == "f64_classical_normalised" {
                        v + 1.0
                    } else {
                        v
                    },
                    w
                ))
                .collect::<Vec<_>>()
        ),
        &measure()
    )
);

// ── The charts' validation, flags and names (R-26, R-25) ────────────────────────────────────────────────────────

/// `validate` on both charts: `Valid` inside `[0,1]²` and on `(ν, K)` above it; `ν ∉ (0, 1)` and non-finite
/// coordinates rejected; `(ν, K)`'s `v < 0` projected to `v = 0`; a subnormal `ν`, whose decode is not finite,
/// rejected by layer 3.
fn check_validate(nu_k: &Burrau<f64>, plane: &EuclidPlane<f64>) {
    let wide_k = Burrau {
        nu_range: [0.0, 1.0],
        ..*nu_k
    };
    let wide_plane = EuclidPlane {
        nu_range: [0.0, 1.0],
        ..*plane
    };
    let reject = |failed| ValidationResult::Reject { failed };
    for &x in &[0.0, 0.25, 0.5, 1.0] {
        for &y in &[0.0, 0.5, 1.0] {
            assert_eq!(
                nu_k.validate(x, y),
                ValidationResult::Valid,
                "(ν, K) at ({x}, {y})"
            );
            assert_eq!(
                plane.validate(x, y),
                ValidationResult::Valid,
                "plane at ({x}, {y})"
            );
        }
    }
    assert_eq!(
        nu_k.validate(0.5, 3.0),
        ValidationResult::Valid,
        "(ν, K) above K_max"
    );
    assert_eq!(
        plane.validate(f64::NAN, 0.5),
        ValidationResult::Valid,
        "the plane reads u"
    );
    assert_eq!(
        nu_k.validate(0.3, -0.5),
        ValidationResult::Project {
            u: 0.3,
            v: 0.0,
            failed: Constraint::KNonNegative
        }
    );
    for (u, failed) in [
        (f64::NAN, Constraint::BurrauNu),
        (0.5, Constraint::KNonNegative),
    ] {
        let v = if u.is_nan() { 0.5 } else { f64::INFINITY };
        assert_eq!(nu_k.validate(u, v), reject(failed), "(ν, K) at ({u}, {v})");
    }
    for nu in [0.0, 1.0, -0.25, 1.5] {
        assert_eq!(
            wide_k.validate(nu, 0.5),
            reject(Constraint::BurrauNu),
            "(ν, K) at ν = {nu}"
        );
        assert_eq!(
            wide_plane.validate(0.5, nu),
            reject(Constraint::BurrauNu),
            "plane at ν = {nu}"
        );
    }
    assert_eq!(
        plane.validate(0.5, f64::INFINITY),
        reject(Constraint::BurrauNu)
    );
    // The open interval's ends, one float in, are valid.
    assert_eq!(
        wide_plane.validate(0.0, 1e-300),
        ValidationResult::Valid,
        "plane at ν = 1e-300"
    );
    assert_eq!(
        wide_plane.validate(0.0, 1.0f64.down()),
        ValidationResult::Valid,
        "plane at ν below 1"
    );
    assert_eq!(
        wide_k.validate(1e-300, 0.5),
        ValidationResult::Valid,
        "(ν, K) at ν = 1e-300"
    );
    // A subnormal ν: μ_ρ underflows to 0 and the reconstruction is not finite.
    let tiny = f64::from_bits(1);
    assert_eq!(
        wide_plane.validate(0.0, tiny),
        reject(Constraint::ComAtOrigin),
        "plane at the least ν"
    );
    assert_eq!(
        wide_k.validate(tiny, 0.5),
        reject(Constraint::ComAtOrigin),
        "(ν, K) at the least ν"
    );
    // K(v) overflows far above K_max: the momenta are not finite.
    assert_eq!(
        nu_k.validate(0.5, 1e200),
        reject(Constraint::MomentumZero),
        "(ν, K) at v = 1e200"
    );
}

#[test]
fn burrau_chart_validate() {
    check_validate(&nu_k::<f64>(), &plane::<f64>());
}

negative_control!(
    burrau_chart_validate,
    "a plane whose range leaves (0, 1) must fail the check",
    expected = "plane at (0, 0)",
    check_validate(
        &nu_k::<f64>(),
        &EuclidPlane {
            nu_range: [0.0, 1.0],
            ..plane::<f64>()
        }
    )
);

/// The flags and names: `(ν, K)` forbids `η_E` (its energy is the construction's), the Euclid plane does not (a rest
/// start); each name goes into the dump header.
fn check_flags(nu_k_forbids: bool, plane_forbids: bool) {
    assert!(nu_k_forbids, "the (ν, K) chart allows η_E");
    assert!(!plane_forbids, "the Euclid plane forbids η_E");
    let mut h = String::new();
    write_dump_header(&mut h, &nu_k::<f64>()).unwrap();
    write_dump_header(&mut h, &plane::<f32>()).unwrap();
    assert_eq!(h, "chart: Burrau\nchart: EuclidPlane\n");
}

#[test]
fn burrau_chart_flags() {
    check_flags(
        nu_k::<f64>().forbids_energy_normalisation(),
        plane::<f64>().forbids_energy_normalisation(),
    );
    // Through ChartDecode, the (ν, K) chart decodes as `decode` does.
    let c = nu_k::<f32>();
    assert_eq!(
        c.decode_at::<Defaults, f32>(0.3, 0.6),
        decode(&c, 0.3, 0.6).0
    );
}

negative_control!(
    burrau_chart_flags,
    "a (ν, K) chart that allows η_E must fail the check",
    expected = "the (ν, K) chart allows η_E",
    check_flags(false, plane::<f64>().forbids_energy_normalisation())
);
