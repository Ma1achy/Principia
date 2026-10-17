//! QA tests for TASK-M2-10, written from the requirements (REQ-CHART-021, REQ-CHART-022, REQ-CHART-023,
//! REQ-CHART-013, REQ-SYS-016, REQ-VAL-016; chart_reference §4.1–§4.5, §2.2, §5.2; chart_decoder_contract Part 5;
//! R-22), not from the implementation. Each check runs at the f64 and the f32 instantiation of the one source,
//! natively (R-265).
//! - `qa_m210_euclid_*` (REQ-CHART-021): Euclid's map against a brute force (every primitive triple with `c ≤ 400`
//!   arises from exactly one `(m, n)`, up to the leg swap); §4.1's table rows; the normalised Burrau system checked
//!   *geometrically*: right angle at body 0 at the origin, hypotenuse 1, each mass proportional to the side opposite its
//!   body, `Σm = 1`, and `ρ`, `λ` the Jacobi vectors by their definition (`ρ = r₁ − r₀`, `λ = r₂ − CoM₀₁`) as well as
//!   §4.2's closed forms; the convention stated is the normalised one.
//! - `qa_m210_nu_*` (REQ-CHART-022): the `ν` embed at every primitive `ν = n/m` (`m ≤ 40`) reproduces the discrete
//!   family; `θ(ν) = arctan((1 − ν²)/(2ν))` and is the triangle's own acute angle at body 2; continuity of positions,
//!   masses and Jacobi vectors (embed and decoded IC) by a constant-free refinement test (a jump survives refinement,
//!   a continuous function's increments shrink with the step); the `(ν, K)` chart's momenta are §2.2's construction
//!   (`Σp = 0`, `L_z = 0`, `K(p) = K_max·v^γ_K`), its configuration the rest decode's, and its seed R-82's largest-norm
//!   seed, decided exactly at `3ν = 1` (checked in integer arithmetic) and visible in the momenta's direction.
//! - `qa_m210_plane_*` (REQ-CHART-023): `m(u)`, `ν(v)` and their defaults; varying `u` at fixed `v` gives bitwise
//!   identical ICs, through every decode entry; the plane decodes `ν(v)` at rest with Burrau masses; the overlay marks
//!   exactly the primitive lattice points by brute force, at their `(u, v)`.
//! - `qa_m210_annotation_*` (REQ-CHART-013): the `m` axis is annotation-only; a 2D density on the plane is refused, a
//!   per-value statistic along `ν` admitted, along `m` refused; the chart is labelled a 1-swept family; the `(ν, K)`
//!   chart, with no annotation axis, admits a density; the decoder ignores `u` even when it is not finite.
//! - `qa_m210_body_indices` (REQ-SYS-016): pair id `k` names the side opposite body `k` (pair 0 = (1, 2), 1 = (2, 0),
//!   2 = (0, 1)), so bodies 0 and 1 collide as pair 2, carried through `sample_descriptor`'s `dmin_pair` bits; the
//!   literature's bodies 1–3 are 0–2; `ICDescriptor`'s `m0 m1 m2` of `(3, 4, 5)` are `(5, 4, 3)/12`, body 0 at the
//!   right angle.
//! - `qa_m210_nu_half` (REQ-VAL-016): `ν = 1/2` (the plane and the `(ν, K)` chart at `K = 0`) against the `(2, 1)`
//!   construction and against the classical configuration canonicalised *here*, by hand (CoM `(1, 1)`, `I = 5`,
//!   `ℓ = √5` in historical units), within REQ-VAL-118's proposed tolerance.
//! - `qa_m210_validate`: `ν ∉ (0, 1)` is refused naming the Burrau constraint (chart_reference §5.1).
//!
//! **Tolerances.** REQ-VAL-016's is REQ-VAL-118's (the register's `burrau_nu_half_tol`, proposed). The construction's
//! `Σp`, `L_z` and `K` residuals are REQ-DEC-046's proposed per-precision tolerances (`TOL_P`, `TOL_L`, `TOL_K`, as
//! `invariant_construction.rs` states them). The remaining bounds are rounding bounds of the few operations each formula
//! writes, every quantity of magnitude at most 1 with derivatives in `ν` at most 2 on `(0, 1)` (§4.3's rational
//! functions): `RND = 32` eps absolute. Continuity has no constant: the refinement ratio `1/4` sits between a continuous
//! function's `1/16` and a jump's `1`. The seed is classified at `√eps` of `Σ|p|`: the two seeds are orthogonal, so one
//! gives a component O(1), the other O(eps).
//!
//! **Domain** (pitfalls §10): the stateless decode, its inputs held fixed; no trajectory.
//!
//! Each test has a registered negative control naming the panic it expects (R-176, R-212).

use std::fmt::Debug;

use kernel::burrau::{euclid, from_literature, literature_pair_id, pair_id, System, Triple};
use kernel::burrau::{Convention, CONVENTION};
use kernel::chart::burrau::{
    acute_angle, decode as nu_k_decode, decode_plane, embed, seed, RestBlocks,
};
use kernel::chart::invariant::WarpReal;
use kernel::chart::{
    admit_statistic, family_label, swept, Axis, AxisMeta, Burrau, Chart, ChartDecode, Constraint,
    EuclidPlane, Statistic, ValidationResult,
};
use kernel::decode::invariant::{Outcome, Seed};
use kernel::decode::{decode_with, Decoded, Ic, Latent};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::{sd_dmin_pair, set_dmin_pair, PayloadReal};
use validation::{
    negative_control, pair_bodies, BURRAU_NU_HALF_TOL, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX,
    EUCLID_NU_MIN, GAMMA_K, K_MAX,
};

