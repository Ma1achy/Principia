//! The lookup ladder and the named directions at the active link parameters (chart_reference §0.1, "every chart reads
//! the active value"; chart_decoder_contract § "Tunable parameters": the encode's clamps and its layer-1 range check;
//! inverse_encode_contract § "Chart-aware validation"; R-481 G4; REQ-DEC-048): a looked-up momentum past the active
//! `q_max` is clamped and flagged `lookup_clamped`, a raw `z` is decoded at the active values, a mixed chart's lock
//! decodes to its axes' values, the named directions read the active `μ_max`, and the outcome view's hatch
//! (REQ-COL-102) reads the chart's check at the active `q_max`; at the defaults every form is today's, bit for bit.

use engine::chart_class::{fill, fill_params, forbidden, forbidden_params};
use engine::contract::sim_config::Plane;
use engine::lookup::{
    lookup, lookup_params, lookup_partial, lookup_partial_params, Exact, Locked, Partial,
    Tolerances,
};
use engine::nav::directions::{
    energy_at_fixed_lz, energy_at_fixed_lz_params, library, library_params, mass_away_from_burrau,
    mass_away_from_burrau_params,
};
use engine::synthetic::Synthetic;
use kernel::chart::mixed::{PLambdaY, PRhoX};
use kernel::chart::{Chart, Latent, Mixed, ValidationResult};
use kernel::decode::{self, momentum, Ic};
use kernel::encode::by_kind::Landing;
use kernel::generated::links::{Defaults, Link, LinkParams, SigmoidQ};
use render::raster::Grid;
use validation::negative_control;
use validation::roundtrip::phys;

const D: LinkParams<f64> = LinkParams::DEFAULT;

/// The frozen controls, every one interior.
const FROZEN: [f64; 8] = [0.3, 0.7, 0.2, 0.1, -0.2, 0.4, 0.5, -0.3];

fn tol() -> Tolerances {
    Tolerances::register(128.0 * f64::EPSILON)
}

fn latent_chart() -> Latent<f64> {
    Latent {
        z0: FROZEN,
        q1: [0.4, 0.0, 0.6, 0.0, 0.0, 0.0, 0.5, 0.0],
        q2: [0.0, 0.3, 0.0, 0.0, 0.7, 0.0, 0.0, 0.4],
    }
}

/// `FROZEN` with its `q₀` control at the default links' value of `q0`.
fn z_with_q0(q0: f64) -> [f64; 8] {
    let [zq] = SigmoidQ::inverse([q0]);
    let mut z = FROZEN;
    z[2] = zq;
    z
}

/// The state whose Jacobi `q₀` is `q0`, the rest `FROZEN`'s, at the defaults.
fn state(q0: f64) -> Ic<f64> {
    decode::decode::<Defaults, f64>(&decode::Latent::new(z_with_q0(q0))).ic
}

fn locked(r: Result<Locked, engine::lookup::Rejected>, what: &str) -> Locked {
    r.unwrap_or_else(|e| panic!("{what} is rejected: {e:?}"))
}

/// The lock's Jacobi momenta decoded at `p`.
fn lock_q(l: &Locked, p: &LinkParams<f64>) -> [f64; 4] {
    momentum::controls_at::<Defaults, f64>([l.z[2], l.z[3], l.z[4], l.z[5]], p)
}

