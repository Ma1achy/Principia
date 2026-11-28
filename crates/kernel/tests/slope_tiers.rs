//! Slope tiers: every chart declares one, an exact slope is the decode's `J_D` at `Dual` and agrees with a central
//! difference, and the footprint of a kink site is flagged.
//!
//! - `slope_tiers_declared`: every chart of `REGISTRY` declares exactly one tier; a tier-3 chart is accepted with no
//!   slope and no kink. Control: a table with one chart's tier removed is reported.
//! - `slope_tiers_value_is_the_f64_decode`: the dual decode's value part is the f64 decode's, bit for bit, so a slope
//!   is the slope of the decode the integrator uses. Control: a value one ulp off is caught by the same comparison.
//! - `slope_tiers_measurement_b`: measurement M-B, below.
//! - `slope_tiers_kink_flag`: the footprint of a point on a listed site is flagged, one far from every site is not.
//!   Control: a chart listing no site flags nothing.
//!
//! **Measurement M-B, written before the run.** For each tier-1 chart at each point of `POINTS` (inside the chart's
//! valid square, flagged by no site at half-width `FOOTPRINT`, larger than every step taken), each of the 15 components
//! of `(m, r, p)` and each of the two columns (`u` and `v`): the dual slope against a central difference at
//! `h = ε^{1/3}·max(1, |x|)` in f64 (the step actually taken, `(x+h) − (x−h)`). Criterion, per column and component:
//!
//! `|dual − difference| ≤ SAFETY·(h²·F3/6 + ε·F0/h)`
//!
//! the difference's own truncation plus its rounding, each of order `ε^{2/3}`. `F0 = 10` bounds `|f|`. `F3` bounds
//! `|f'''|` from the closed form: a component is a polynomial of degree at most 3 in the sines and cosines of the
//! chart's angles and in smooth weights, each of unit size, so a third derivative in a coordinate that spans `S`
//! per unit is at most `50·S³`: `S = 2π` for a chart whose coordinate is an angle, and the range's span for the Euclid
//! plane's mass axis. `SAFETY = 2` is the slack for the dual's own rounding. The control flips the sign of a column
//! whose norm is at least 100 times the tolerance, and must fail.
//!
//! A second criterion does not depend on `F3`: a central difference is second order, so at the steps `1e-3` and `1e-4`
//! its error against the exact slope falls by 100 (within a factor 2), for every component whose error at `1e-3` is
//! over `1e-6`. A wrong slope does not converge to the difference.
//!
//! *First run:* `F3` was first stated as 100 for every chart. The Shape sphere's component 7 failed it (dual
//! 0.74271306782, difference 0.74271304850, off by 1.9e-8 against 2e-9); a scratch run at five steps showed the
//! difference's error falling as `h²` (5.3e-2, 5.3e-4, 5.3e-6 at 1e-2, 1e-3, 1e-4) onto the dual, so the dual was right
//! and the bound was wrong: `|f'''|` is near 3e3 there, an angle's `(2π)³` times a few, which the bound above states.

use kernel::chart::burrau::{Burrau, EuclidPlane};
use kernel::chart::burrau_acute::{BurrauThetaK, BurrauThetaLz};
use kernel::chart::construction::{HeldLz, RestStart};
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Alpha, PRhoX};
use kernel::chart::{
    BurrauThetaDeltaM, Chart, InvariantLE, InvariantLK, Latent, MassSimplex, Mixed, ShapeMap,
    ShapeSphere, ValidationResult, REGISTRY,
};
use kernel::decode::{Decoded, Ic};
use kernel::dual::Dual;
use kernel::generated::links::Defaults;
use kernel::slope::{
    declaration, exact, flag, undeclared_in, Declaration, Kink, Tier, DECLARATIONS,
};
use validation::negative_control;

const TAU: f64 = std::f64::consts::TAU;
const F0: f64 = 10.0;
const SAFETY: f64 = 2.0;
const FOOTPRINT: f64 = 1e-3;
const POINTS: [(f64, f64); 3] = [(0.37, 0.61), (0.30, 0.40), (0.12, 0.77)];