/// Rounding bound, in eps, absolute, for a quantity of magnitude ≤ 1 (the module's note).
const RND: f64 = 32.0;
/// `|Σpᵢ|` in units of `eps·Σ|pᵢ|` (REQ-DEC-046, proposed).
const TOL_P: f64 = 4.0;
/// `|L_z(p) − L_z|` in units of `eps·(|L_z| + √(2K*))` (REQ-DEC-046, proposed).
const TOL_L: f64 = 4.0;
/// `|K(p) − K*|` in units of `eps·K*` (REQ-DEC-046, proposed).
const TOL_K: f64 = 8.0;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// An instantiation under test.
trait Q: LinkReal + PayloadReal + WarpReal + Copy + Debug + PartialEq + PartialOrd {
    const EPS: f64;
    const LABEL: &'static str;
    fn f(self) -> f64;
    fn of(x: f64) -> Self;
    /// Whether `3·self ≤ 1` holds exactly, for a finite `self ∈ [1/1024, 1)`, decided in integer arithmetic.
    fn three_at_most_one(self) -> bool;
    /// The float `k` steps above `self` (below for negative `k`), `self` positive.
    fn step(self, k: i64) -> Self;
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
    fn three_at_most_one(self) -> bool {
        let b = self.to_bits();
        let mant = (b & ((1u64 << 52) - 1)) | (1u64 << 52);
        let e = ((b >> 52) & 0x7ff) as i64 - 1075;
        assert!((-80..0).contains(&e), "ν = {self} outside the exact range");
        3 * mant as u128 <= 1u128 << (-e)
    }
    fn step(self, k: i64) -> Self {
        f64::from_bits((self.to_bits() as i64 + k) as u64)
    }
}