/// The ladder at `p15` (`q_max` = 1.5): a looked-up state whose `q₀` is 1.8, exact and partial, is clamped and flagged
/// `lookup_clamped`, its lock decoding to at most 1.5, and not at the defaults; a raw `z` is decoded at `p15`, so its
/// lock keeps it bit for bit with nothing clamped; a mixed `(p_ρ,x, p_λ,y)` chart's state lands at its own point and
/// its lock decodes to the axes' values, and at `q_max` = 8 a `p_ρ,x` of 2.4 is valid; and at the defaults the ladder
/// is today's.
fn check_lookup(p15: &LinkParams<f64>) {
    let chart = latent_chart();
    let x = state(1.8);
    let at15 = locked(
        lookup_params::<Defaults, _, _>(&chart, &Exact::Physical(x), phys, &tol(), p15),
        "q₀ = 1.8",
    );
    assert_ne!(
        at15.notices.moved.controls & 1 << 2,
        0,
        "a looked-up q₀ of 1.8 is not flagged lookup_clamped at {p15:?}"
    );
    assert!(at15.notices.lookup_clamped());
    let q = lock_q(&at15, p15);
    assert!(
        q[0] <= p15.q_max,
        "the clamped lock decodes to q₀ = {}",
        q[0]
    );
    let at2 = locked(
        lookup::<Defaults, _, _>(&chart, &Exact::Physical(x), phys, &tol()),
        "q₀ = 1.8 at the defaults",
    );
    assert_eq!(
        at2.notices.moved.controls, 0,
        "q₀ = 1.8 is clamped at q_max = 2"
    );
    let partial = Partial {
        masses: Some(x.m),
        positions: Some(x.r),
        momenta: Some(x.p),
    };
    let part15 = locked(
        lookup_partial_params::<Defaults, _, _>(&chart, &partial, None, phys, &tol(), p15),
        "the partial q₀ = 1.8",
    );
    assert_ne!(
        part15.notices.moved.controls & 1 << 2,
        0,
        "a partial q₀ of 1.8 is not flagged at {p15:?}"
    );

    // A raw z past the active q_max's reach under the defaults: decoded at p15, so nothing to clamp.
    let raw = z_with_q0(1.9);
    let r15 = locked(
        lookup_params::<Defaults, _, _>(&chart, &Exact::Raw(raw), phys, &tol(), p15),
        "the raw z",
    );
    assert_eq!(
        r15.notices.moved.controls, 0,
        "the raw z is clamped at its own q_max"
    );
    assert_eq!(r15.z.map(f64::to_bits), raw.map(f64::to_bits));

    // A mixed chart: the state at its point (0.8, 0.3) at q_max = 1.5 lands there; the lock decodes to the axes' values.
    let mixed: Mixed<PRhoX, PLambdaY, f64> = Mixed::new(FROZEN, [-1.8, 1.8], [-1.2, 1.2]);
    let x = mixed.decode_params::<Defaults, f64>(0.8, 0.3, p15).ic;
    let m15 = locked(
        lookup_params::<Defaults, _, _>(&mixed, &Exact::Physical(x), phys, &tol(), p15),
        "the mixed chart's state",
    );
    match m15.landing {
        Landing::Chart { u, v } => {
            assert!(
                (u - 0.8).abs() <= 1e-9 && (v - 0.3).abs() <= 1e-9,
                "lands at ({u}, {v})"
            );
        }
        Landing::Latent => panic!("the mixed chart's own state lands in latent z"),
    }
    let q = lock_q(&m15, p15);
    assert!(
        (q[0] - mixed.h(0.8)).abs() <= 1e-9 && (q[3] - mixed.v(0.3)).abs() <= 1e-9,
        "the mixed lock decodes to {q:?}, not the axes' values"
    );
    // Layer 2 at q_max = 8: p_ρ,x = 2.4 is valid, though past the default 2.
    let q8 = LinkParams { q_max: 8.0, ..D };
    let wide: Mixed<PRhoX, PLambdaY, f64> = Mixed::new(FROZEN, [-3.0, 3.0], [-1.2, 1.2]);
    let x = wide.decode_params::<Defaults, f64>(0.9, 0.5, &q8).ic;
    let w8 = locked(
        lookup_params::<Defaults, _, _>(&wide, &Exact::Physical(x), phys, &tol(), &q8),
        "p_ρ,x = 2.4 at q_max = 8",
    );
    assert_eq!(
        w8.notices.clamped, None,
        "p_ρ,x = 2.4 is clamped at q_max = 8"
    );
    assert_eq!(w8.notices.moved.controls, 0);

    // The defaults.
    for entry in [
        Exact::Physical(state(1.8)),
        Exact::Raw(raw),
        Exact::Physical(state(0.4)),
    ] {
        assert_eq!(
            format!(
                "{:?}",
                lookup::<Defaults, _, _>(&chart, &entry, phys, &tol())
            ),
            format!(
                "{:?}",
                lookup_params::<Defaults, _, _>(&chart, &entry, phys, &tol(), &D)
            ),
            "the ladder at the defaults"
        );
    }
    assert_eq!(
        format!(
            "{:?}",
            lookup_partial::<Defaults, _, _>(&chart, &partial, None, phys, &tol())
        ),
        format!(
            "{:?}",
            lookup_partial_params::<Defaults, _, _>(&chart, &partial, None, phys, &tol(), &D)
        ),
        "the partial ladder at the defaults"
    );
}

#[test]
fn link_params_active_lookup() {
    check_lookup(&LinkParams { q_max: 1.5, ..D });
}

negative_control!(
    link_params_active_lookup,
    "at q_max = 2 a q₀ of 1.8 is inside the range, so nothing is flagged",
    expected = "a looked-up q₀ of 1.8 is not flagged",
    check_lookup(&D)
);

/// The named directions at `mu`: the energy direction reads the decoded masses at `μ_max` = `mu.mu_max`, so it moves;
/// the mass direction's `z_eq` is the equal masses' inverse, `μₖ = 0`, at every `μ_max`; and at the defaults each is
/// today's, bit for bit.
fn check_nav(mu: &LinkParams<f64>) {
    let z0 = FROZEN;
    let e = energy_at_fixed_lz_params::<Defaults>(&z0, mu).expect("d_E");
    assert_ne!(
        e,
        energy_at_fixed_lz::<Defaults>(&z0).expect("d_E"),
        "the energy direction at {mu:?} is the default's"
    );
    assert_eq!(
        mass_away_from_burrau_params::<Defaults>(&z0, mu),
        mass_away_from_burrau::<Defaults>(&z0),
        "equal masses' controls moved with μ_max"
    );
    let plane = Plane {
        z0,
        q1: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        q2: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    };
    assert_eq!(library_params::<Defaults>(&plane, mu).energy, Ok(e));
    assert_eq!(
        library::<Defaults>(&plane),
        library_params::<Defaults>(&plane, &D)
    );
}

