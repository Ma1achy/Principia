//! The acute-angle Burrau charts and the strips (`kernel::chart::burrau_acute`; chart_reference §4.5, §5.1; R-27,
//! R-104, R-157), each decode check at the f64 and the f32 instantiation natively (R-118, R-265):
//! - `burrau_theta_*`: `ν(θ) = sec θ − tan θ` inverts `θ(ν)` on `(0, π/4]`; `Φ_θ,K`'s momenta are §2's construction at
//!   `L_z = 0`, `K(v)`; `Φ_θ,L_z` holds `K` fixed and sweeps `L_z` over `[−λ_L·√(2K), λ_L·√(2K)]`, feasible at its
//!   edges; the register's proposed defaults lie inside the fold; validation (R-26), the flags, names and axes
//!   (REQ-CHART-024).
//! - `burrau_delta_m_strip_*`: the `v = 0` row's masses are the Burrau masses and the `v = 1` row's `m_target`, the
//!   default target equal masses; the triangle is the Burrau one under the blended masses, at rest; validation
//!   (REQ-CHART-027).
//! - `burrau_quotients_*`: the full-range Euclid chart is `DoubleCover` and the acute-angle charts bijective, their
//!   values differ and are shown in the chart labels, and the shape-fraction guard refuses the full-range chart
//!   (REQ-CHART-025).
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.

mod decode_support;

use std::f64::consts::FRAC_PI_4;

use decode_support::*;
use kernel::chart::burrau::{acute_angle, decode_plane, embed, isosceles_nu, nu_image, seed};
use kernel::chart::burrau_acute::{
    decode_delta_m, decode_theta_k, decode_theta_lz, jacobi, nu_of, theta_of,
};
use kernel::chart::invariant::{l_max, WarpReal};
use kernel::chart::{
    admit_shape_fraction, write_dump_header, AxisMeta, BlockTouch, Burrau, BurrauThetaDeltaM,
    BurrauThetaK, BurrauThetaLz, Chart, ChartDecode, ChartLabel, Constraint, EuclidPlane,
    StatisticRefused, SystemImage, ValidationResult,
};
use kernel::decode::invariant::{construct_seeded, infeasible, Outcome};
use kernel::decode::mass::from_masses;
use kernel::decode::{Decoded, Latent};
use kernel::generated::links::Defaults;
use kernel::stage::decode::sample;
use validation::{
    negative_control, prop, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
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
}

impl Q for f64 {
    const LABEL: &'static str = "f64";
}

impl Q for f32 {
    const LABEL: &'static str = "f32";
}

/// The register's proposed `θ` range (REQ-CHART-045), at `R`.
fn theta_range<R: Q>() -> [R; 2] {
    [
        R::of(BURRAU_THETA_MIN.number()),
        R::of(BURRAU_THETA_MAX.number()),
    ]
}

/// `Φ_θ,K` at the register's defaults: the proposed `θ` range and the warp's `K_max`, `γ_K` (REQ-CHART-044).
fn theta_k<R: Q>() -> BurrauThetaK<R> {
    BurrauThetaK {
        theta_range: theta_range(),
        k_max: R::of(K_MAX.number()),
        gamma_k: R::of(GAMMA_K.number()),
    }
}

/// `Φ_θ,L_z` at the register's defaults (REQ-CHART-045, REQ-CHART-046).
fn theta_lz<R: Q>() -> BurrauThetaLz<R> {
    BurrauThetaLz {
        theta_range: theta_range(),
        k: R::of(BURRAU_THETA_LZ_K.number()),
        lz_fraction: R::of(BURRAU_THETA_LZ_FRACTION.number()),
    }
}

/// The δm strip on the proposed `θ` range, toward equal masses.
fn delta_m<R: Q>() -> BurrauThetaDeltaM<R> {
    BurrauThetaDeltaM {
        theta_range: theta_range(),
        m_target: BurrauThetaDeltaM::<R>::equal_masses(),
    }
}

/// The Euclid plane at the register's defaults (chart_reference §4.5).
fn plane() -> EuclidPlane<f64> {
    EuclidPlane {
        m_range: [EUCLID_M_MIN.number(), EUCLID_M_MAX.number()],
        nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
    }
}