impl Q for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const LABEL: &'static str = "f32";
    fn f(self) -> f64 {
        self as f64
    }
    fn of(x: f64) -> Self {
        x as f32
    }
    fn three_at_most_one(self) -> bool {
        let b = self.to_bits();
        let mant = (b & ((1u32 << 23) - 1)) | (1u32 << 23);
        let e = ((b >> 23) & 0xff) as i64 - 150;
        assert!((-40..0).contains(&e), "ν = {self} outside the exact range");
        3 * mant as u128 <= 1u128 << (-e)
    }
    fn step(self, k: i64) -> Self {
        f32::from_bits((self.to_bits() as i64 + k) as u32)
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// A deterministic low-discrepancy sequence on `(0, 1)`: the golden-ratio sequence.
fn golden(i: usize) -> f64 {
    (0.5 + i as f64 * 0.618_033_988_749_894_9).fract()
}

fn plane<R: Q>() -> EuclidPlane<R> {
    EuclidPlane {
        m_range: [R::of(EUCLID_M_MIN.number()), R::of(EUCLID_M_MAX.number())],
        nu_range: [R::of(EUCLID_NU_MIN.number()), R::of(EUCLID_NU_MAX.number())],
    }
}

fn nu_k<R: Q>() -> Burrau<R> {
    Burrau {
        nu_range: [R::of(EUCLID_NU_MIN.number()), R::of(EUCLID_NU_MAX.number())],
        k_max: R::of(K_MAX.number()),
        gamma_k: R::of(GAMMA_K.number()),
    }
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn norm(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn w2<R: Q>(v: [R; 2]) -> [f64; 2] {
    [v[0].f(), v[1].f()]
}

// ── qa_m210_euclid (REQ-CHART-021) ──────────────────────────────────────────────────────────────────────────────

/// Euclid's map: a triple exactly at primitive `(m, n)` with `a = m² − n²`, `b = 2mn`, `c = m² + n²`; §4.1's rows;
/// every primitive triple with `c ≤ 400` from exactly one `(m, n)`, legs in either order.
fn check_euclid(f: fn(u32, u32) -> Option<Triple>) {
    for ((m, n), (a, b, c)) in [
        ((2, 1), (3, 4, 5)),
        ((3, 2), (5, 12, 13)),
        ((4, 1), (15, 8, 17)),
        ((4, 3), (7, 24, 25)),
    ] {
        assert_eq!(
            f(m, n),
            Some(Triple { a, b, c }),
            "§4.1's row ({m}, {n}) is not ({a}, {b}, {c})"
        );
    }
    for m in 0..=40u32 {
        for n in 0..=40u32 {
            let prim = m > n && n > 0 && gcd(m as u64, n as u64) == 1 && (m - n) % 2 == 1;
            let (mm, nn) = (m as u64, n as u64);
            let want = prim.then(|| Triple {
                a: mm * mm - nn * nn,
                b: 2 * mm * nn,
                c: mm * mm + nn * nn,
            });
            assert_eq!(
                f(m, n),
                want,
                "Euclid's map at ({m}, {n}) is not the primitive triple rule"
            );
        }
    }
    // Every primitive triple (a, b, c), c ≤ 400, arises exactly once up to the leg swap.
    for c in 1..=400u64 {
        for a in 1..c {
            let bb = c * c - a * a;
            let b = (bb as f64).sqrt().round() as u64;
            if b * b != bb || a > b || gcd(a, b) != 1 {
                continue;
            }
            let hits = (1..=21u32)
                .flat_map(|m| (1..m).map(move |n| (m, n)))
                .filter_map(|(m, n)| f(m, n))
                .filter(|t| t.c == c && ((t.a, t.b) == (a, b) || (t.a, t.b) == (b, a)))
                .count();
            assert_eq!(
                hits, 1,
                "the primitive triple ({a}, {b}, {c}) arises {hits} times"
            );
        }
    }
}

#[test]
fn qa_m210_euclid_triples() {
    check_euclid(euclid);
}

negative_control!(
    qa_m210_euclid_triples,
    "A map that also accepts m − n even is not Euclid's primitive rule",
    expected = "is not the primitive triple rule",
    check_euclid(|m, n| {
        if m > n && n > 0 {
            let (m, n) = (m as u64, n as u64);
            Some(Triple {
                a: m * m - n * n,
                b: 2 * m * n,
                c: m * m + n * n,
            })
        } else {
            None
        }
    })
);

/// §4.2's system for the triple `(a, b, c)`, checked by geometry and by definition at `R`: `r₀ = 0`, the right angle
/// at body 0, the hypotenuse `|r₂ − r₁| = 1`, legs `a/c` and `b/c`; each mass proportional to the side opposite its
/// body, `Σm = 1`; `ρ = r₁ − r₀`, `λ = r₂ − CoM₀₁`; and §4.2's closed forms.
fn check_system<R: Q>(s: &System<R>, (a, b, c): (u64, u64, u64)) {
    let tol = RND * R::EPS;
    let r = s.r.map(w2);
    let m = s.m.map(|x| x.f());
    let (a, b, c) = (a as f64, b as f64, c as f64);
    assert_eq!(r[0], [0.0, 0.0], "body 0 is not at the origin");
    let (e1, e2) = (sub(r[1], r[0]), sub(r[2], r[0]));
    assert!(
        dot(e1, e2).abs() <= tol,
        "the right angle is not at body 0 ({})",
        R::LABEL
    );
    assert!(
        (norm(sub(r[2], r[1])) - 1.0).abs() <= tol,
        "the hypotenuse is not normalised to 1 ({})",
        R::LABEL
    );
    assert!(
        (norm(e1) - a / c).abs() <= tol && (norm(e2) - b / c).abs() <= tol,
        "the legs are not a/c, b/c"
    );
    // Each mass equal to its opposite side, over the perimeter: body k's opposite side is between the other two.
    let opposite = [
        norm(sub(r[2], r[1])),
        norm(sub(r[2], r[0])),
        norm(sub(r[1], r[0])),
    ];
    let per: f64 = opposite.iter().sum();
    for k in 0..3 {
        assert!(
            (m[k] - opposite[k] / per).abs() <= tol,
            "mass {k} is not its opposite side over the perimeter ({}): {m:?}",
            R::LABEL
        );
    }
    assert!((m.iter().sum::<f64>() - 1.0).abs() <= tol, "Σm is not 1");
    // The Jacobi vectors, by their definition.
    let m01 = m[0] + m[1];
    let com01 = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    let (rho, lambda) = (w2(s.rho), w2(s.lambda));
    assert!(norm(sub(rho, e1)) <= tol, "ρ is not r₁ − r₀ ({})", R::LABEL);
    assert!(
        norm(sub(lambda, sub(r[2], com01))) <= tol,
        "λ is not r₂ − CoM₀₁ ({})",
        R::LABEL
    );
    // §4.2's closed forms.
    assert!(norm(sub(rho, [a / c, 0.0])) <= tol, "ρ is not (a/c, 0)");
    assert!(
        norm(sub(lambda, [-a * b / (c * (b + c)), b / c])) <= tol,
        "λ is not (−ab/(c(b+c)), b/c) ({})",
        R::LABEL
    );
}

#[test]
fn qa_m210_euclid_burrau_system() {
    // (2, 1): exactly (5, 4, 3)/12, at both precisions.
    let t = euclid(2, 1).unwrap();
    let s64: System<f64> = t.burrau();
    let s32: System<f32> = t.burrau();
    for (k, want) in [5.0, 4.0, 3.0].into_iter().enumerate() {
        assert!(
            (s64.m[k] - want / 12.0).abs() <= f64::EPSILON,
            "m{k} of (3, 4, 5) is not {want}/12"
        );
        assert!(
            ((s32.m[k] as f64) - want / 12.0).abs() <= f32::EPSILON as f64,
            "m{k} (f32) is not {want}/12"
        );
    }
    assert_eq!(s64.r[1], [0.6, 0.0]);
    assert_eq!(s64.r[2], [0.0, 0.8]);
    for m in 2..=24u32 {
        for n in 1..m {
            if let Some(t) = euclid(m, n) {
                check_system::<f64>(&t.burrau(), (t.a, t.b, t.c));
                check_system::<f32>(&t.burrau(), (t.a, t.b, t.c));
            }
        }
    }
    // The convention in use is stated, and is the normalised one, on both charts.
    assert_eq!(CONVENTION, Convention::Normalised);
    assert_eq!(plane::<f64>().convention(), Convention::Normalised);
    assert_eq!(nu_k::<f64>().convention(), Convention::Normalised);
    let said = nu_k::<f64>().convention_statement().to_lowercase();
    assert!(
        said.contains("normalised"),
        "the chart does not state the normalised convention: {said}"
    );
    assert_eq!(
        plane::<f64>().convention_statement(),
        nu_k::<f64>().convention_statement()
    );
}

negative_control!(
    qa_m210_euclid_burrau_system,
    "The historical mass order on the normalised triangle (m = (a, b, c)/(a+b+c)) puts no mass on its opposite side",
    expected = "is not its opposite side over the perimeter",
    check_system::<f64>(
        &{
            let mut s: System<f64> = euclid(2, 1).unwrap().burrau();
            s.m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
            s
        },
        (3, 4, 5)
    )
);

// ── qa_m210_nu (REQ-CHART-022) ──────────────────────────────────────────────────────────────────────────────────

/// The system's quantities as one list: masses, positions, `ρ`, `λ`.
fn flat<R: Q>(s: &System<R>) -> Vec<f64> {
    let mut v: Vec<f64> = s.m.iter().map(|x| x.f()).collect();
    v.extend(s.r.iter().flatten().map(|x| x.f()));
    v.extend(s.rho.iter().chain(&s.lambda).map(|x| x.f()));
    v
}

fn largest(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// The embed at every primitive `ν = n/m`, `m ≤ 40`, is the triple's system within [`RND`] eps.
fn check_primitive_nu<R: Q>(at: fn(R) -> System<R>) {
    let mut seen = 0;
    for m in 2..=40u32 {
        for n in 1..m {
            let Some(t) = euclid(m, n) else { continue };
            seen += 1;
            let nu = R::of(n as f64) / R::of(m as f64);
            let d = largest(&flat(&at(nu)), &flat(&t.burrau::<R>())) / R::EPS;
            assert!(
                d <= RND,
                "ν = {n}/{m} is {d:.1} eps from the triple ({}, {}, {}) at {}",
                t.a,
                t.b,
                t.c,
                R::LABEL
            );
        }
    }
    assert!(seen > 200, "only {seen} primitive triples checked");
}

#[test]
fn qa_m210_nu_reproduces_the_family() {
    check_primitive_nu::<f64>(embed);
    check_primitive_nu::<f32>(embed);
}

negative_control!(
    qa_m210_nu_reproduces_the_family,
    "An embed with the legs swapped (a ∝ 2ν, b ∝ 1 − ν²) does not reproduce the family",
    expected = "eps from the triple",
    check_primitive_nu::<f64>(|nu| System::of_sides(nu + nu, 1.0 - nu * nu, 1.0 + nu * nu))
);

/// `θ(ν) = arctan((1 − ν²)/(2ν))`, and it is the embed's angle at body 2 (opposite the leg `a`), within [`RND`] eps.
fn check_angle<R: Q>(theta: fn(R) -> R) {
    for i in 0..2000 {
        let nu = 0.001 + 0.998 * golden(i);
        let r = R::of(nu);
        let nu = r.f();
        let want = ((1.0 - nu * nu) / (2.0 * nu)).atan();
        let got = theta(r).f();
        assert!(
            (got - want).abs() <= RND * R::EPS,
            "θ({nu}) = {got}, not arctan((1 − ν²)/(2ν)) = {want}"
        );
        let s = embed(r);
        let p = s.r.map(w2);
        let (u, w) = (sub(p[0], p[2]), sub(p[1], p[2]));
        let at2 = (u[0] * w[1] - u[1] * w[0]).abs().atan2(dot(u, w));
        assert!(
            (at2 - want).abs() <= RND * R::EPS,
            "θ({nu}) is not the angle at body 2"
        );
    }
    // §4.1's table: (2, 1) → 36.9°, (3, 2) → 22.6°, (4, 1) → 61.9°, (4, 3) → 16.3°, to the table's 0.05°.
    for (n, m, deg) in [
        (1.0, 2.0, 36.9),
        (2.0, 3.0, 22.6),
        (1.0, 4.0, 61.9),
        (3.0, 4.0, 16.3),
    ] {
        let got = theta(R::of(n) / R::of(m)).f().to_degrees();
        assert!(
            (got - deg).abs() <= 0.05,
            "θ at ({m}, {n}) is {got}°, not the table's {deg}°"
        );
    }
}

#[test]
fn qa_m210_nu_acute_angle() {
    check_angle::<f64>(acute_angle);
    check_angle::<f32>(acute_angle);
}

negative_control!(
    qa_m210_nu_acute_angle,
    "The complementary angle arctan(2ν/(1 − ν²)) is not θ(ν)",
    expected = "not arctan((1 − ν²)/(2ν))",
    check_angle::<f64>(|nu| (2.0 * nu / (1.0 - nu * nu)).atan())
);

/// The largest increment of `f` between neighbours of the grid of `n` steps on `[lo, hi]`.
fn max_step(f: &dyn Fn(f64) -> Vec<f64>, lo: f64, hi: f64, n: usize) -> f64 {
    let mut prev = f(lo);
    let mut worst: f64 = 0.0;
    for i in 1..=n {
        let next = f(lo + (hi - lo) * i as f64 / n as f64);
        worst = worst.max(largest(&prev, &next));
        prev = next;
    }
    worst
}

/// `f` is continuous on `[lo, hi]`: refining the grid sixteenfold shrinks its largest increment at least fourfold (the
/// module's note); a jump keeps it.
fn check_continuous(what: &str, f: &dyn Fn(f64) -> Vec<f64>, lo: f64, hi: f64) {
    let coarse = max_step(f, lo, hi, 2_000);
    let fine = max_step(f, lo, hi, 32_000);
    assert!(coarse > 0.0, "{what} does not vary with ν");
    assert!(
        fine <= coarse / 4.0,
        "{what} is not continuous in ν: its largest increment {coarse:e} only falls to {fine:e} on refinement"
    );
}

/// The decoded IC's masses and positions.
fn ic_cfg<R: Q>(d: &Decoded<R>) -> Vec<f64> {
    let mut v: Vec<f64> = d.ic.m.iter().map(|x| x.f()).collect();
    v.extend(d.ic.r.iter().flatten().map(|x| x.f()));
    v
}

#[test]
fn qa_m210_nu_continuous() {
    let (lo, hi) = (EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number());
    check_continuous("the embed (f64)", &|nu| flat(&embed(nu)), lo, hi);
    check_continuous("the embed (f32)", &|nu| flat(&embed(nu as f32)), lo, hi);
    let p = plane::<f64>();
    check_continuous(
        "the plane's decoded IC",
        &|v| ic_cfg(&decode_plane(&p, v)),
        0.0,
        1.0,
    );
    let c = nu_k::<f64>();
    check_continuous(
        "the (ν, K) chart's masses and positions",
        &|u| ic_cfg(&nu_k_decode(&c, u, 0.7).0),
        0.0,
        1.0,
    );
}

negative_control!(
    qa_m210_nu_continuous,
    "An embed that jumps at ν = 1/2 is not continuous",
    expected = "is not continuous in ν",
    check_continuous(
        "a jumping embed",
        &|nu| flat(&embed(if nu > 0.5 { nu + 0.01 } else { nu })),
        1.0 / 32.0,
        31.0 / 32.0
    )
);

/// `L_z(p) = Σ (r × p)_z` and `K(p) = Σ |p|²/2m`, in f64.
fn lz_k<R: Q>(ic: &Ic<R>) -> (f64, f64) {
    let (mut lz, mut k) = (0.0, 0.0);
    for i in 0..3 {
        let (r, p, m) = (w2(ic.r[i]), w2(ic.p[i]), ic.m[i].f());
        lz += r[0] * p[1] - r[1] * p[0];
        k += dot(p, p) / (2.0 * m);
    }
    (lz, k)
}

/// A `(ν, K)` decode under check: the decode and the construction's outcome.
type NuKDecode<R> = fn(&Burrau<R>, R, R) -> (Decoded<R>, Outcome);

/// The `(ν, K)` chart at `(u, v)`: §2.2's construction at `L_z = 0`, `K* = K_max·v^γ_K` (R-113, §4.5's `K(v)`), on the
/// rest decode's masses and configuration, its seed R-82's (the larger mass-weighted Jacobi norm, `(ρ, 0)` on a tie),
/// decided exactly at `3ν = 1`.
fn check_construction<R: Q>(dec: NuKDecode<R>, u: f64, v: f64) {
    let c = nu_k::<R>();
    let (u, v) = (R::of(u), R::of(v));
    let (d, outcome) = dec(&c, u, v);
    assert_eq!(
        outcome,
        Outcome::Feasible,
        "the construction is not feasible at L_z = 0"
    );
    let k_star = K_MAX.number() * v.f().powf(GAMMA_K.number());
    let p = d.ic.p.map(w2);
    let abs: f64 = p.iter().map(|q| q[0].abs() + q[1].abs()).sum();
    let sum = [p[0][0] + p[1][0] + p[2][0], p[0][1] + p[1][1] + p[2][1]];
    assert!(
        norm(sum) <= TOL_P * R::EPS * abs.max(f64::MIN_POSITIVE),
        "Σp ≠ 0 ({})",
        R::LABEL
    );
    let (lz, k) = lz_k(&d.ic);
    assert!(
        lz.abs() <= TOL_L * R::EPS * (2.0 * k_star).sqrt().max(f64::MIN_POSITIVE),
        "L_z(p) = {lz} ≠ 0"
    );
    // K* here is computed in f64 from the same v; the instantiation's own K* is one powf and one product from it, 2
    // eps more.
    assert!(
        (k - k_star).abs() <= (TOL_K + 2.0) * R::EPS * k_star,
        "K(p) = {k} is not K* = {k_star} at v = {}",
        v.f()
    );
    // The configuration and masses are the rest decode's at the same ν: K only sets the momenta.
    let rest = decode_plane(
        &EuclidPlane {
            m_range: [R::of(1.0), R::of(32.0)],
            nu_range: c.nu_range,
        },
        u,
    );
    assert_eq!(
        (d.ic.m, d.ic.r),
        (rest.ic.m, rest.ic.r),
        "K moves the configuration or the masses"
    );
    // The seed: R-82's largest mass-weighted Jacobi norm, from the decoded configuration (the norms of (ρ, 0) and
    // (0, λ) are ‖ρ̃‖² and ‖λ̃‖²), decided exactly at the tie 3ν = 1; and in the momenta's direction: (ρ, 0) moves
    // only the inner pair (p₂ = 0), (0, λ) moves bodies 0 and 1 together (v₀ = v₁).
    let nu = c.nu(u);
    let rho = w2(d.config.rho_t);
    let lam = w2(d.config.lambda_t);
    let rho_wins = if (3.0 * nu.f() - 1.0).abs() > 1e-4 {
        dot(rho, rho) >= dot(lam, lam)
    } else {
        nu.three_at_most_one()
    };
    if k_star > 0.0 {
        let gap = R::EPS.sqrt() * abs;
        let v0 = [p[0][0] / d.ic.m[0].f(), p[0][1] / d.ic.m[0].f()];
        let v1 = [p[1][0] / d.ic.m[1].f(), p[1][1] / d.ic.m[1].f()];
        let took_rho = norm(p[2]) <= gap;
        let took_lambda = norm(sub(v0, v1)) * d.ic.m[0].f() <= gap;
        assert!(
            took_rho != took_lambda,
            "the momenta are neither seed's at ν = {}",
            nu.f()
        );
        assert_eq!(
            took_rho,
            rho_wins,
            "ν = {:e} took the wrong seed ({})",
            nu.f(),
            R::LABEL
        );
    }
    assert_eq!(
        seed(nu) == Seed::Rho,
        rho_wins,
        "seed({:e}) is not R-82's ({})",
        nu.f(),
        R::LABEL
    );
}

#[test]
fn qa_m210_nu_k_construction() {
    for i in 0..400 {
        let (u, v) = (golden(i), golden(i + 7919));
        check_construction::<f64>(nu_k_decode, u, v);
        check_construction::<f32>(nu_k_decode, u, v);
    }
    for v in [0.0, 1.0, 1.5] {
        check_construction::<f64>(nu_k_decode, 0.25, v);
        check_construction::<f32>(nu_k_decode, 0.25, v);
    }
    // At the tie 3ν = 1, ν = 1/3 ± a few ulps, the seed is decided exactly.
    let c64 = nu_k::<f64>();
    let c32 = nu_k::<f32>();
    let third64 = (1.0 / 3.0 - c64.nu_range[0]) / (c64.nu_range[1] - c64.nu_range[0]);
    for k in -40..=40 {
        check_construction::<f64>(nu_k_decode, third64.step(k), 0.6);
        let u32_third = ((1.0 / 3.0 - c32.nu_range[0].f())
            / (c32.nu_range[1].f() - c32.nu_range[0].f())) as f32;
        check_construction::<f32>(nu_k_decode, u32_third.step(k).f(), 0.6);
    }
    for k in -2000..=2000 {
        let nu = (1.0f64 / 3.0).step(k);
        assert_eq!(
            seed(nu) == Seed::Rho,
            nu.three_at_most_one(),
            "seed({nu:e}) is not decided exactly"
        );
        let nu = (1.0f32 / 3.0).step(k);
        assert_eq!(
            seed(nu) == Seed::Rho,
            nu.three_at_most_one(),
            "seed({nu:e}) (f32) is not decided exactly"
        );
    }
}

/// The `(ν, K)` decode with `K` halved: not `K_max·v^γ_K`.
#[cfg(feature = "controls")]
fn half_k(c: &Burrau<f64>, u: f64, v: f64) -> (Decoded<f64>, Outcome) {
    nu_k_decode(
        &Burrau {
            k_max: c.k_max / 2.0,
            ..*c
        },
        u,
        v,
    )
}

negative_control!(
    qa_m210_nu_k_construction,
    "A (ν, K) chart whose K is half of K_max·v^γ_K is not §2's construction at K(v)",
    expected = "is not K* =",
    check_construction::<f64>(half_k, 0.3, 0.8)
);

// ── qa_m210_plane (REQ-CHART-023) ───────────────────────────────────────────────────────────────────────────────

/// The plane's maps and defaults: `m(u) = m_min·(m_max/m_min)^u` on `[1, 32]`, `ν(v)` linear on `[1/32, 31/32]`.
fn check_maps(m_of: &dyn Fn(f64) -> f64, nu_of: &dyn Fn(f64) -> f64) {
    assert_eq!(
        (
            EUCLID_M_MIN.number(),
            EUCLID_M_MAX.number(),
            EUCLID_NU_MIN.number(),
            EUCLID_NU_MAX.number()
        ),
        (1.0, 32.0, 1.0 / 32.0, 31.0 / 32.0),
        "the plane's defaults are not §4.5's"
    );
    for i in 0..=64 {
        let t = i as f64 / 64.0;
        let m = 32f64.powf(t);
        assert!(
            (m_of(t) - m).abs() <= RND * f64::EPSILON * m,
            "m({t}) = {} is not 32^{t}",
            m_of(t)
        );
        let nu = 1.0 / 32.0 + 30.0 / 32.0 * t;
        assert!(
            (nu_of(t) - nu).abs() <= RND * f64::EPSILON,
            "ν({t}) = {} is not linear on [1/32, 31/32]",
            nu_of(t)
        );
    }
}

#[test]
fn qa_m210_plane_maps() {
    let c = plane::<f64>();
    check_maps(&|u| c.m(u), &|v| c.nu(v));
}

negative_control!(
    qa_m210_plane_maps,
    "A plane whose m axis is linear on [1, 32] is not §4.5's m(u)",
    expected = "is not 32^",
    check_maps(&|u| 1.0 + 31.0 * u, &|v| plane::<f64>().nu(v))
);

/// Varying `u` at fixed `v` gives identical ICs (every decode entry), the IC being `ν(v)`'s Burrau system at rest.
fn check_identical<R: Q>(dec: fn(&EuclidPlane<R>, R, R) -> Decoded<R>) {
    let c = plane::<R>();
    for j in 0..200 {
        let v = R::of(golden(j));
        let base = dec(&c, R::of(0.0), v);
        for i in 0..50 {
            let u = R::of(golden(i + 1000));
            assert_eq!(
                dec(&c, u, v),
                base,
                "the decode at v = {} varies with u ({})",
                v.f(),
                R::LABEL
            );
        }
        assert!(
            base.ic.p.iter().flatten().all(|x| x.f() == 0.0),
            "the plane's IC is not at rest"
        );
        let s = embed(c.nu(v));
        let w: f64 = s.m.iter().map(|x| x.f()).sum();
        for k in 0..3 {
            assert!(
                (base.ic.m[k].f() - s.m[k].f() / w).abs() <= RND * R::EPS,
                "the plane's masses are not Burrau's"
            );
        }
    }
}

fn plane_entry<R: Q>(c: &EuclidPlane<R>, u: R, v: R) -> Decoded<R>
where
    f64: From<R>,
{
    let a = Chart::decode::<Defaults, R>(c, u, v);
    let b = ChartDecode::decode_at::<Defaults, R>(c, u, v);
    let d = kernel::chart::decode::<Defaults, R, _>(c, u, v);
    assert_eq!(a, b, "Chart::decode and ChartDecode::decode_at differ");
    assert_eq!(a, d, "Chart::decode and chart::decode differ");
    a
}

#[test]
fn qa_m210_plane_identical_ics() {
    check_identical::<f64>(plane_entry);
    check_identical::<f32>(plane_entry);
}

/// A plane whose `ν` reads `u` slightly.
#[cfg(feature = "controls")]
fn leaky(c: &EuclidPlane<f64>, u: f64, v: f64) -> Decoded<f64> {
    decode_plane(c, v + 1e-9 * u)
}

negative_control!(
    qa_m210_plane_identical_ics,
    "A plane whose ν depends on u, however slightly, varies with u",
    expected = "varies with u",
    check_identical::<f64>(leaky)
);

/// The primitive-triple overlay marks exactly the lattice points `m, n ∈ ℤ`, `m > n`, `gcd = 1`, `m − n` odd inside
/// the plane, each at the `(u, v)` where `m(u) = m` and `ν(v) = n/m`.
fn check_overlay(points: &[(u32, u32, f64, f64, bool)]) {
    let c = plane::<f64>();
    let marked: Vec<(u32, u32)> = points.iter().filter(|p| p.4).map(|p| (p.0, p.1)).collect();
    let mut want = Vec::new();
    for m in 1..=32u32 {
        for n in 1..m {
            let nu = n as f64 / m as f64;
            if gcd(m as u64, n as u64) == 1
                && (m - n) % 2 == 1
                && (1.0 / 32.0..=31.0 / 32.0).contains(&nu)
            {
                want.push((m, n));
            }
        }
    }
    let mut got = marked.clone();
    got.sort();
    want.sort();
    assert_eq!(
        got, want,
        "the overlay does not mark exactly the primitive-triple lattice points"
    );
    for &(m, n, u, v, _) in points {
        assert!(
            (c.m(u) - m as f64).abs() <= 1e-9 * m as f64,
            "({m}, {n}) is not at m(u) = {m}"
        );
        assert!(
            (c.nu(v) - n as f64 / m as f64).abs() <= 1e-12,
            "({m}, {n}) is not at ν(v) = n/m"
        );
    }
}

fn overlay() -> Vec<(u32, u32, f64, f64, bool)> {
    plane::<f64>()
        .lattice()
        .map(|p| (p.m, p.n, p.u, p.v, p.primitive))
        .collect()
}

#[test]
fn qa_m210_plane_overlay() {
    let pts = overlay();
    check_overlay(&pts);
    assert!(
        pts.iter().any(|p| (p.0, p.1) == (2, 1) && p.4),
        "(2, 1) is not marked"
    );
    assert!(
        pts.iter().any(|p| (p.0, p.1) == (3, 1) && !p.4),
        "(3, 1), m − n even, is marked or absent"
    );
    assert!(
        pts.iter().any(|p| (p.0, p.1) == (6, 3) && !p.4),
        "(6, 3), gcd 3, is marked or absent"
    );
}

negative_control!(
    qa_m210_plane_overlay,
    "An overlay that also marks m − n even points is not the primitive-triple overlay",
    expected = "does not mark exactly",
    check_overlay(
        &overlay()
            .into_iter()
            .map(|p| (p.0, p.1, p.2, p.3, p.4 || gcd(p.0 as u64, p.1 as u64) == 1))
            .collect::<Vec<_>>()
    )
);

// ── qa_m210_annotation (REQ-CHART-013) ──────────────────────────────────────────────────────────────────────────

/// The plane's axes: `m` annotation-only, a 2D density refused, a per-value statistic along `ν` admitted and along
/// `m` refused; labelled a 1-swept family rendered in 2D.
fn check_annotation(axes: [AxisMeta; 2]) {
    assert!(
        axes[0].annotation && axes[0].touches.is_empty(),
        "the m axis is not annotation-only"
    );
    assert!(!axes[1].annotation, "the ν axis is not swept");
    assert!(
        admit_statistic(axes, Statistic::Density2D).is_err(),
        "a 2D density across the annotation axis is admitted"
    );
    assert!(
        admit_statistic(axes, Statistic::PerValue(Axis::Vertical)).is_ok(),
        "per-value statistics along ν refused"
    );
    assert!(
        admit_statistic(axes, Statistic::PerValue(Axis::Horizontal)).is_err(),
        "a statistic along m is admitted"
    );
    assert_eq!(swept(axes), 1);
    assert_eq!(family_label(axes), "1-swept family rendered in 2D");
}

#[test]
fn qa_m210_annotation_axis() {
    check_annotation(plane::<f64>().axes());
    // The (ν, K) chart has no annotation axis: 2-swept, a density admitted.
    let k = nu_k::<f64>().axes();
    assert!(!k[0].annotation && !k[1].annotation);
    assert!(
        admit_statistic(k, Statistic::Density2D).is_ok(),
        "the 2-swept (ν, K) chart refuses a density"
    );
    // The decoder ignores the annotation axis, whatever its value, finite or not.
    let c = plane::<f64>();
    for v in [0.0, 0.3, 0.5, 1.0] {
        let base = plane_entry(&c, 0.5, v);
        for u in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -7.0, 1e9, -0.0] {
            assert_eq!(
                plane_entry(&c, u, v),
                base,
                "the decode at v = {v} reads u = {u}"
            );
        }
        assert_eq!(
            c.validate(f64::NAN, v),
            c.validate(0.5, v),
            "validation reads the annotation axis"
        );
    }
}

negative_control!(
    qa_m210_annotation_axis,
    "A plane whose m axis is declared swept admits a 2D density",
    expected = "the m axis is not annotation-only",
    check_annotation({
        let mut a = plane::<f64>().axes();
        a[0] = nu_k::<f64>().axes()[0];
        a
    })
);

// ── qa_m210_body_indices (REQ-SYS-016) ──────────────────────────────────────────────────────────────────────────

/// Pair id `k` names the side opposite body `k`: pair 0 = (1, 2), 1 = (2, 0), 2 = (0, 1), in either order, and the
/// collision of bodies 0 and 1 is pair 2 through the payload's map and `sample_descriptor`'s `dmin_pair` bits.
fn check_pairs(f: fn(u32, u32) -> Option<u32>) {
    let id = f(0, 1).expect("bodies 0 and 1 have no pair id");
    assert_eq!(
        id, 2,
        "the collision of bodies 0 and 1 reports pair id {id}, not 2"
    );
    let packed = set_dmin_pair(0, id);
    assert_eq!(
        pair_bodies()[sd_dmin_pair(packed) as usize],
        [0, 1],
        "pair id {id} does not read back as (0, 1)"
    );
    for (k, (i, j)) in [(0, (1, 2)), (1, (2, 0)), (2, (0, 1))] {
        assert_eq!(f(i, j), Some(k), "pair ({i}, {j}) is not pair id {k}");
        assert_eq!(f(j, i), Some(k), "pair ({j}, {i}) is not pair id {k}");
        assert!(i != k && j != k, "pair {k} contains body {k}");
        assert_eq!(
            pair_bodies()[k as usize],
            [i, j],
            "the payload's map is not R-22's at pair {k}"
        );
    }
}

#[test]
fn qa_m210_body_indices() {
    check_pairs(pair_id);
    assert_eq!(
        [1, 2, 3].map(from_literature),
        [Some(0), Some(1), Some(2)],
        "literature body k is not body k − 1"
    );
    assert_eq!(
        literature_pair_id(1, 2),
        Some(2),
        "the literature's inner pair 1–2 is not pair id 2"
    );
    // ICDescriptor's fields are m0 m1 m2, 0-based; (3, 4, 5) loads with (5, 4, 3)/12, body 0 at the right angle.
    for d in [
        decode_plane(&plane::<f64>(), 0.5),
        nu_k_decode(&nu_k::<f64>(), 0.5, 0.0).0,
    ] {
        let named = [d.descriptor.m0, d.descriptor.m1, d.descriptor.m2];
        for (k, want) in [5.0f64, 4.0, 3.0].into_iter().enumerate() {
            assert!(
                (named[k] - want / 12.0).abs() <= f64::EPSILON,
                "m{k} of (3, 4, 5) is not {want}/12: {named:?}"
            );
        }
        let r = d.ic.r.map(w2);
        assert!(
            dot(sub(r[1], r[0]), sub(r[2], r[0])).abs() <= RND * f64::EPSILON,
            "body 0 is not at the right angle"
        );
        assert!(
            norm(sub(r[1], r[0])) < norm(sub(r[2], r[0])),
            "body 1 is not on the short leg (3)"
        );
    }
}

negative_control!(
    qa_m210_body_indices,
    "A pair id naming the pair by its smaller body is not R-22's",
    expected = "reports pair id 0, not 2",
    check_pairs(|i, j| (i < 3 && j < 3 && i != j).then(|| i.min(j)))
);

// ── qa_m210_nu_half (REQ-VAL-016) ───────────────────────────────────────────────────────────────────────────────

/// The classical configuration in the canonical section, by hand: masses `(5, 4, 3)/12` on `r₀ = (0, 0)`,
/// `r₁ = (3, 0)`, `r₂ = (0, 4)` (historical units). CoM `(1, 1)`; `ρ = (3, 0)` already on `+x`, `λ_y > 0`, so no
/// rotation or mirror; `I = Σmᵢ|rᵢ − CoM|² = (5·2 + 4·5 + 3·10)/12 = 5`, so `ℓ = √5`; at rest.
fn classical() -> ([f64; 3], [[f64; 2]; 3]) {
    let l = 5f64.sqrt();
    (
        [5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0],
        [
            [-1.0 / l, -1.0 / l],
            [2.0 / l, -1.0 / l],
            [-1.0 / l, 3.0 / l],
        ],
    )
}

/// The `ν = 1/2` decode against the `(2, 1)` construction and the classical configuration, each within REQ-VAL-118's
/// tolerance, componentwise on the canonical `(m, r, p)`, in eps.
fn check_nu_half<R: Q>(d: &Decoded<R>) {
    let tol = BURRAU_NU_HALF_TOL.number();
    let flat_ic = |m: [f64; 3], r: [[f64; 2]; 3], p: [[f64; 2]; 3]| -> Vec<f64> {
        let mut v = m.to_vec();
        v.extend(r.iter().flatten());
        v.extend(p.iter().flatten());
        v
    };
    let got = flat_ic(d.ic.m.map(|x| x.f()), d.ic.r.map(w2), d.ic.p.map(w2));
    let (m, r) = classical();
    let classical = flat_ic(m, r, [[0.0; 2]; 3]);
    let t = euclid(2, 1).expect("(2, 1) is primitive");
    assert_eq!((t.a, t.b, t.c), (3, 4, 5), "(2, 1) is not (3, 4, 5)");
    let rest = decode_with(
        &mut RestBlocks::new(t.burrau::<R>()),
        &Latent::new([R::of(0.0); 8]),
    );
    for (what, reference) in [
        ("classical configuration", classical),
        (
            "(2, 1) construction",
            flat_ic(
                rest.ic.m.map(|x| x.f()),
                rest.ic.r.map(w2),
                rest.ic.p.map(w2),
            ),
        ),
    ] {
        let e = largest(&got, &reference) / R::EPS;
        eprintln!(
            "qa_m210_nu_half ({}): ν = 1/2 against the {what}: {e:.3} eps",
            R::LABEL
        );
        assert!(
            e <= tol,
            "ν = 1/2 is {e:.2} eps from the {what} at {}, past REQ-VAL-118's {tol}",
            R::LABEL
        );
    }
}

#[test]
fn qa_m210_nu_half() {
    assert_eq!(plane::<f64>().nu(0.5), 0.5, "the plane's ν(1/2) is not 1/2");
    assert_eq!(
        nu_k::<f64>().nu(0.5),
        0.5,
        "the (ν, K) chart's ν(1/2) is not 1/2"
    );
    check_nu_half::<f64>(&decode_plane(&plane::<f64>(), 0.5));
    check_nu_half::<f32>(&decode_plane(&plane::<f32>(), 0.5));
    check_nu_half::<f64>(&nu_k_decode(&nu_k::<f64>(), 0.5, 0.0).0);
    check_nu_half::<f32>(&nu_k_decode(&nu_k::<f32>(), 0.5, 0.0).0);
    assert_eq!(
        BURRAU_NU_HALF_TOL.provisional,
        Some("M2"),
        "REQ-VAL-118's tolerance is not marked pending the M2 gate"
    );
    // The fixture states the register's tolerance.
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/gates/burrau_nu_half/measurement.toml"),
    )
    .expect("the REQ-VAL-118 measurement reads");
    let line = text
        .lines()
        .find(|l| l.starts_with("tolerance"))
        .expect("the measurement states its tolerance");
    let held: f64 = line
        .split('=')
        .nth(1)
        .unwrap()
        .split('#')
        .next()
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(
        held,
        BURRAU_NU_HALF_TOL.number(),
        "the measurement's tolerance is not the register's"
    );
}

negative_control!(
    qa_m210_nu_half,
    "The decode at ν = 1/2 + 2⁻²⁰ is not the classical configuration",
    expected = "past REQ-VAL-118's",
    check_nu_half::<f64>(&decode_plane(
        &plane::<f64>(),
        0.5 + 1.0 / (1u64 << 20) as f64
    ))
);

// ── qa_m210_validate (R-26; chart_reference §5.1) ───────────────────────────────────────────────────────────────

/// `ν ∉ (0, 1)` is refused naming the Burrau charts' constraint; inside, `(u, v)` in the square is valid.
fn check_validate<C: Chart>(c: &C, outside: [(f64, f64); 2], inside: (f64, f64)) {
    for (u, v) in outside {
        assert_eq!(
            c.validate(u, v),
            ValidationResult::Reject {
                failed: Constraint::BurrauNu
            },
            "{} at ({u}, {v}), ν outside (0, 1), is not refused on ν ∈ (0, 1)",
            c.name()
        );
    }
    assert_eq!(
        c.validate(inside.0, inside.1),
        ValidationResult::Valid,
        "{} at {inside:?} is not valid",
        c.name()
    );
}

#[test]
fn qa_m210_validate() {
    // ν(t) = 1/32 + (30/32)·t: ν = 0 at t = −1/30, ν = 1 at t = 31/30.
    check_validate(&plane::<f64>(), [(0.5, -0.04), (0.5, 1.04)], (0.5, 0.5));
    check_validate(&nu_k::<f64>(), [(-0.04, 0.5), (1.04, 0.5)], (0.5, 0.5));
    // (ν, K): v < 0 is K < 0; the remedy keeps ν and puts K at 0.
    match nu_k::<f64>().validate(0.5, -0.25) {
        ValidationResult::Project { u, v, failed } => {
            assert_eq!((u, v, failed), (0.5, 0.0, Constraint::KNonNegative));
        }
        other => panic!("(ν, K) at v < 0 is {other:?}, not projected to K = 0"),
    }
}

negative_control!(
    qa_m210_validate,
    "A point inside (0, 1) is not refused",
    expected = "is not refused on ν ∈ (0, 1)",
    check_validate(&plane::<f64>(), [(0.5, 0.5), (0.5, 0.5)], (0.5, 0.5))
);
