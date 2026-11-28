//! Slope tiers of the primitive registry's rows.
//!
//! - `slope_tier_declared`: every row of the registry, and of a registry a caller adds rows to, declares a slope tier,
//!   so a row a later task adds with none is caught with no wiring of its own; an exact row carries the forward that
//!   gives its slope; a tier-3 row flags no kink site. Control: a tierless row registered fails.
//! - `slope_tier_exact_rows`: measurement M-B on each exact row at `PROBES`, along each of the state's 15 components
//!   (the three masses, six positions, six momenta, in that order): the dual forward's value is the f64 forward's
//!   within `1e-12·max(1, |f|)` (the same formula, summed in the same order), and its slope against a central
//!   difference at `h = ε^{1/3}·max(1, |x|)` in f64, the step actually taken, within
//!   `SAFETY·(h²·F3/6 + ε·F0/h)`: the difference's truncation plus its rounding. Written before the first run:
//!   `F3 = 1e4` bounds `|f'''|` and `F0 = 100` bounds `|f|` on the probes, whose masses are at least 0.15, separations
//!   at least 0.4 and angles unit-sized (a third derivative of `m·m/r` in `r` is `6mm/r⁴`, below 1e2 there; the
//!   encode's angles carry the factor `(2π)³`, about 2.5e2, and a few more of unit size); `SAFETY = 2`. A second
//!   criterion needs no bound: a central difference is second order, so its error against the exact slope falls by 100
//!   (within a factor 2) from `h = 1e-3` to `1e-4`, for every component whose error at `1e-3` is over `1e-6`. Control:
//!   the slope of a component whose norm is at least 100 times the tolerance, sign flipped, fails.
//!   The probes are decodes moved off the encode's canonical-form sites (`generic`), so the difference is not the
//!   check of a site.
//! - `slope_tier_canon_sites`: at the decodes as they come, the encode rows' slopes are the smooth ones (below).

use engine::chart_authoring::registry::{Registry, Row, Slope};
use kernel::decode::{decode, Ic, Latent};
use kernel::dual::Dual;
use kernel::generated::links::Defaults;
use kernel::slope::Tier;
use validation::negative_control;

const F3: f64 = 1e4;
const F0: f64 = 100.0;
const SAFETY: f64 = 2.0;
const TURN: f64 = 0.7;
const GROW: f64 = 1.3;
const SHIFT: [f64; 2] = [0.05, -0.03];
const KICK: [f64; 2] = [0.03, 0.01];
const PROBES: [[f64; 8]; 3] = [
    [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.1, -0.1],
    [-0.4, 0.6, -0.3, 0.2, 0.1, -0.2, -0.1, 0.15],
    [0.1, 0.9, 0.25, 0.3, -0.25, 0.1, 0.05, 0.1],
];

