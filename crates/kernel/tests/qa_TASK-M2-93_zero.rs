//! QA tests for the invariant construction at zero moment of inertia, written from the source,
//! `principia_dd_decoder.md` § 2 ("`NO_QUALIFYING_SEED` … only where its `‖w⁽²⁾‖²_m` is 0, which is `I = 0` about the
//! CoM, a finite 0 there") and § 3.4 ("`‖w⁽²⁾‖²_m = 0`, which is `I = 0` about the CoM only → DEGENERATE"), not from
//! the implementation.
//!
//! - `qa_m293_zero_inertia_degenerate`: the other side of "`NO_QUALIFYING_SEED` unreachable for `I > 0`", which
//!   `qa_TASK-M2-93.rs` covers: at `I = 0` the seed in force raises `DEGENERATE` (`NO_QUALIFYING_SEED`), takes no
//!   seed, has `a = 0`, and every field it emits is finite, its expansion norm an exact 0, its momenta the rigid
//!   rotation `pᵢ = mᵢ·L_z·J rᵢ` (0 where every position is 0). Fixtures: a decoded configuration scaled by 0; the same
//!   scaled by a power of two below the precision's smallest subnormal, so every stored position underflows to 0 (a
//!   configuration collapsed toward a point, as the decode would store it); and every body at one point off the origin
//!   (`I` about the CoM 0, `I` about the origin not). At f64 and f32, feasible targets (`K* ≥ L_z²/2`), `L_z` 0 and not.
//!
//! **Tolerances.** Exact where the source says exact (the norm, `a`, the momenta at `r = 0`); a few roundings relative
//! for the rigid rotation off the origin, one product of three factors.
//!
//! **Domain** (pitfalls §10): the stateless construction, its inputs held fixed; no trajectory.
//!
//! The control: §3.4's formula taken literally, `w = r/√I` with `I = Σ mᵢ‖rᵢ‖²` unguarded, which is 0/0 at
//! `I = 0`; its norm and direction are NaN, so the check must fail.

use kernel::decode::config::{self, Config};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::invariant::{construct_with, Construction, ExactSign, Outcome, Seeding};
use kernel::decode::mass::{self, Masses};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use kernel::real::IeeeBits;
use validation::negative_control;

trait Q: LinkReal + PayloadReal + IeeeBits + ExactSign + Copy {
    const LABEL: &'static str;
    /// A power of two whose product with any decoded position (`|r| ≤ 2`) rounds to 0.
    const UNDER: Self;
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    fn eps() -> f64;
}

impl Q for f64 {
    const LABEL: &'static str = "f64";
    // 2^-1076 is below half the smallest subnormal (2^-1074) for |r| < 2.
    const UNDER: f64 = f64::MIN_POSITIVE * f64::EPSILON * 0.125;
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn eps() -> f64 {
        f64::EPSILON
    }
}

impl Q for f32 {
    const LABEL: &'static str = "f32";
    // 2^-152, below half the smallest subnormal (2^-149) for |r| < 2.
    const UNDER: f32 = f32::MIN_POSITIVE * f32::EPSILON * 0.125;
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        f64::from(self)
    }
    fn eps() -> f64 {
        f64::from(f32::EPSILON)
    }
}

/// A construction on a configuration: masses, configuration, `L_z`, `K*`.
type Build<R> = fn(&Masses<R>, &Config<R>, R, R) -> Construction<R>;

fn in_force<R: Q>(m: &Masses<R>, c: &Config<R>, lz: R, k: R) -> Construction<R> {
    construct_with(m, c, Seeding::Expansion, R::zero(), lz, k)
}

fn decoded<R: Q>(z: [f64; 4]) -> (Masses<R>, Config<R>) {
    let masses = mass::decode::<Defaults, R>([R::of(z[2]), R::of(z[3])]);
    let config = config::decode::<Defaults, R>([R::of(z[0]), R::of(z[1])], &masses);
    (masses, config)
}