/// The `(ν, K)` chart on `nu_range` under the register's warp defaults.
fn nu_k(nu_range: [f64; 2]) -> Burrau<f64> {
    Burrau {
        nu_range,
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

/// `(Σpᵢ, L_z(p), K(p))` of a decode, and the scale `Σ|pᵢ|` its `Σpᵢ` is measured against.
fn invariants<R: Q>(d: &Decoded<R>) -> ([f64; 2], f64, f64, f64) {
    let (m, r, p) = (d.ic.m, d.ic.r, d.ic.p);
    let sum = [
        p[0][0].f() + p[1][0].f() + p[2][0].f(),
        p[0][1].f() + p[1][1].f() + p[2][1].f(),
    ];
    let scale = p.iter().flatten().map(|x| x.f().abs()).sum();
    let lz = (0..3)
        .map(|i| r[i][0].f() * p[i][1].f() - r[i][1].f() * p[i][0].f())
        .sum();
    let kin = (0..3)
        .map(|i| (p[i][0].f().powi(2) + p[i][1].f().powi(2)) / (2.0 * m[i].f()))
        .sum();
    (sum, scale, lz, kin)
}

/// The edge coordinates every check runs at, with the fuzz between them.
const EDGES: [f64; 6] = [0.0, 1.0, 0.5, 0.25, 0.75, 0.999];

// ── burrau_theta (REQ-CHART-024) ─────────────────────────────────────────────────────────────────────────────────

/// The `ν(θ)` under check.
type NuOf = fn(f64) -> f64;

/// `ν(θ)` inverts `θ(ν)` on `(0, π/4]`: `θ(ν(θ)) = θ` within 8 eps absolute, at f64 and f32, from `θ = 10⁻⁶` to the
/// fold's end; `ν(θ(ν)) = ν` on `[√2 − 1, 1)` likewise; `ν(θ)` is `sec θ − tan θ` within 8 eps, `ν(π/4) = √2 − 1` and
/// `ν(arctan(3/4)) = 1/2`, the classical point; and `ν` falls from 1 to `√2 − 1` across the fold.
fn check_inverts(nu_f64: NuOf) {
    let mut thetas: Vec<f64> = (0..=400).map(|i| FRAC_PI_4 * i as f64 / 400.0).collect();
    thetas[0] = 1e-6;
    thetas.extend([BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number(), 1e-3]);
    let mut last = 1.0;
    let mut sorted = thetas.clone();
    sorted.sort_by(f64::total_cmp);
    for &theta in &sorted {
        let nu = nu_f64(theta);
        assert!(
            nu < last && nu >= isosceles_nu() - 1e-15,
            "ν({theta:e}) = {nu} is not in [√2 − 1, 1), falling"
        );
        last = nu;
        let back = acute_angle(nu);
        assert!(
            (back - theta).abs() <= 8.0 * f64::EPSILON,
            "θ(ν(θ)) at θ = {theta:e}, f64: {back:e}"
        );
        let direct = 1.0 / theta.cos() - theta.tan();
        assert!(
            (nu - direct).abs() <= 8.0 * f64::EPSILON,
            "ν(θ) = {nu} is not sec θ − tan θ = {direct} at θ = {theta:e}"
        );
        let t32 = theta as f32;
        let back32 = theta_of(nu_of(t32));
        assert!(
            (back32 - t32).abs() <= 8.0 * f32::EPSILON,
            "θ(ν(θ)) at θ = {theta:e}, f32: {back32:e}"
        );
    }
    for i in 0..=400 {
        let nu = isosceles_nu() + (1.0 - isosceles_nu()) * i as f64 / 401.0;
        let back = nu_f64(theta_of(nu));
        assert!(
            (back - nu).abs() <= 8.0 * f64::EPSILON,
            "ν(θ(ν)) at ν = {nu}: {back}"
        );
    }
    assert!((nu_f64(FRAC_PI_4) - isosceles_nu()).abs() <= 2.0 * f64::EPSILON);
    assert!((nu_f64((0.75f64).atan()) - 0.5).abs() <= 2.0 * f64::EPSILON);
}

#[test]
fn burrau_theta_nu_inverts() {
    check_inverts(nu_of);
}

negative_control!(
    burrau_theta_nu_inverts,
    "sec θ + tan θ in place of sec θ − tan θ must fail the check",
    expected = "is not in [√2 − 1, 1)",
    check_inverts(|t| 1.0 / t.cos() + t.tan())
);

/// A `Φ_θ,K` decode under check.
type ThetaKDecode<R> = fn(&BurrauThetaK<R>, R, R) -> (Decoded<R>, Outcome);

/// `Φ_θ,K` at `(u, v)` is the shared blocks with §2's construction at `L_z = 0`, `K* = K(v)`: its masses the embed's at
/// `ν(θ(u))`, its momenta [`construct_seeded`]'s with the seed [`seed`]`(ν)`, bit for bit, the outcome feasible; and
/// they meet `L_z(p) = 0`, `K(p) = K*` and `Σpᵢ = 0` within REQ-DEC-046's tolerances.
fn check_theta_k<R: Q>(dec: ThetaKDecode<R>, u: f64, v: f64) {
    let chart = theta_k::<R>();
    let (u, v) = (R::of(u), R::of(v));
    let (d, outcome) = dec(&chart, u, v);
    let at = format!("(u, v) = ({:e}, {:e}) at {}", u.f(), v.f(), R::LABEL);
    let nu = chart.nu(u);
    let k = chart.kinetic(v);
    assert_eq!(
        d.ic.m,
        embed(nu).m,
        "the masses are not the embed's at {at}"
    );
    let c = construct_seeded(&d.masses, &d.config, seed(nu), R::zero(), k);
    assert_eq!(
        d.ic.p, c.p,
        "the momenta are not the construction's at {at}"
    );
    assert_eq!(outcome, Outcome::Feasible, "the outcome at {at}");
    let (sum, scale, lz, kin) = invariants(&d);
    let eps = R::eps();
    assert!(
        lz.abs() <= TOL_L * eps * (2.0 * k.f()).sqrt().max(f64::MIN_POSITIVE),
        "L_z(p) = {lz:e}, not 0, at {at}"
    );
    assert!(
        (kin - k.f()).abs() <= TOL_K * eps * k.f(),
        "K(p) = {kin:e}, K* = {:e} at {at}",
        k.f()
    );
    assert!(
        sum[0].abs().max(sum[1].abs()) <= TOL_P * eps * f64::max(scale, f64::MIN_POSITIVE),
        "Σpᵢ = {sum:?} at {at}"
    );
}

#[test]
fn burrau_theta_k_lz_zero() {
    for &u in &EDGES {
        for &v in &EDGES {
            check_theta_k::<f64>(decode_theta_k, u, v);
            check_theta_k::<f32>(decode_theta_k, u, v);
        }
    }
    prop::run(&(0.0f64..=1.0, 0.0f64..=1.0), |(u, v)| {
        check_theta_k::<f64>(decode_theta_k, u, v);
        check_theta_k::<f32>(decode_theta_k, u, v);
        Ok(())
    });
    // At K = 0 the construction gives the rest start: the bottom row is the Euclid plane's system at the same ν.
    let c = theta_k::<f64>();
    let nu = c.nu(0.4);
    let row = EuclidPlane {
        nu_range: [nu, nu],
        ..plane()
    };
    assert_eq!(
        decode_theta_k(&c, 0.4, 0.0).0.ic,
        decode_plane(&row, 0.0).ic
    );
}

negative_control!(
    burrau_theta_k_lz_zero,
    "momenta built at L_z = 0.05 must fail the check",
    expected = "the momenta are not the construction's",
    check_theta_k::<f64>(
        |chart, u, v| {
            let lz = BurrauThetaLz {
                theta_range: chart.theta_range,
                k: chart.kinetic(v),
                lz_fraction: 1.0,
            };
            let at = 0.5 + 0.05 / (2.0 * lz.lz_max::<f64>());
            decode_theta_lz(&lz, u, at)
        },
        0.4,
        0.7
    )
);

/// A `Φ_θ,L_z` decode under check.
type ThetaLzDecode<R> = fn(&BurrauThetaLz<R>, R, R) -> (Decoded<R>, Outcome);

/// `Φ_θ,L_z` at `(u, v)` holds `K` fixed: `K(p) = K` at every `v`, `L_z(p) = L_z(v) = (2v − 1)·λ_L·L_max(K)`, `Σpᵢ = 0`,
/// within REQ-DEC-046's tolerances; its momenta are [`construct_seeded`]'s at `(L_z(v), K)`, the outcome feasible, the
/// edges `v ∈ {0, 1}` included, where `L_z = ∓λ_L·L_max(K)`, and `L_max(K)² ≤ 2K`.
fn check_theta_lz<R: Q>(dec: ThetaLzDecode<R>, chart: &BurrauThetaLz<R>, u: f64, v: f64) {
    let (u, v) = (R::of(u), R::of(v));
    let (d, outcome) = dec(chart, u, v);
    let at = format!("(u, v) = ({:e}, {:e}) at {}", u.f(), v.f(), R::LABEL);
    let k = chart.kinetic::<R>();
    let target = chart.lz(v);
    let (sum, scale, lz, kin) = invariants(&d);
    let eps = R::eps();
    assert!(
        (kin - k.f()).abs() <= TOL_K * eps * k.f(),
        "K(p) = {kin:e}, not the fixed K = {:e}, at {at}",
        k.f()
    );
    assert!(
        (lz - target.f()).abs() <= TOL_L * eps * (target.f().abs() + (2.0 * k.f()).sqrt()),
        "L_z(p) = {lz:e}, L_z(v) = {:e} at {at}",
        target.f()
    );
    assert!(
        sum[0].abs().max(sum[1].abs()) <= TOL_P * eps * f64::max(scale, f64::MIN_POSITIVE),
        "Σpᵢ = {sum:?} at {at}"
    );
    let nu = chart.nu(u);
    let c = construct_seeded(&d.masses, &d.config, seed(nu), target, k);
    assert_eq!(
        d.ic.p, c.p,
        "the momenta are not the construction's at {at}"
    );
    assert_eq!(outcome, Outcome::Feasible, "the outcome at {at}");
    assert_eq!(
        d.ic.m,
        embed(nu).m,
        "the masses are not the embed's at {at}"
    );
    let half = chart.lz_max::<R>();
    let l = l_max(k);
    assert!(
        !infeasible(R::one(), k, l) && half.f() <= l.f(),
        "L_max(K) = {:e} at {at}",
        l.f()
    );
    if v.f() == 0.0 || v.f() == 1.0 {
        assert_eq!(
            target.f().abs(),
            half.f(),
            "the edge's L_z at {at} is not ±λ_L·L_max(K)"
        );
    }
}

#[test]
fn burrau_theta_lz_holds_k() {
    let half = BurrauThetaLz {
        lz_fraction: 0.5,
        ..theta_lz::<f64>()
    };
    let hot = BurrauThetaLz {
        k: 0.9,
        ..theta_lz::<f32>()
    };
    for &u in &EDGES {
        for &v in &EDGES {
            check_theta_lz::<f64>(decode_theta_lz, &theta_lz(), u, v);
            check_theta_lz::<f32>(decode_theta_lz, &theta_lz(), u, v);
            check_theta_lz::<f64>(decode_theta_lz, &half, u, v);
            check_theta_lz::<f32>(decode_theta_lz, &hot, u, v);
        }
    }
    prop::run(&(0.0f64..=1.0, 0.0f64..=1.0), |(u, v)| {
        check_theta_lz::<f64>(decode_theta_lz, &theta_lz(), u, v);
        check_theta_lz::<f32>(decode_theta_lz, &theta_lz(), u, v);
        Ok(())
    });
    // The band spans ±√(2K) at the default λ_L = 1, to rounding.
    let c = theta_lz::<f64>();
    let span = (2.0 * c.k).sqrt();
    assert!((c.lz(1.0) - span).abs() <= 2.0 * f64::EPSILON * span);
    assert!((c.lz(0.0) + span).abs() <= 2.0 * f64::EPSILON * span);
    assert_eq!(c.lz(0.5), 0.0);
}

negative_control!(
    burrau_theta_lz_holds_k,
    "a K that grows with v must fail the check",
    expected = "not the fixed K",
    check_theta_lz::<f64>(
        |chart, u, v| {
            let warmer = BurrauThetaLz {
                k: chart.k * (1.0 + v),
                ..*chart
            };
            decode_theta_lz(&warmer, u, v)
        },
        &theta_lz(),
        0.3,
        0.8
    )
);

/// The register's proposed defaults: `θ_min` is `θ(31/32)` (the Euclid plane's `ν_max`) rounded to f32, `θ_max` the
/// largest f32 at most `π/4`, both f32-exact and inside `(0, π/4]` at f32 and f64; `K > 0` and `λ_L ∈ (0, 1]`; and on
/// a 65 × 65 grid with its edges, every `Φ_θ,L_z` pixel is feasible at both precisions, `L_z(v)² ≤ 2K` exactly.
fn check_defaults(theta: [f64; 2], k: f64, fraction: f64) {
    let [lo, hi] = theta;
    assert_eq!(
        lo,
        f64::from(acute_angle(EUCLID_NU_MAX.number()) as f32),
        "θ_min"
    );
    assert_eq!(f64::from(lo as f32), lo, "θ_min is not f32-exact");
    assert_eq!(f64::from(hi as f32), hi, "θ_max is not f32-exact");
    assert!(
        hi <= FRAC_PI_4 && hi as f32 <= std::f32::consts::FRAC_PI_4,
        "θ_max past π/4"
    );
    assert!(
        f64::from((hi as f32).next_up()) > FRAC_PI_4,
        "θ_max is not the largest f32 inside the fold"
    );
    assert!(lo > 0.0 && lo < hi, "θ_min");
    assert!(k > 0.0 && fraction > 0.0 && fraction <= 1.0, "K or λ_L");
    let c = BurrauThetaLz {
        theta_range: theta,
        k,
        lz_fraction: fraction,
    };
    let narrow = BurrauThetaLz {
        theta_range: theta.map(|x| x as f32),
        k: k as f32,
        lz_fraction: fraction as f32,
    };
    for i in 0..=64 {
        for j in 0..=64 {
            let (u, v) = (i as f64 / 64.0, j as f64 / 64.0);
            assert_eq!(
                decode_theta_lz(&c, u, v).1,
                Outcome::Feasible,
                "f64 at ({u}, {v})"
            );
            assert_eq!(
                decode_theta_lz(&narrow, u as f32, v as f32).1,
                Outcome::Feasible,
                "f32 at ({u}, {v})"
            );
        }
    }
}

#[test]
fn burrau_theta_default_range() {
    check_defaults(
        [BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number()],
        BURRAU_THETA_LZ_K.number(),
        BURRAU_THETA_LZ_FRACTION.number(),
    );
}

negative_control!(
    burrau_theta_default_range,
    "θ_max = f32's nearest π/4, which lies above it, must fail the check",
    expected = "θ_max past π/4",
    check_defaults(
        [
            BURRAU_THETA_MIN.number(),
            f64::from(std::f32::consts::FRAC_PI_4)
        ],
        BURRAU_THETA_LZ_K.number(),
        BURRAU_THETA_LZ_FRACTION.number(),
    )
);

/// Validation (R-26) on the acute-angle charts: inside `[0,1]²` every point is `Valid`; `θ ∉ (0, π/4]` and a non-finite
/// `u` are rejected (`BurrauTheta`), `θ = π/4` and a `θ` one float above 0 valid; `Φ_θ,K`'s `v < 0` is projected to
/// `v = 0` (`KNonNegative`), `v > 1` valid; `Φ_θ,L_z`'s infeasible `v` is projected to the band's edge
/// (`KAboveLzBound`), the feasible band past `[0, 1]` at `λ_L < 1` valid; a non-finite `v` rejected naming the vertical
/// axis' constraint; and a `θ` so small that `ν` rounds to 1, `a = 0`, rejected by layer 3 (`DecodedMassPositive`).
fn check_validate(k: &BurrauThetaK<f64>, lz: &BurrauThetaLz<f64>) {
    let reject = |failed| ValidationResult::Reject { failed };
    for &x in &EDGES {
        for &y in &EDGES {
            assert_eq!(
                k.validate(x, y),
                ValidationResult::Valid,
                "Φ_θ,K at ({x}, {y})"
            );
            assert_eq!(
                lz.validate(x, y),
                ValidationResult::Valid,
                "Φ_θ,L_z at ({x}, {y})"
            );
        }
    }
    let wide_k = BurrauThetaK {
        theta_range: [0.0, 1.0],
        ..*k
    };
    let wide_lz = BurrauThetaLz {
        theta_range: [0.0, 1.0],
        ..*lz
    };
    let above = FRAC_PI_4.next_up();
    for theta in [0.0, -0.25, above, 1.0, 2.0] {
        assert_eq!(
            wide_k.validate(theta, 0.5),
            reject(Constraint::BurrauTheta),
            "Φ_θ,K at θ = {theta}"
        );
        assert_eq!(
            wide_lz.validate(theta, 0.5),
            reject(Constraint::BurrauTheta),
            "Φ_θ,L_z at θ = {theta}"
        );
    }
    for theta in [FRAC_PI_4, 1e-8] {
        assert_eq!(
            wide_k.validate(theta, 0.5),
            ValidationResult::Valid,
            "Φ_θ,K at θ = {theta:e}"
        );
        assert_eq!(
            wide_lz.validate(theta, 0.5),
            ValidationResult::Valid,
            "Φ_θ,L_z at θ = {theta:e}"
        );
    }
    // θ = 10⁻¹⁷: ν = 1/(1 + θ) rounds to 1, so a = 0 and the lightest mass is 0.
    assert_eq!(
        wide_k.validate(1e-17, 0.5),
        reject(Constraint::DecodedMassPositive)
    );
    assert_eq!(
        wide_lz.validate(1e-17, 0.5),
        reject(Constraint::DecodedMassPositive)
    );
    assert_eq!(
        k.validate(0.5, 3.0),
        ValidationResult::Valid,
        "Φ_θ,K above K_max"
    );
    assert_eq!(
        k.validate(0.3, -0.5),
        ValidationResult::Project {
            u: 0.3,
            v: 0.0,
            failed: Constraint::KNonNegative
        }
    );
    assert_eq!(
        k.validate(0.3, -f64::MIN_POSITIVE),
        ValidationResult::Project {
            u: 0.3,
            v: 0.0,
            failed: Constraint::KNonNegative
        }
    );
    assert_eq!(
        k.validate(0.5, 1e200),
        reject(Constraint::MomentumZero),
        "Φ_θ,K at v = 1e200"
    );
    for (v, edge) in [
        (1.25, 1.0),
        (1.0f64.next_up(), 1.0),
        (-0.5, 0.0),
        (-1e-15, 0.0),
    ] {
        assert_eq!(
            lz.validate(0.6, v),
            ValidationResult::Project {
                u: 0.6,
                v: edge,
                failed: Constraint::KAboveLzBound
            },
            "Φ_θ,L_z at v = {v:e}"
        );
    }
    let band = BurrauThetaLz {
        lz_fraction: 0.5,
        ..*lz
    };
    for v in [1.25, 1.5, -0.5, -0.25] {
        assert_eq!(
            band.validate(0.6, v),
            ValidationResult::Valid,
            "λ_L = 1/2 at v = {v}"
        );
    }
    for (v, edge) in [(1.75, 1.5), (-0.75, -0.5)] {
        assert_eq!(
            band.validate(0.6, v),
            ValidationResult::Project {
                u: 0.6,
                v: edge,
                failed: Constraint::KAboveLzBound
            },
            "λ_L = 1/2 at v = {v}"
        );
    }
    // λ_L = 0.3: the band's edges ½ ± ½/0.3 are not dyadic, and the projection lands on a feasible v at most a few
    // floats inside them.
    let odd = BurrauThetaLz {
        lz_fraction: 0.3,
        ..*lz
    };
    for v in [3.0, -3.0] {
        match odd.validate(0.6, v) {
            ValidationResult::Project {
                u: 0.6,
                v: t,
                failed: Constraint::KAboveLzBound,
            } => {
                let edge = 0.5 + 0.5 / 0.3 * v.signum();
                assert!(
                    (t - edge).abs() <= 8.0 * f64::EPSILON * edge.abs(),
                    "λ_L = 0.3: {t} for {edge}"
                );
                assert!(
                    !infeasible(1.0, odd.k, odd.lz(t)),
                    "λ_L = 0.3: {t} is infeasible"
                );
                assert!(infeasible(
                    1.0,
                    odd.k,
                    odd.lz(t + 64.0 * f64::EPSILON * v.signum())
                ));
            }
            other => panic!("λ_L = 0.3 at v = {v}: {other:?}"),
        }
    }
    for (u, v, failed) in [
        (f64::NAN, 0.5, Constraint::BurrauTheta),
        (f64::INFINITY, 0.5, Constraint::BurrauTheta),
        (0.5, f64::NAN, Constraint::KAboveLzBound),
    ] {
        assert_eq!(lz.validate(u, v), reject(failed), "Φ_θ,L_z at ({u}, {v})");
    }
    for (u, v, failed) in [
        (f64::NAN, 0.5, Constraint::BurrauTheta),
        (0.5, f64::INFINITY, Constraint::KNonNegative),
    ] {
        assert_eq!(k.validate(u, v), reject(failed), "Φ_θ,K at ({u}, {v})");
    }
}

#[test]
fn burrau_theta_validate() {
    check_validate(&theta_k(), &theta_lz());
}

negative_control!(
    burrau_theta_validate,
    "a θ range past the fold's end must fail the check",
    expected = "Φ_θ,K at (1, 0)",
    check_validate(
        &BurrauThetaK {
            theta_range: [0.1, 1.0],
            ..theta_k()
        },
        &theta_lz()
    )
);

/// The flags, names and axes: `Φ_θ,K` and `Φ_θ,L_z` forbid `η_E` (their energy is the construction's), the δm strip
/// does not (a rest start); each name goes into the dump header; `θ` writes the configuration and the masses, `K` and
/// `L_z` the momenta, δm the masses; no axis is annotation-only; each states the normalised convention; and the stage's
/// decode ([`sample`], through [`ChartDecode`]) is each chart's own.
fn check_flags(forbids: [bool; 3], names: &str) {
    assert_eq!(forbids, [true, true, false], "the η_E flags");
    let mut h = String::new();
    write_dump_header(&mut h, &theta_k::<f64>()).unwrap();
    write_dump_header(&mut h, &theta_lz::<f32>()).unwrap();
    write_dump_header(&mut h, &delta_m::<f64>()).unwrap();
    assert_eq!(h, names, "the dump headers");
}

#[test]
fn burrau_theta_flags() {
    check_flags(
        [
            theta_k::<f64>().forbids_energy_normalisation(),
            theta_lz::<f64>().forbids_energy_normalisation(),
            delta_m::<f64>().forbids_energy_normalisation(),
        ],
        "chart: BurrauThetaK\nchart: BurrauThetaLz\nchart: BurrauThetaDeltaM\n",
    );
    let block = |config, momentum, mass| AxisMeta {
        touches: BlockTouch {
            config,
            momentum,
            mass,
        },
        annotation: false,
    };
    let theta = block(true, false, true);
    assert_eq!(theta_k::<f64>().axes(), [theta, block(false, true, false)]);
    assert_eq!(theta_lz::<f64>().axes(), [theta, block(false, true, false)]);
    assert_eq!(delta_m::<f64>().axes(), [theta, block(false, false, true)]);
    let conventions = [
        theta_k::<f64>().convention_statement(),
        theta_lz::<f64>().convention_statement(),
        delta_m::<f64>().convention_statement(),
    ];
    assert!(conventions
        .iter()
        .all(|s| *s == kernel::burrau::CONVENTION_STATEMENT));
    assert_eq!(
        theta_k::<f32>().convention(),
        kernel::burrau::Convention::Normalised
    );
    assert_eq!(
        theta_lz::<f32>().convention(),
        kernel::burrau::Convention::Normalised
    );
    assert_eq!(
        delta_m::<f32>().convention(),
        kernel::burrau::Convention::Normalised
    );
    let (k, lz, dm) = (theta_k::<f32>(), theta_lz::<f32>(), delta_m::<f32>());
    assert_eq!(
        sample::<_, Defaults, f32>(&k, 0.3, 0.6).ic,
        decode_theta_k(&k, 0.3, 0.6).0.ic
    );
    assert_eq!(
        sample::<_, Defaults, f32>(&lz, 0.3, 0.6).ic,
        decode_theta_lz(&lz, 0.3, 0.6).0.ic
    );
    assert_eq!(
        sample::<_, Defaults, f32>(&dm, 0.3, 0.6).ic,
        decode_delta_m(&dm, 0.3, 0.6).ic
    );
    assert_eq!(
        dm.decode_at::<Defaults, f32>(0.3, 0.6),
        decode_delta_m(&dm, 0.3, 0.6)
    );
    assert_eq!(k.map::<f32>(0.3, 0.6).z, Latent::new([0.0; 8]));
    assert_eq!(lz.map::<f32>(0.3, 0.6).z, Latent::new([0.0; 8]));
    assert_eq!(dm.map::<f32>(0.3, 0.6).z, Latent::new([0.0; 8]));
}

negative_control!(
    burrau_theta_flags,
    "a Φ_θ,L_z that allows η_E must fail the check",
    expected = "the η_E flags",
    check_flags([true, false, false], "")
);

// ── burrau_delta_m_strip (REQ-CHART-027) ─────────────────────────────────────────────────────────────────────────

/// The masses under check, `(chart, u, v) → m(u, v)`.
type Blend<R> = fn(&BurrauThetaDeltaM<R>, R, R) -> [R; 3];

/// The `v = 0` row's masses equal the Burrau masses at `ν(θ(u))` and the `v = 1` row's `m_target`, exactly, as the
/// chart gives them and as the decode carries them (`d.ic.m`), at f64 and f32 and for the default target, equal masses
/// `(1, 1, 1)/3`, and another; and in between each mass is the blend `(1 − v)·m_Burrau + v·m_target` within 2 eps.
fn check_rows<R: Q>(masses: Blend<R>) {
    let equal = R::one() / R::of(3.0);
    assert_eq!(
        BurrauThetaDeltaM::<R>::equal_masses(),
        [equal; 3],
        "the default target"
    );
    let other = BurrauThetaDeltaM {
        m_target: [R::of(0.5), R::of(0.25), R::of(0.25)],
        ..delta_m::<R>()
    };
    for chart in [delta_m::<R>(), other] {
        for i in 0..=32 {
            let u = R::of(i as f64 / 32.0);
            let burrau = embed(chart.nu(u)).m;
            let at = format!("u = {:e} at {}", u.f(), R::LABEL);
            assert_eq!(
                masses(&chart, u, R::zero()),
                burrau,
                "the v = 0 row at {at}"
            );
            assert_eq!(
                masses(&chart, u, R::one()),
                chart.m_target,
                "the v = 1 row at {at}"
            );
            assert_eq!(
                decode_delta_m(&chart, u, R::zero()).ic.m,
                burrau,
                "the decoded v = 0 row at {at}"
            );
            assert_eq!(
                decode_delta_m(&chart, u, R::one()).ic.m,
                chart.m_target,
                "the decoded v = 1 row at {at}"
            );
            for j in 1..8 {
                let v = j as f64 / 8.0;
                let m = masses(&chart, u, R::of(v));
                for k in 0..3 {
                    let want = (1.0 - v) * burrau[k].f() + v * chart.m_target[k].f();
                    assert!(
                        (m[k].f() - want).abs() <= 2.0 * R::eps(),
                        "m{k} at v = {v}, {at}: {:e}, not the blend {want:e}",
                        m[k].f()
                    );
                }
            }
        }
    }
}

#[test]
fn burrau_delta_m_strip_rows() {
    check_rows::<f64>(BurrauThetaDeltaM::masses);
    check_rows::<f32>(BurrauThetaDeltaM::masses);
}

negative_control!(
    burrau_delta_m_strip_rows,
    "a blend that runs the other way, v·m_Burrau + (1 − v)·m_target, must fail the check",
    expected = "the v = 0 row",
    check_rows::<f64>(|c, u, v| {
        let b = embed(c.nu(u)).m;
        let t = c.m_target;
        [0, 1, 2].map(|k| v * b[k] + (1.0 - v) * t[k])
    })
);

/// A δm decode under check.
type DeltaMDecode<R> = fn(&BurrauThetaDeltaM<R>, R, R) -> Decoded<R>;

/// The δm strip at `(u, v)` is the Burrau triangle at `ν(θ(u))` under the blended masses, at rest: the decoded
/// positions have the right angle at body 0 and the legs in the ratio `a : b` of the embed (within 64 eps), the CoM at
/// the origin and `I = 1` (within 16 eps); the Jacobi vectors are [`jacobi`]'s, `λ_x = −(m₁/M₀₁)·a/c`, so at `v = 0`
/// within 4 eps of §4.2's; every momentum is 0; `Σmᵢ = 1` within 4 eps; and the `v = 0` row decodes as the Euclid
/// plane's rest start at the same `ν`, within 64 eps.
fn check_geometry<R: Q>(dec: DeltaMDecode<R>, u: f64, v: f64) {
    let chart = delta_m::<R>();
    let (u, v) = (R::of(u), R::of(v));
    let d = dec(&chart, u, v);
    let at = format!("(u, v) = ({:e}, {:e}) at {}", u.f(), v.f(), R::LABEL);
    let eps = R::eps();
    let r = d.ic.r.map(|p| p.map(|x| x.f()));
    let m = d.ic.m.map(|x| x.f());
    let leg = |i: usize| [r[i][0] - r[0][0], r[i][1] - r[0][1]];
    let (l1, l2) = (leg(1), leg(2));
    let n1 = l1[0].hypot(l1[1]);
    let n2 = l2[0].hypot(l2[1]);
    let dot = (l1[0] * l2[0] + l1[1] * l2[1]) / (n1 * n2);
    assert!(
        dot.abs() <= 64.0 * eps,
        "the angle at body 0 is not right at {at}: cos = {dot:e}"
    );
    let system = embed(chart.nu(u));
    let ratio = system.rho[0].f() / system.lambda[1].f();
    assert!(
        (n1 / n2 - ratio).abs() <= 64.0 * eps * ratio,
        "the legs' ratio {:e} is not a/b = {ratio:e} at {at}",
        n1 / n2
    );
    let com = [0, 1].map(|c| (0..3).map(|i| m[i] * r[i][c]).sum::<f64>());
    let inertia: f64 = (0..3)
        .map(|i| m[i] * (r[i][0].powi(2) + r[i][1].powi(2)))
        .sum();
    assert!(
        com[0].abs().max(com[1].abs()) <= 16.0 * eps,
        "the CoM {com:?} at {at}"
    );
    assert!(
        (inertia - 1.0).abs() <= 16.0 * eps,
        "I = {inertia:e} at {at}"
    );
    assert!(
        d.ic.p.iter().flatten().all(|x| x.f() == 0.0),
        "a momentum is not 0 at {at}"
    );
    assert!(
        (m.iter().sum::<f64>() - 1.0).abs() <= 4.0 * eps,
        "Σmᵢ at {at}"
    );
    let masses = from_masses(chart.masses(u, v));
    let (rho, lambda) = jacobi(&system, &masses);
    let w = masses.m[1].f() / (masses.m[0].f() + masses.m[1].f());
    assert_eq!(rho, system.rho, "ρ at {at}");
    assert!(
        (lambda[0].f() + w * system.rho[0].f()).abs() <= 4.0 * eps,
        "λ_x at {at}"
    );
    assert_eq!(lambda[1], system.lambda[1], "λ_y at {at}");
    if v.f() == 0.0 {
        assert!(
            (lambda[0].f() - system.lambda[0].f()).abs() <= 4.0 * eps,
            "λ at v = 0 is not §4.2's at {at}"
        );
        let nu = chart.nu(u);
        let row = EuclidPlane {
            m_range: [R::one(), R::of(2.0)],
            nu_range: [nu, nu],
        };
        let rest = decode_plane(&row, R::zero());
        let gap = rest
            .ic
            .r
            .iter()
            .flatten()
            .zip(d.ic.r.iter().flatten())
            .map(|(a, b)| (a.f() - b.f()).abs())
            .fold(0.0, f64::max);
        assert!(
            gap <= 64.0 * eps,
            "the v = 0 row is {gap:e} from the Euclid plane's at {at}"
        );
    }
}

#[test]
fn burrau_delta_m_strip_geometry() {
    for &u in &EDGES {
        for &v in &EDGES {
            check_geometry::<f64>(decode_delta_m, u, v);
            check_geometry::<f32>(decode_delta_m, u, v);
        }
    }
    prop::run(&(0.0f64..=1.0, 0.0f64..=1.0), |(u, v)| {
        check_geometry::<f64>(decode_delta_m, u, v);
        check_geometry::<f32>(decode_delta_m, u, v);
        Ok(())
    });
}

negative_control!(
    burrau_delta_m_strip_geometry,
    "the Burrau masses' λ under the blended masses must fail the check",
    expected = "is not right",
    check_geometry::<f64>(
        |c, u, v| {
            let mut d = decode_delta_m(c, u, v);
            let s = embed(c.nu(u));
            let masses = from_masses(c.masses(u, v));
            d.ic.r = kernel::decode::config::from_jacobi(&masses, s.rho, s.lambda).r;
            d
        },
        0.2,
        1.0
    )
);

/// Validation (R-26) on the δm strip: inside `[0,1]²` every point is `Valid`; past it the blend continues, valid while
/// every mass is positive (`v = 2` toward equal masses) and rejected where one is zero or negative (`MassPositive`;
/// `v = 4`, `v = −3`, and either side of the `v` where `m₀` reaches 0); `θ ∉ (0, π/4]` and a non-finite `u` rejected
/// (`BurrauTheta`), a non-finite `v` naming `MassPositive`; a `θ` so small that `ν` rounds to 1, at `v = 0`, rejected
/// for its zero Burrau mass (`MassPositive`).
fn check_delta_m_validate(c: &BurrauThetaDeltaM<f64>) {
    let reject = |failed| ValidationResult::Reject { failed };
    for &x in &EDGES {
        for &y in &EDGES {
            assert_eq!(
                c.validate(x, y),
                ValidationResult::Valid,
                "δm at ({x}, {y})"
            );
        }
    }
    assert_eq!(c.validate(0.0, 2.0), ValidationResult::Valid, "δm at v = 2");
    for v in [4.0, -3.0, f64::NAN] {
        assert_eq!(
            c.validate(0.0, v),
            reject(Constraint::MassPositive),
            "δm at v = {v}"
        );
    }
    // The mass that reaches 0 first past v = 1 is m₀ (the heaviest, c/(a+b+c)), at v = 1 + m_t/(m_B − m_t).
    let b = embed(c.nu(0.0f64)).m;
    let zero_at = 1.0 + (1.0 / 3.0) / (b[0] - 1.0 / 3.0);
    assert_eq!(
        c.validate(0.0, zero_at * (1.0 + 1e-9)),
        reject(Constraint::MassPositive)
    );
    assert_eq!(
        c.validate(0.0, zero_at * (1.0 - 1e-9)),
        ValidationResult::Valid
    );
    let wide = BurrauThetaDeltaM {
        theta_range: [0.0, 1.0],
        ..*c
    };
    for theta in [0.0, -0.5, FRAC_PI_4.next_up(), 1.0] {
        assert_eq!(
            wide.validate(theta, 0.5),
            reject(Constraint::BurrauTheta),
            "δm at θ = {theta}"
        );
    }
    assert_eq!(wide.validate(FRAC_PI_4, 0.5), ValidationResult::Valid);
    assert_eq!(
        wide.validate(f64::NAN, 0.5),
        reject(Constraint::BurrauTheta)
    );
    // θ = 10⁻¹⁷: ν rounds to 1, so the Burrau masses' lightest is 0, and the v = 0 row's blend has a zero mass.
    assert_eq!(wide.validate(1e-17, 0.0), reject(Constraint::MassPositive));
}

#[test]
fn burrau_delta_m_strip_validate() {
    check_delta_m_validate(&delta_m());
}

negative_control!(
    burrau_delta_m_strip_validate,
    "a target with a zero mass must fail the check",
    expected = "δm at (0, 1)",
    check_delta_m_validate(&BurrauThetaDeltaM {
        m_target: [0.5, 0.5, 0.0],
        ..delta_m()
    })
);

// ── burrau_quotients (REQ-CHART-025) ─────────────────────────────────────────────────────────────────────────────

/// The `system_image` values and labels: the Euclid plane at its default range and the `(ν, K)` chart on it are
/// `DoubleCover`, the acute-angle charts bijective, so the two shape charts' values differ; a `ν` range reaching both
/// sides of `√2 − 1` is `DoubleCover`, one ending at it or on one side bijective; each label shows the chart's name,
/// its value and its quotient, "shape × labelling" or "shape only".
fn check_images(full: SystemImage, folded: SystemImage, label: &str) {
    assert_eq!(
        full,
        SystemImage::DoubleCover,
        "the full-range chart's system_image"
    );
    assert_eq!(
        folded,
        SystemImage::Bijective,
        "the folded chart's system_image"
    );
    assert_ne!(full, folded);
    assert_eq!(
        label,
        "EuclidPlane · system_image DoubleCover · shape × labelling: both leg orderings, a shape covered twice as two \
         labelled systems",
        "the Euclid plane's label"
    );
}

#[test]
fn burrau_quotients_system_image() {
    let p = plane();
    check_images(
        p.system_image(),
        theta_k::<f64>().system_image(),
        &p.label().to_string(),
    );
    let shape_only = "shape only: one leg ordering, each shape covered once";
    for (label, name) in [
        (theta_k::<f64>().label(), "BurrauThetaK"),
        (theta_lz::<f64>().label(), "BurrauThetaLz"),
        (delta_m::<f64>().label(), "BurrauThetaDeltaM"),
    ] {
        assert_eq!(
            label,
            ChartLabel {
                name,
                image: SystemImage::Bijective,
                quotient: shape_only
            }
        );
        assert_eq!(
            label.to_string(),
            format!("{name} · system_image bijective · {shape_only}")
        );
    }
    assert_eq!(theta_lz::<f32>().system_image(), SystemImage::Bijective);
    assert_eq!(delta_m::<f32>().system_image(), SystemImage::Bijective);
    let full = nu_k([EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()]);
    assert_eq!(full.system_image(), SystemImage::DoubleCover);
    assert!(full
        .label()
        .to_string()
        .starts_with("Burrau · system_image DoubleCover · shape × labelling"));
    let s = isosceles_nu();
    assert_eq!(s, 2.0f64.sqrt() - 1.0);
    assert!((acute_angle(s) - FRAC_PI_4).abs() <= 2.0 * f64::EPSILON);
    for (range, image) in [
        ([s, 0.9], SystemImage::Bijective),
        ([0.1, s], SystemImage::Bijective),
        ([0.5, 0.9], SystemImage::Bijective),
        ([0.1, 0.3], SystemImage::Bijective),
        ([s.next_down(), 0.9], SystemImage::DoubleCover),
        ([0.1, s.next_up()], SystemImage::DoubleCover),
        ([0.3, 0.5], SystemImage::DoubleCover),
    ] {
        assert_eq!(nu_image(range), image, "ν range {range:?}");
        assert_eq!(nu_k(range).system_image(), image, "(ν, K) on {range:?}");
        let e = EuclidPlane {
            nu_range: range,
            ..plane()
        };
        assert_eq!(e.system_image(), image, "Euclid plane on {range:?}");
    }
    assert_eq!(
        EuclidPlane {
            nu_range: [0.5, 0.9],
            ..plane()
        }
        .label()
        .to_string(),
        format!("EuclidPlane · system_image bijective · {shape_only}")
    );
    assert_eq!(SystemImage::NToOne(2).to_string(), "2-to-1");
    assert_eq!(SystemImage::RayDegenerate.to_string(), "ray-degenerate");
}

negative_control!(
    burrau_quotients_system_image,
    "a Euclid plane labelled bijective must fail the check",
    expected = "the full-range chart's system_image",
    check_images(
        SystemImage::Bijective,
        SystemImage::Bijective,
        "EuclidPlane · system_image bijective"
    )
);

/// The shape-fraction guard under check.
type Guard = fn(SystemImage) -> Result<(), StatisticRefused>;

/// The guard refuses a shape fraction on the full-range chart (the Euclid plane at its default range, `DoubleCover`)
/// and on a ray-degenerate one, and admits it on the acute-angle charts (bijective) and on an n-to-1 chart.
fn check_guard(guard: Guard) {
    assert_eq!(
        guard(plane().system_image()),
        Err(StatisticRefused::ShapeFractionOnDoubleCover),
        "a shape fraction on the full-range chart"
    );
    assert_eq!(
        guard(nu_k([EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()]).system_image()),
        Err(StatisticRefused::ShapeFractionOnDoubleCover),
        "a shape fraction on the full-range (ν, K) chart"
    );
    assert_eq!(
        guard(SystemImage::RayDegenerate),
        Err(StatisticRefused::ShapeFractionOnRayDegenerate)
    );
    for image in [
        theta_k::<f64>().system_image(),
        theta_lz::<f64>().system_image(),
        delta_m::<f64>().system_image(),
        SystemImage::NToOne(2),
    ] {
        assert_eq!(guard(image), Ok(()), "a shape fraction on {image}");
    }
}

#[test]
fn burrau_quotients_fraction_guard() {
    check_guard(admit_shape_fraction);
}

negative_control!(
    burrau_quotients_fraction_guard,
    "a guard that admits every chart must fail the check",
    expected = "a shape fraction on the full-range chart",
    check_guard(|_| Ok(()))
);