/// `ic` moved off the encode's sites: positions grown by `GROW` (so `I ≠ 1`) and shifted by `SHIFT` (centre of mass off
/// the origin), body 0's momentum kicked by `KICK` (`Σp ≠ 0`), then everything turned by `TURN` about the origin
/// (`ρ` off `+x`). The masses are unchanged.
fn generic(ic: &Ic<f64>) -> Ic<f64> {
    let (s, c) = TURN.sin_cos();
    let rot = |v: [f64; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    let mut r =
        ic.r.map(|v| [GROW * v[0] + SHIFT[0], GROW * v[1] + SHIFT[1]]);
    let mut p = ic.p;
    p[0] = [p[0][0] + KICK[0], p[0][1] + KICK[1]];
    r = r.map(rot);
    Ic {
        m: ic.m,
        r,
        p: p.map(rot),
    }
}

/// The probes' states, held to the module's note: masses at least 0.15, separations at least 0.4 and pairwise 0.05 apart.
fn probes() -> Vec<Ic<f64>> {
    PROBES
        .iter()
        .map(|z| {
            let ic = generic(&decode::<Defaults, f64>(&Latent::new(*z)).ic);
            let s = |i: usize, j: usize| {
                ((ic.r[i][0] - ic.r[j][0]).powi(2) + (ic.r[i][1] - ic.r[j][1]).powi(2)).sqrt()
            };
            let sep = [s(0, 1), s(0, 2), s(1, 2)];
            assert!(
                ic.m.iter().all(|m| *m >= 0.15),
                "probe {z:?}: masses {:?} below 0.15",
                ic.m
            );
            assert!(
                sep.iter().all(|r| *r >= 0.4),
                "probe {z:?}: separations {sep:?} below 0.4"
            );
            assert!(
                (0..3).all(|i| (i + 1..3).all(|j| (sep[i] - sep[j]).abs() >= 0.05)),
                "probe {z:?}: separations {sep:?} tie"
            );
            ic
        })
        .collect()
}

/// The state's 15 components, the order the module's note gives.
fn flat(ic: &Ic<f64>) -> [f64; 15] {
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

fn unflat(c: &[f64; 15]) -> Ic<f64> {
    Ic {
        m: [c[0], c[1], c[2]],
        r: [[c[3], c[4]], [c[5], c[6]], [c[7], c[8]]],
        p: [[c[9], c[10]], [c[11], c[12]], [c[13], c[14]]],
    }
}

/// `ic` with component `k` a dual variable and the rest constants.
fn seeded(ic: &Ic<f64>, k: usize) -> Ic<Dual> {
    let c = flat(ic);
    let d = |i: usize| {
        if i == k {
            Dual::variable(c[i])
        } else {
            Dual::constant(c[i])
        }
    };
    Ic {
        m: [d(0), d(1), d(2)],
        r: [[d(3), d(4)], [d(5), d(6)], [d(7), d(8)]],
        p: [[d(9), d(10)], [d(11), d(12)], [d(13), d(14)]],
    }
}

fn declared(registry: &Registry) {
    let bare: Vec<&str> = registry
        .rows()
        .iter()
        .filter(|r| r.slope.is_none())
        .map(|r| r.name)
        .collect();
    assert!(
        bare.is_empty(),
        "rows with no declared slope tier: {bare:?}"
    );
    for r in registry.rows() {
        let tier = r.slope.map(Slope::tier);
        assert!(
            tier != Some(Tier::None) || r.kinks.is_empty(),
            "{}: a tier-3 row flags no site",
            r.name
        );
    }
}

#[test]
fn slope_tier_declared() {
    declared(&Registry::standard());
    let tiered = Row {
        name: "a row a later task adds, with its tier",
        ..*Registry::standard().row("K").unwrap()
    };
    declared(&Registry::with([tiered]).unwrap());
}

negative_control!(
    slope_tier_declared,
    "a registered row with no declared tier is reported",
    expected = "no declared slope tier",
    {
        let tierless = Row {
            name: "a tierless row",
            slope: None,
            ..*Registry::standard().row("K").unwrap()
        };
        declared(&Registry::with([tierless]).unwrap());
    }
);

/// A row's `F3`: `1e4` unless the table gives it. The eccentricity is `√(1 + 2εh²/M²)` with `εh²` a polynomial of degree 4
/// in the relative velocity and the probes' momenta are near 0.1, so its third derivative is `|f|/0.1³` times the
/// polynomial's degree factors, and `1e6` bounds it; the first statement, `1e4` for every row, failed this row alone (a
/// third derivative near 3e5 by the run) and is kept for the others.
const ROW_F3: [(&str, f64); 1] = [("inner eccentricity e (0, 1)", 1e6)];

fn f3(row: &str) -> f64 {
    ROW_F3
        .iter()
        .find(|(n, _)| *n == row)
        .map_or(F3, |(_, b)| *b)
}

fn tolerance(row: &str, h: f64) -> f64 {
    SAFETY * (h * h * f3(row) / 6.0 + f64::EPSILON * F0 / h)
}

fn check_rows(flip: bool) {
    let registry = Registry::standard();
    let mut checked = 0;
    for row in registry.rows() {
        let (Some(Slope::Exact(dual)), Some(forward)) = (row.slope, row.forward) else {
            continue;
        };
        for ic in probes() {
            let base = flat(&ic);
            let value = forward(&ic);
            let at = |c: &[f64; 15]| forward(&unflat(c));
            for k in 0..15 {
                let got = dual(&seeded(&ic, k));
                assert!(
                    (got.v - value).abs() <= 1e-12 * value.abs().max(1.0),
                    "{} component {k}: dual value {} against forward {value}",
                    row.name,
                    got.v
                );
                let slope = if flip { -got.d } else { got.d };
                let difference = |h: f64| {
                    let (mut up, mut down) = (base, base);
                    up[k] += h;
                    down[k] -= h;
                    (at(&up) - at(&down)) / (up[k] - down[k])
                };
                let h = f64::EPSILON.cbrt() * base[k].abs().max(1.0);
                let (err, tol) = ((slope - difference(h)).abs(), tolerance(row.name, h));
                assert!(
                    err <= tol,
                    "{} component {k}: dual {slope} against difference {} off by {err:e}, over {tol:e}",
                    row.name,
                    difference(h)
                );
                if flip {
                    assert!(
                        got.d.abs() >= 100.0 * tol,
                        "{} component {k}: too small to be a control",
                        row.name
                    );
                } else {
                    let coarse = (got.d - difference(1e-3)).abs();
                    if coarse > 1e-6 {
                        let ratio = coarse / (got.d - difference(1e-4)).abs();
                        assert!(
                            (50.0..=200.0).contains(&ratio),
                            "{} component {k}: the difference's error falls by {ratio}, not 100",
                            row.name
                        );
                    }
                }
                checked += 1;
            }
        }
        if flip {
            return;
        }
    }
    assert!(checked > 0, "no exact row was checked");
}

#[test]
fn slope_tier_exact_rows() {
    check_rows(false);
}

negative_control!(
    slope_tier_exact_rows,
    "a slope of the opposite sign is off from the central difference by twice its size, so it must fail",
    expected = "off by",
    {
        check_rows(true);
    }
);

/// Each encode row's slope at the decodes as they come, every component, against the central difference: the largest
/// excess over the row's tolerance (negative where every slope agrees). `flip` flips the dual's sign.
fn canon_sites(flip: bool) -> f64 {
    let registry = Registry::standard();
    let mut worst = f64::NEG_INFINITY;
    for name in ["z_α", "z_β", "z_q0", "z_q1", "z_q2", "z_q3", "α", "β"] {
        let row = registry.row(name).unwrap();
        let Some(Slope::Exact(dual)) = row.slope else {
            panic!("{name} is exact")
        };
        let f = row.forward.unwrap();
        for z in PROBES {
            let ic = decode::<Defaults, f64>(&Latent::new(z)).ic;
            let base = flat(&ic);
            for k in 0..15 {
                let slope = dual(&seeded(&ic, k)).d * if flip { -1.0 } else { 1.0 };
                let h = f64::EPSILON.cbrt() * base[k].abs().max(1.0);
                let (mut up, mut down) = (base, base);
                up[k] += h;
                down[k] -= h;
                let difference = (f(&unflat(&up)) - f(&unflat(&down))) / (up[k] - down[k]);
                worst = worst.max((slope - difference).abs() - tolerance(name, h));
            }
        }
    }
    worst
}

/// The decode puts `I = 1`, the centre of mass at 0 and `ρ_y = 0`: the encode's canonical-form sites, which are
/// bookkeeping shortcuts and not kinks, so the slope there is the smooth one. Criterion, written before the run: every
/// encode row, along every one of the 15 components, within `tolerance(row, h)` of the central difference at
/// `h = ε^{1/3}·max(1, |x|)`, which steps off the sites on both sides. Control: the dual's sign flipped.
#[test]
fn slope_tier_canon_sites() {
    let worst = canon_sites(false);
    assert!(
        worst <= 0.0,
        "a slope at a canonical-form site is off the difference by {worst:e} over its tolerance"
    );
}

negative_control!(
    slope_tier_canon_sites,
    "slopes of the opposite sign are off the difference, so the test must fail",
    expected = "off the difference",
    {
        let worst = canon_sites(true);
        assert!(worst <= 0.0, "a slope at a canonical-form site is off the difference by {worst:e} over its tolerance");
    }
);