fn times<R: Q>(config: &Config<R>, s: R) -> Config<R> {
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

fn finite<R: Q>(x: R) -> bool {
    x.f().is_finite()
}

/// The `I = 0` fixtures at precision `R`: masses, configuration, and a label.
fn fixtures<R: Q>() -> Vec<(Masses<R>, Config<R>, String)> {
    let mut out = Vec::new();
    for z in [[0.7, -0.4, 0.3, -0.6], [-2.1, 1.3, -1.2, 0.9], [0.0; 4]] {
        let (masses, config) = decoded::<R>(z);
        let zero = times(&config, R::zero());
        let under = times(&config, R::UNDER);
        assert!(
            under.r.iter().flatten().all(|x| x.f() == 0.0),
            "fixture: the scaling did not underflow every position ({})",
            R::LABEL
        );
        out.push((masses, zero, format!("z = {z:?} scaled by 0")));
        out.push((
            masses,
            under,
            format!("z = {z:?} scaled below the smallest subnormal"),
        ));
        let point = Config {
            r: [[R::of(1.25), R::of(-0.75)]; 3],
            ..config
        };
        out.push((
            masses,
            point,
            format!("z = {z:?}'s masses, every body at (1.25, -0.75)"),
        ));
    }
    out
}

/// At `I = 0` about the CoM: `DEGENERATE` with `NO_QUALIFYING_SEED`, no seed, `a = 0`, every emitted field finite, the
/// expansion norm an exact 0, and the momenta the rigid rotation `mᵢ·L_z·J rᵢ`.
fn check_zero<R: Q>(build: Build<R>) {
    for (masses, config, at) in fixtures::<R>() {
        for (lz, k) in [(0.0, 0.5), (0.25, 1.0), (-0.5, 0.125)] {
            let c = build(&masses, &config, R::of(lz), R::of(k));
            let tag = format!("{at}, L_z = {lz}, K* = {k} ({})", R::LABEL);
            let fields =
                c.p.iter()
                    .chain(c.w.iter())
                    .flatten()
                    .chain(c.norms.iter())
                    .chain(std::iter::once(&c.a));
            assert!(
                fields.copied().all(finite),
                "{tag}: a field of the construction is not finite at I = 0: {c:?}"
            );
            assert!(
                c.norms[0].f() == 0.0,
                "{tag}: the expansion norm is not an exact 0 at I = 0: {:e}",
                c.norms[0].f()
            );
            assert_eq!(
                c.outcome,
                Outcome::Degenerate,
                "{tag}: the outcome at I = 0"
            );
            assert_eq!(
                c.outcome.tag(),
                Some(Degenerate::NoQualifyingSeed),
                "{tag}: the reason at I = 0"
            );
            assert!(
                c.seed.is_none(),
                "{tag}: a seed taken at I = 0: {:?}",
                c.seed
            );
            assert!(c.a.f() == 0.0, "{tag}: a = {:e} at I = 0", c.a.f());
            for i in 0..3 {
                let m = masses.m[i].f();
                let r = config.r[i].map(Q::f);
                let want = [-m * lz * r[1], m * lz * r[0]];
                let got = c.p[i].map(Q::f);
                for x in 0..2 {
                    assert!(
                        (got[x] - want[x]).abs() <= 8.0 * R::eps() * want[x].abs(),
                        "{tag}: p[{i}] = {got:?} is not the rigid rotation {want:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_m293_zero_inertia_degenerate() {
    check_zero::<f64>(in_force::<f64>);
    check_zero::<f32>(in_force::<f32>);
}

/// §3.4's formula taken literally, `w = r/√I` at the stored positions' `I`, unguarded: 0/0 at `I = 0`.
#[cfg(feature = "controls")]
fn literal(m: &Masses<f64>, c: &Config<f64>, lz: f64, k: f64) -> Construction<f64> {
    let mm = m.m;
    let i_ = (0..3)
        .map(|b| mm[b] * (c.r[b][0] * c.r[b][0] + c.r[b][1] * c.r[b][1]))
        .sum::<f64>();
    let w = c.r.map(|v| v.map(|x| x / i_.sqrt()));
    let n2 = (0..3)
        .map(|b| mm[b] * (w[b][0] * w[b][0] + w[b][1] * w[b][1]))
        .sum::<f64>();
    let mut out = in_force(m, c, lz, k);
    out.w = w;
    out.norms[0] = n2;
    out
}

negative_control!(
    qa_m293_zero_inertia_degenerate,
    "§3.4's w = r/√I taken literally is 0/0 at I = 0, a NaN direction and norm, and must fail",
    expected = "is not finite at I = 0",
    check_zero::<f64>(literal)
);