fn components(ic: &Ic<f64>) -> [f64; 15] {
    let mut c = [0.0; 15];
    c[..3].copy_from_slice(&ic.m);
    for i in 0..3 {
        c[3 + 2 * i] = ic.r[i][0];
        c[4 + 2 * i] = ic.r[i][1];
        c[9 + 2 * i] = ic.p[i][0];
        c[10 + 2 * i] = ic.p[i][1];
    }
    c
}

fn decoded<C: Chart<Param = f64>>(c: &C, u: f64, v: f64) -> [f64; 15] {
    components(&c.decode::<Defaults, f64>(u, v).ic)
}

/// The central difference of the decode along `u` (`axis` 0) or `v` (1), the step as the criterion states it.
fn difference<C: Chart<Param = f64>>(c: &C, u: f64, v: f64, axis: usize) -> ([f64; 15], f64) {
    let x = if axis == 0 { u } else { v };
    let h = f64::EPSILON.cbrt() * x.abs().max(1.0);
    let at = |x: f64| {
        if axis == 0 {
            decoded(c, x, v)
        } else {
            decoded(c, u, x)
        }
    };
    let (up, down) = (x + h, x - h);
    let (a, b) = (at(up), at(down));
    let span = up - down;
    let mut col = [0.0; 15];
    for k in 0..15 {
        col[k] = (a[k] - b[k]) / span;
    }
    (col, h)
}

fn tolerance(h: f64, span: f64) -> f64 {
    let f3 = 50.0 * span.powi(3);
    SAFETY * (h * h * f3 / 6.0 + f64::EPSILON * F0 / h)
}

/// The difference's error against the exact slope at a given step, per component.
fn error_at<C: Chart<Param = f64>>(
    c: &C,
    (u, v): (f64, f64),
    axis: usize,
    h: f64,
    slope: &[f64; 15],
) -> [f64; 15] {
    let at = |s: f64| {
        if axis == 0 {
            decoded(c, u + s, v)
        } else {
            decoded(c, u, v + s)
        }
    };
    let (a, b) = (at(h), at(-h));
    let mut e = [0.0; 15];
    for k in 0..15 {
        e[k] = ((a[k] - b[k]) / (2.0 * h) - slope[k]).abs();
    }
    e
}

/// Every exact slope of `c` at `POINTS`, against the difference; `flip` flips the `u` column's sign (the control).
fn check_slopes<C: Chart<Param = f64>>(name: &str, c: &C, span: f64, flip: bool) {
    for (u, v) in POINTS {
        assert!(
            matches!(c.validate(u, v), ValidationResult::Valid)
                && c.decode::<Defaults, f64>(u, v).degenerate.is_none(),
            "{name} at ({u}, {v}) is not a valid decode"
        );
        let site = flag::<Defaults, C>(c, &[Kink::Pole], u, v, FOOTPRINT);
        assert_eq!(
            site, None,
            "{name} at ({u}, {v}) is within a site's footprint: {site:?}"
        );
        let (_, mut su, sv) = exact::<Defaults, C>(c, u, v);
        if flip {
            su = su.map(|x| -x);
        }
        for (axis, slope) in [(0, su), (1, sv)] {
            let (diff, h) = difference(c, u, v, axis);
            let tol = tolerance(h, span);
            for k in 0..15 {
                let err = (slope[k] - diff[k]).abs();
                assert!(
                    err <= tol,
                    "{name} at ({u}, {v}) column {axis} component {k}: dual {} against difference {} off by {err:e}, over {tol:e}",
                    slope[k],
                    diff[k]
                );
            }
            if !flip {
                let coarse = error_at(c, (u, v), axis, 1e-3, &slope);
                let fine = error_at(c, (u, v), axis, 1e-4, &slope);
                for k in 0..15 {
                    if coarse[k] > 1e-6 {
                        let ratio = coarse[k] / fine[k];
                        assert!(
                            (50.0..=200.0).contains(&ratio),
                            "{name} at ({u}, {v}) column {axis} component {k}: the difference's error falls by {ratio}, not 100"
                        );
                    }
                }
            }
            if flip {
                let norm = slope.iter().map(|x| x * x).sum::<f64>().sqrt();
                assert!(
                    norm >= 100.0 * tol || axis == 1,
                    "{name}: the flipped column is too small for a control"
                );
            }
        }
    }
}