#[test]
fn link_params_active_nav() {
    check_nav(&LinkParams { mu_max: 4.0, ..D });
}

negative_control!(
    link_params_active_nav,
    "at the defaults the direction is the default's, as one that ignored the active value would be",
    expected = "is the default's",
    check_nav(&D)
);

// ── The hatch (REQ-COL-102) ─────────────────────────────────────────────────────────────────────────────────────────

type Wide = Mixed<PRhoX, PLambdaY, f64>;

/// `(p_ρ,x, p_λ,y)` on `[−3, 3] × [−1.2, 1.2]`: `u` = 0.77 reads 1.62, `u` = 0.9 reads 2.4.
fn wide() -> Wide {
    Mixed::new(FROZEN, [-3.0, 3.0], [-1.2, 1.2])
}

/// A classification of the chart point `(u, v)` at the parameters `p`.
type Classify = fn(&Wide, f64, f64, &LinkParams<f64>) -> Option<bool>;

fn classify_params(c: &Wide, u: f64, v: f64, p: &LinkParams<f64>) -> Option<bool> {
    forbidden_params(c, u, v, p)
}

/// The hatch read off the chart's check at the defaults, whatever `p`.
fn classify_default(c: &Wide, u: f64, v: f64, p: &LinkParams<f64>) -> Option<bool> {
    forbidden_params(c, u, v, p).map(|_| c.validate(u, v) != ValidationResult::Valid)
}

/// The hatch at the active `q_max`: `p_ρ,x` = 1.62 is forbidden at `q_max` = 1.5 and not at the default 2; 2.4 is
/// valid at `q_max` = 8 and forbidden at the default; `fill_params` writes the field at `p`; and at the defaults the
/// classification and the fill are today's.
fn check_hatch(classify: Classify) {
    let c = wide();
    let q15 = LinkParams { q_max: 1.5, ..D };
    let q8 = LinkParams { q_max: 8.0, ..D };
    assert!((c.h(0.77_f64) - 1.62).abs() < 1e-12 && (c.h(0.9_f64) - 2.4).abs() < 1e-12);
    assert_eq!(
        classify(&c, 0.77, 0.5, &q15),
        Some(true),
        "p_ρ,x = 1.62 is not hatched at q_max = 1.5"
    );
    assert_eq!(
        classify(&c, 0.77, 0.5, &D),
        Some(false),
        "p_ρ,x = 1.62 is hatched at q_max = 2"
    );
    assert_eq!(
        classify(&c, 0.9, 0.5, &q8),
        Some(false),
        "p_ρ,x = 2.4 is hatched at q_max = 8"
    );
    assert_eq!(
        classify(&c, 0.9, 0.5, &D),
        Some(true),
        "p_ρ,x = 2.4 is not hatched at q_max = 2"
    );
    let points = [[0.5, 0.5], [0.77, 0.5], [0.9, 0.5]];
    let grid = Grid::new([3, 1], 1, 0, 8).expect("a grid");
    let mut set = Synthetic::flat(grid, 3);
    fill_params(&mut set, &c, |i| points[i as usize], &q15).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        (0..3).map(|i| set.forbidden(i)).collect::<Vec<_>>(),
        [false, true, true]
    );
    let mut today = Synthetic::flat(grid, 3);
    fill(&mut today, &c, |i| points[i as usize]).unwrap_or_else(|e| panic!("{e}"));
    let mut at_d = Synthetic::flat(grid, 3);
    fill_params(&mut at_d, &c, |i| points[i as usize], &D).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        today.bytes().chart_forbidden,
        at_d.bytes().chart_forbidden,
        "the fill at the defaults"
    );
    for u in [0.0, 0.3, 0.6, 0.77, 0.9, 1.0, 1.5] {
        assert_eq!(
            forbidden(&c, u, 0.5),
            forbidden_params(&c, u, 0.5, &D),
            "the hatch at the defaults"
        );
    }
}

#[test]
fn link_params_active_hatch() {
    check_hatch(classify_params);
    // What the threading prevents: the default check leaves 1.62 unhatched at q_max = 1.5.
    let q15 = LinkParams { q_max: 1.5, ..D };
    assert_eq!(classify_default(&wide(), 0.77, 0.5, &q15), Some(false));
}

negative_control!(
    link_params_active_hatch,
    "a hatch that reads validate, at the defaults, leaves 1.62 unhatched at q_max = 1.5",
    expected = "p_ρ,x = 1.62 is not hatched at q_max = 1.5",
    check_hatch(classify_default)
);
