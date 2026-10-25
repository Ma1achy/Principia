//! QA tests for TASK-M2-41, the chart side of REQ-DEC-048, written from the requirement, not from the implementation:
//! "Everything that reads `mu_max` or `q_max` by name must read the active value: the link trees, the codomain's
//! bounds (host-side checks …) …; the register constants must keep their values as the defaults" (R-481 G4;
//! chart_decoder_contract § "Three hard requirements on any registered link", the output in its codomain;
//! inverse_encode_contract § "Chart-aware validation", layer 2).
//!
//! - `qa_m241_charts_bounded`: every chart that decodes through the links (latent, shape sphere, both invariant charts,
//!   a mixed chart with momentum axes) keeps each Jacobi momentum control in `[−q_max, q_max]` at the active
//!   `q_max` = 1.5, and each mass ratio `|ln(mₖ/m₀)|` within the active `μ_max` = 4, over a 21 × 21 grid of `[0,1]²`
//!   with saturating directions; the latent chart reaches past 1.4 and 3.9, so the bound is the active one and not met
//!   by staying small. Control: the same charts decoded at the defaults (what a chart that ignored the active value
//!   would give) pass 1.5 and 4.
//! - `qa_m241_hatch_iff`: on a `(p_ρ,x, p_λ,y)` chart over `[−3, 3]`, the host-side hatch at the active `q_max`
//!   (1.5, the default 2, and 8) marks a point exactly when its `p_ρ,x` label is past the active `q_max`, at every
//!   grid point at least 10⁻⁶ from the edge. Control: the hatch read at the defaults misclassifies at `q_max` = 1.5.
//!
//! **Domain** (pitfalls §10): stateless decodes and checks at fixed chart points; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use engine::chart_class::{forbidden, forbidden_params};
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{PLambdaY, PRhoX};
use kernel::chart::{Chart, InvariantLE, InvariantLK, Latent, Mixed, ShapeMap, ShapeSphere};
use kernel::decode::Decoded;
use kernel::generated::links::{Defaults, LinkParams};
use validation::negative_control;

const D: LinkParams<f64> = LinkParams::DEFAULT;

/// Relative slack on a bound, for the rounding of `q_max·g(z)` and `μ_max·tanh(z)`: a few ulps.
const SLACK: f64 = 8.0 * f64::EPSILON;

/// A decode of `chart` at `(u, v)` and the parameters `p`.
type DecodeFn<C> = fn(&C, f64, f64, &LinkParams<f64>) -> Decoded<f64>;

fn at_params<C: Chart<Param = f64>>(c: &C, u: f64, v: f64, p: &LinkParams<f64>) -> Decoded<f64> {
    c.decode_params::<Defaults, f64>(u, v, p)
}

/// The decode at the defaults, whatever `p`: a chart that ignored the active value.
fn at_default<C: Chart<Param = f64>>(c: &C, u: f64, v: f64, _p: &LinkParams<f64>) -> Decoded<f64> {
    c.decode::<Defaults, f64>(u, v)
}

/// The grid's 21 points per side of `[0,1]²`, the corners included.
fn grid() -> impl Iterator<Item = (f64, f64)> {
    (0..=20).flat_map(|i| (0..=20).map(move |j| (i as f64 / 20.0, j as f64 / 20.0)))
}

/// The largest `|qₖ|` and the largest `|ln(mₖ/m₀)|` of `decode` over the grid.
fn peaks<C: Chart<Param = f64>>(c: &C, decode: DecodeFn<C>, p: &LinkParams<f64>) -> (f64, f64) {
    grid().fold((0.0f64, 0.0f64), |(q, mu), (u, v)| {
        let d = decode(c, u, v, p);
        let m = d.masses.m;
        let dq = d.momentum.q.iter().fold(0.0f64, |s, x| s.max(x.abs()));
        let dm = (m[1] / m[0]).ln().abs().max((m[2] / m[0]).ln().abs());
        (q.max(dq), mu.max(dm))
    })
}