fn sphere(map: ShapeMap) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0: [1.0, 0.0, 0.0],
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.4],
        z_mom: [0.2, -0.1, 0.15, 0.05],
    }
}

fn mixed() -> Mixed<Alpha, PRhoX, f64> {
    Mixed::new(
        [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4],
        [0.05, 1.5],
        [-1.8, 1.8],
    )
}

fn invariant() -> InvariantLE<f64> {
    InvariantLE(Invariant {
        z_cfg: [0.3, -0.5],
        z_mass: [0.7, -0.4],
        k_max: 1.0,
        gamma_k: 1.5,
    })
}

fn simplex() -> MassSimplex<f64> {
    MassSimplex {
        nu0: 0.5,
        eps_m: 1e-4,
    }
}

fn euclid() -> EuclidPlane<f64> {
    EuclidPlane {
        m_range: [1.0, 32.0],
        nu_range: [1.0 / 32.0, 31.0 / 32.0],
    }
}

fn burrau() -> Burrau<f64> {
    Burrau {
        nu_range: [0.1, 0.9],
        k_max: 1.0,
        gamma_k: 1.5,
    }
}

fn latent() -> Latent<f64> {
    Latent {
        z0: [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4],
        q1: [1.0, 0.2, 0.0, 0.5, 0.0, 0.0, 0.3, 0.0],
        q2: [0.0, 0.5, 0.4, 0.0, 0.0, 0.3, 0.0, 0.2],
    }
}

fn invariant_lk() -> InvariantLK<f64> {
    InvariantLK(invariant().0)
}

fn theta_k() -> HeldLz<BurrauThetaK<f64>, f64> {
    HeldLz::new(BurrauThetaK {
        theta_range: [0.2, std::f64::consts::FRAC_PI_4],
        k_max: 1.0,
        gamma_k: 1.5,
    })
}

fn theta_lz() -> BurrauThetaLz<f64> {
    BurrauThetaLz {
        theta_range: [0.2, std::f64::consts::FRAC_PI_4],
        k: 0.5,
        lz_fraction: 1.0,
    }
}

fn delta_m() -> RestStart<BurrauThetaDeltaM<f64>, f64> {
    RestStart::new(BurrauThetaDeltaM {
        theta_range: [0.2, std::f64::consts::FRAC_PI_4],
        m_target: BurrauThetaDeltaM::equal_masses(),
    })
}

fn all_slopes(flip: bool) {
    check_slopes("Latent", &latent(), TAU, flip);
    check_slopes("InvariantLK", &invariant_lk(), TAU, flip);
    check_slopes("BurrauThetaK", &theta_k(), TAU, flip);
    check_slopes("BurrauThetaLz", &theta_lz(), TAU, flip);
    check_slopes("BurrauThetaDeltaM", &delta_m(), TAU, flip);
    check_slopes(
        "ShapeSphere (spherical)",
        &sphere(ShapeMap::Spherical),
        TAU,
        flip,
    );
    check_slopes(
        "ShapeSphere (exponential)",
        &sphere(ShapeMap::Exponential),
        TAU,
        flip,
    );
    check_slopes("Mixed", &mixed(), TAU, flip);
    check_slopes("InvariantLE", &invariant(), TAU, flip);
    check_slopes("MassSimplex", &simplex(), TAU, flip);
    check_slopes("EuclidPlane", &euclid(), 31.0, flip);
    check_slopes("Burrau", &burrau(), TAU, flip);
}

#[test]
fn slope_tiers_measurement_b() {
    all_slopes(false);
}