/// A latent chart whose directions saturate every momentum and mass control at the grid's edges.
fn latent() -> Latent<f64> {
    Latent {
        z0: [0.3, 0.7, 0.2, -0.1, 0.4, -0.3, 0.5, -0.2],
        q1: [0.0, 0.0, 9.0, 0.0, -9.0, 0.0, 9.0, 0.0],
        q2: [0.0, 0.0, 0.0, 9.0, 0.0, -9.0, 0.0, -9.0],
    }
}

fn sphere() -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 0.5,
        inertia: 1.7,
        phase: 0.3,
        z_mass: [7.0, -7.0],
        z_mom: [7.0, -7.0, 6.0, -6.0],
    }
}

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.3, 0.7],
        z_mass: [7.0, -7.0],
        k_max: 1.5,
        gamma_k: 2.0,
    }
}

type Pp = Mixed<PRhoX, PLambdaY, f64>;

/// `(p_ρ,x, p_λ,y)` on `[−3, 3] × [−3, 3]`, its frozen mass controls saturating.
fn mixed() -> Pp {
    Mixed::new(
        [0.3, 0.7, 0.2, 0.1, -0.2, 0.4, 7.0, -7.0],
        [-3.0, 3.0],
        [-3.0, 3.0],
    )
}

/// Which decode each chart is read through: the chart's own at `p`, or the defaults' (the control).
struct Decodes {
    latent: DecodeFn<Latent<f64>>,
    sphere: DecodeFn<ShapeSphere<f64>>,
    le: DecodeFn<InvariantLE<f64>>,
    lk: DecodeFn<InvariantLK<f64>>,
    mixed: DecodeFn<Pp>,
}

fn check_bounded(d: &Decodes) {
    let q15 = LinkParams { q_max: 1.5, ..D };
    let mu4 = LinkParams { mu_max: 4.0, ..D };
    let q_bound = 1.5 * (1.0 + SLACK);
    let mu_bound = 4.0 * (1.0 + SLACK);
    let mut reach = (0.0f64, 0.0f64);
    // (name, peak |q| at q_max = 1.5 or None where the momenta are not link-decoded, peak |ln(mₖ/m₀)| at μ_max = 4)
    let rows: [(&str, Option<f64>, f64); 5] = [
        (
            "latent",
            Some(peaks(&latent(), d.latent, &q15).0),
            peaks(&latent(), d.latent, &mu4).1,
        ),
        (
            "shape sphere",
            Some(peaks(&sphere(), d.sphere, &q15).0),
            peaks(&sphere(), d.sphere, &mu4).1,
        ),
        (
            "(L_z, E)",
            None,
            peaks(&InvariantLE(invariant()), d.le, &mu4).1,
        ),
        (
            "(L_z, K)",
            None,
            peaks(&InvariantLK(invariant()), d.lk, &mu4).1,
        ),
        (
            "(p_ρ,x, p_λ,y)",
            Some(peaks(&mixed(), d.mixed, &q15).0),
            peaks(&mixed(), d.mixed, &mu4).1,
        ),
    ];
    for (name, q, mu) in rows {
        if let Some(q) = q {
            assert!(
                q <= q_bound,
                "the {name} chart decodes |q| = {q} past the active q_max = 1.5 (active)"
            );
            if name == "latent" {
                reach.0 = q;
            }
        }
        assert!(
            mu <= mu_bound,
            "the {name} chart decodes a mass ratio e^{mu} past the active μ_max = 4 (active)"
        );
        if name == "latent" {
            reach.1 = mu;
        }
    }
    assert!(
        reach.0 > 1.4 && reach.1 > 3.9,
        "the latent chart reaches only |q| = {} and ln ratio {}: the bounds are not exercised",
        reach.0,
        reach.1
    );
}

#[test]
fn qa_m241_charts_bounded() {
    check_bounded(&Decodes {
        latent: at_params,
        sphere: at_params,
        le: at_params,
        lk: at_params,
        mixed: at_params,
    });
    // The fixture discriminates, chart by chart: each read at the defaults passes the active bound it is checked
    // against, so a chart whose decode ignored the active value would fail its own row.
    let q15 = LinkParams { q_max: 1.5, ..D };
    let mu4 = LinkParams { mu_max: 4.0, ..D };
    let defaults: [(&str, Option<f64>, f64); 5] = [
        (
            "latent",
            Some(peaks(&latent(), at_default, &q15).0),
            peaks(&latent(), at_default, &mu4).1,
        ),
        (
            "shape sphere",
            Some(peaks(&sphere(), at_default, &q15).0),
            peaks(&sphere(), at_default, &mu4).1,
        ),
        (
            "(L_z, E)",
            None,
            peaks(&InvariantLE(invariant()), at_default, &mu4).1,
        ),
        (
            "(L_z, K)",
            None,
            peaks(&InvariantLK(invariant()), at_default, &mu4).1,
        ),
        (
            "(p_ρ,x, p_λ,y)",
            Some(peaks(&mixed(), at_default, &q15).0),
            peaks(&mixed(), at_default, &mu4).1,
        ),
    ];
    for (name, q, mu) in defaults {
        assert!(
            q.is_none_or(|q| q > 1.5) && mu > 4.0,
            "the {name} chart at the defaults reaches only |q| = {q:?}, ln ratio {mu}"
        );
    }
}

negative_control!(
    qa_m241_charts_bounded,
    "the charts decoded at the defaults pass q_max = 1.5 and μ_max = 4",
    expected = "(active)",
    check_bounded(&Decodes {
        latent: at_default,
        sphere: at_default,
        le: at_default,
        lk: at_default,
        mixed: at_default,
    })
);

// ── The host-side hatch ─────────────────────────────────────────────────────────────────────────────────────────────

/// A hatch classification of `(u, v)` at `p`.
type Hatch = fn(&Pp, f64, f64, &LinkParams<f64>) -> Option<bool>;

fn hatch_params(c: &Pp, u: f64, v: f64, p: &LinkParams<f64>) -> Option<bool> {
    forbidden_params(c, u, v, p)
}

fn hatch_default(c: &Pp, u: f64, v: f64, _p: &LinkParams<f64>) -> Option<bool> {
    forbidden(c, u, v)
}

/// `(p_ρ,x, p_λ,y)` on `[−3, 3] × [−1.2, 1.2]`, every frozen control interior.
fn wide() -> Pp {
    Mixed::new(
        [0.3, 0.7, 0.2, 0.1, -0.2, 0.4, 0.5, -0.3],
        [-3.0, 3.0],
        [-1.2, 1.2],
    )
}

fn check_hatch(hatch: Hatch) {
    let c = wide();
    for q in [1.5, 2.0, 8.0] {
        let p = LinkParams { q_max: q, ..D };
        let mut hatched = 0;
        for i in 0..=120 {
            let u = i as f64 / 120.0;
            let x: f64 = c.h(u);
            if (x.abs() - q).abs() < 1e-6 {
                continue;
            }
            let want = x.abs() > q;
            hatched += usize::from(want);
            for v in [0.25, 0.5, 0.75] {
                assert_eq!(
                    hatch(&c, u, v, &p),
                    Some(want),
                    "p_ρ,x = {x} at q_max = {q} is hatched {:?}, not {want} (active)",
                    hatch(&c, u, v, &p)
                );
            }
        }
        // A range that hatches nothing would make the iff trivial on one side.
        assert_eq!(
            hatched > 0,
            q < 3.0,
            "{hatched} points hatched at q_max = {q}"
        );
    }
}

#[test]
fn qa_m241_hatch_iff() {
    check_hatch(hatch_params);
    // The fixture discriminates: read at the defaults, p_ρ,x = 1.62 is unhatched at q_max = 1.5.
    let c = wide();
    let u = (1.62 + 3.0) / 6.0;
    assert_eq!(
        hatch_default(&c, u, 0.5, &LinkParams { q_max: 1.5, ..D }),
        Some(false)
    );
}

negative_control!(
    qa_m241_hatch_iff,
    "a hatch read at the defaults misclassifies p_ρ,x between 1.5 and 2 at q_max = 1.5",
    expected = "(active)",
    check_hatch(hatch_default)
);