negative_control!(
    slope_tiers_measurement_b,
    "a column of the opposite sign differs from the central difference by twice its size, so it must fail",
    expected = "off by",
    {
        all_slopes(true);
    }
);

fn declared(table: &[Declaration]) {
    let missing: Vec<&str> = undeclared_in(table).collect();
    assert!(
        missing.is_empty(),
        "charts with no declared slope tier: {missing:?}"
    );
    for d in table {
        assert!(
            d.tier != Some(Tier::None) || d.kinks.is_empty(),
            "{}: a tier-3 chart has no slope, so no site to flag",
            d.chart
        );
    }
}

#[test]
fn slope_tiers_declared() {
    declared(&DECLARATIONS);
    for name in REGISTRY {
        assert!(declaration(name).is_some(), "{name} has no declaration");
    }
    // A tier-3 chart is accepted with no slope.
    let mut table = DECLARATIONS;
    table[1] = Declaration {
        tier: Some(Tier::None),
        kinks: &[],
        ..table[1]
    };
    declared(&table);
}

negative_control!(
    slope_tiers_declared,
    "a table with one chart's tier removed is reported",
    expected = "no declared slope tier",
    {
        let mut table = DECLARATIONS;
        table[2] = Declaration {
            tier: None,
            ..table[2]
        };
        declared(&table);
    }
);

fn value_is_f64(nudge: bool) {
    for (u, v) in POINTS {
        let c = sphere(ShapeMap::Spherical);
        let dual: Decoded<Dual> = c.decode::<Defaults, Dual>(Dual::variable(u), Dual::constant(v));
        let plain = c.decode::<Defaults, f64>(u, v);
        let mut got = components(&Ic {
            m: dual.ic.m.map(|x| x.v),
            r: dual.ic.r.map(|r| r.map(|x| x.v)),
            p: dual.ic.p.map(|p| p.map(|x| x.v)),
        });
        if nudge {
            got[3] = f64::from_bits(got[3].to_bits() + 1);
        }
        for (k, (a, b)) in got.iter().zip(components(&plain.ic)).enumerate() {
            assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "component {k} differs in its bits"
            );
        }
    }
}

#[test]
fn slope_tiers_value_is_the_f64_decode() {
    value_is_f64(false);
}

negative_control!(
    slope_tiers_value_is_the_f64_decode,
    "a value one ulp off is a different decode, so it must fail",
    expected = "differs in its bits",
    {
        value_is_f64(true);
    }
);

#[test]
fn slope_tiers_kink_flag() {
    let sites = declaration("ShapeSphere").unwrap().kinks;
    // The sphere's pole, `n = −n₀` at `(0.5, 0.5)`, where the slope is 1e17: the footprint meets it. The exponential
    // map's `r = 0` is `n = n₀` at the same point, where the slope is NaN.
    let pole = flag::<Defaults, _>(&sphere(ShapeMap::Spherical), sites, 0.5, 0.5, FOOTPRINT);
    assert!(pole.is_some(), "the footprint of the pole is not flagged");
    let origin = flag::<Defaults, _>(&sphere(ShapeMap::Exponential), sites, 0.5, 0.5, FOOTPRINT);
    assert_eq!(
        origin,
        Some(Kink::ExpMapRZero),
        "the footprint of `r = 0` is not flagged as it"
    );
    // A footprint of the first measurement point meets no site, on either map.
    for map in [ShapeMap::Spherical, ShapeMap::Exponential] {
        let inside = flag::<Defaults, _>(&sphere(map), sites, 0.37, 0.61, FOOTPRINT);
        assert_eq!(inside, None, "an interior point is flagged as {inside:?}");
    }
}

negative_control!(
    slope_tiers_kink_flag,
    "a chart that lists no site flags nothing at its pole, so the test must fail",
    expected = "is not flagged",
    {
        let sph = sphere(ShapeMap::Spherical);
        let at_pole = flag::<Defaults, _>(&sph, &[], 0.5, 0.5, FOOTPRINT);
        assert!(
            at_pole.is_some(),
            "the footprint of the pole is not flagged"
        );
    }
);
