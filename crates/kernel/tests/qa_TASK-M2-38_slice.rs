//! QA's tests for TASK-M2-38's slice of a held coordinate, written from REQ-CHART-060 ("accept a pan, slice, … edit
//! only if the chart's centre … still passes validate's layers 2 and 3, a centre whose Φ fails counting as failing; …
//! a lock on a pixel that is not a system must be refused, the centre unchanged"), not from the implementation.
//!
//! The oracle is the decode at the accepted centre, never the chart's own report: what the stop accepts must decode to
//! a three-body system, every mass positive and finite, every position and momentum finite, and no `DEGENERATE`
//! reason (R-469). Each held coordinate whose value at the slice centre the chart states (the rest starts' four
//! momentum controls on the mass simplex, the Euclid plane and the δm strip, bare and under a tilt; the shape sphere's
//! momentum controls and mass logits; the held `L_z` of `(ν, K)` and `Φ_θ,K`; the held `K` of `Φ_θ,L_z`, each warped
//! and linear) is sliced from that value toward NaN, ±∞, ±`f64::MAX` and ordinary values, and locked on each.
//!
//! The arm binds: on every chart the NaN target is not a system, and the stop names a check for every target that is
//! not one; a lock on one is refused, the value unchanged. A slice toward NaN or ±∞ leaves the value where it was:
//! `Φ` there is not finite (an infinite control may still decode finite through a saturating link, but it is no
//! point of the chart), and no finite value but the start lies on the path. Control: the stop disabled (each slice
//! taken whole) accepts a centre that is not a system.

use kernel::chart::construction::{HELD_K, HELD_LZ};
use kernel::chart::nav::{nav_stop, Edit, Stop, View};
use kernel::chart::tilt::{Hidden, Tiltable};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, EuclidPlane, HeldK, HeldLz,
    Mapping, MassSimplex, RestStart, ShapeMap, ShapeSphere, Tilt, TiltMap, TiltSpec, Tilted,
    ValidationResult,
};
use kernel::generated::links::{Defaults, LinkParams};
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
    MASS_SIMPLEX_EPS_M,
};

const P: LinkParams<f64> = LinkParams::DEFAULT;
/// A view's centre off `(½, ½)`.
const C: [f64; 2] = [0.37, 0.61];

/// The targets that are no value of a coordinate.
const NOT_NUMBERS: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
/// The finite targets: ordinary, large and the greatest.
const FINITE: [f64; 10] = [
    0.0,
    0.25,
    -0.7,
    3.0,
    -40.0,
    1e10,
    1e300,
    -1e300,
    f64::MAX,
    f64::MIN,
];

/// Whether a view exists and its centre decodes to a system: masses positive and finite, positions and momenta
/// finite, no degenerate reason.
fn is_system<C: Chart<Param = f64>>(view: Option<View<C>>) -> bool {
    let Some(View {
        chart,
        centre: [u, v],
    }) = view
    else {
        return false;
    };
    let d = chart.decode::<Defaults, f64>(u, v);
    d.degenerate.is_none()
        && d.ic.m.iter().all(|m| m.is_finite() && *m > 0.0)
        && d.ic
            .r
            .iter()
            .chain(&d.ic.p)
            .flatten()
            .all(|x| x.is_finite())
}

/// The slices of one held coordinate from `from` to every target, each through the stop (or, `disabled`, taken
/// whole), and a lock on each: every accepted value must be a system. Returns how many targets are not systems.
fn slices<C: Chart<Param = f64>>(
    name: &str,
    view: &dyn Fn(f64) -> Option<View<C>>,
    from: f64,
    disabled: bool,
) -> u32 {
    let start = view(from);
    assert!(
        start.as_ref().is_some_and(|w| {
            w.chart.validate_params(w.centre[0], w.centre[1], &P) == ValidationResult::Valid
        }),
        "{name}: the start {from} does not validate"
    );
    assert!(is_system(start), "{name}: the start {from} is not a system");
    let mut bad = 0;
    for &to in NOT_NUMBERS.iter().chain(&FINITE) {
        let target_bad = !is_system(view(to));
        for lock in [false, true] {
            let edit = if lock {
                Edit::Lock { from, to }
            } else {
                Edit::Path { from, to }
            };
            let s: Stop<f64> = if disabled {
                Stop {
                    value: to,
                    lambda: 1.0,
                    steps: 0,
                    stopped: None,
                }
            } else {
                nav_stop(edit, view, &P)
            };
            let what = if lock { "lock on" } else { "slice to" };
            assert!(
                is_system(view(s.value)),
                "{name}: the stop accepted {} ({what} {to}, {:?}), which is not a system",
                s.value,
                s.stopped
            );
            if s.stopped.is_none() {
                assert!(
                    s.value.to_bits() == to.to_bits(),
                    "{name}: an unstopped {what} {to} moved elsewhere, to {}",
                    s.value
                );
            }
            if target_bad {
                assert!(
                    s.stopped.is_some(),
                    "{name}: the {what} {to}, not a system, names no check"
                );
            }
            if lock && target_bad {
                assert!(
                    s.value.to_bits() == from.to_bits(),
                    "{name}: a refused lock on {to} moved the value to {}",
                    s.value
                );
            }
            if !to.is_finite() {
                // No finite value lies on a path to NaN or ±∞ but the start: `from + λ·(to − from)` is `to` or NaN
                // for every λ > 0.
                assert!(
                    s.stopped.is_some() && s.value.to_bits() == from.to_bits(),
                    "{name}: the {what} {to} left the value at {} ({:?}), not stopped where it was",
                    s.value,
                    s.stopped
                );
            }
        }
        if target_bad {
            bad += 1;
        }
    }
    println!("{name}: {bad} targets not a system");
    assert!(
        bad >= 1,
        "{name}: a NaN held coordinate reads as a system: the probe cannot bind"
    );
    bad
}

/// The slices of the held coordinate `k` of `chart`, bare.
fn bare<H: Hidden<Param = f64> + Copy>(name: &str, chart: H, k: u32, from: f64, disabled: bool) {
    assert!(chart.hidden(k), "{name}: {k} is not a held coordinate");
    let view = move |x: f64| {
        chart.sliced(k, x).map(|c| View {
            chart: c,
            centre: C,
        })
    };
    slices(&format!("{name}, held {k}"), &view, from, disabled);
}

/// The slices of the held coordinate `k` of `chart` under a tilt toward the held coordinate `target`.
fn tilted<H: Tiltable<Param = f64> + Copy>(
    name: &str,
    chart: H,
    k: u32,
    target: u32,
    from: f64,
    disabled: bool,
) {
    for (what, tau1, map) in [
        ("untilted", 0.0, TiltMap::Sheet),
        ("sheet", 25.0, TiltMap::Sheet),
        ("lean", -15.0, TiltMap::Lean),
    ] {
        let spec = TiltSpec {
            tau1,
            tau2: 0.0,
            map,
            x: target,
            y: target,
        };
        let view = move |x: f64| {
            let sliced = chart.sliced(k, x)?;
            Tilted::new(sliced, Tilt::new(&spec, C, [2.0, 2.0])).map(|t| View {
                chart: t,
                centre: C,
            })
        };
        slices(
            &format!("{name}, held {k}, tilt toward {target} ({what})"),
            &view,
            from,
            disabled,
        );
    }
}

/// A rest start's four momentum controls, from rest and from a held `0.3`, bare and tilted.
fn rest<R>(name: &str, chart: RestStart<R, f64>, disabled: bool)
where
    RestStart<R, f64>: Tiltable<Param = f64> + Copy,
{
    for k in 2..6 {
        bare(name, chart, k, 0.0, disabled);
        let moved = chart.sliced(k, 0.3).expect("a momentum control");
        bare(name, moved, k, 0.3, disabled);
        // The tilt toward the sliced control itself, and toward another.
        tilted(name, chart, k, k, 0.0, disabled);
        tilted(name, chart, k, 2 + (k - 1) % 4, 0.0, disabled);
    }
}

fn theta_range() -> [f64; 2] {
    [BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number()]
}

fn check(disabled: bool) {
    rest(
        "mass simplex",
        RestStart::new(MassSimplex {
            nu0: 0.5,
            eps_m: MASS_SIMPLEX_EPS_M.number(),
        }),
        disabled,
    );
    rest(
        "Euclid plane",
        RestStart::new(EuclidPlane {
            m_range: [EUCLID_M_MIN.number(), EUCLID_M_MAX.number()],
            nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
        }),
        disabled,
    );
    rest(
        "Φ_θ,δm",
        RestStart::new(BurrauThetaDeltaM {
            theta_range: theta_range(),
            m_target: BurrauThetaDeltaM::<f64>::equal_masses(),
        }),
        disabled,
    );

    let sphere = ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.6, 0.8],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 0.8, -0.6],
        scale: 1.0,
        inertia: 1.3,
        phase: 0.2,
        z_mass: [0.1, -0.2],
        z_mom: [0.3, 0.0, -0.25, 0.1],
    };
    for k in 2..6 {
        bare(
            "shape sphere",
            sphere,
            k,
            sphere.z_mom[(k - 2) as usize],
            disabled,
        );
    }
    for k in 6..8 {
        bare(
            "shape sphere",
            sphere,
            k,
            sphere.z_mass[(k - 6) as usize],
            disabled,
        );
    }

    for mapping in [Mapping::Warped, Mapping::Linear] {
        let nu = HeldLz {
            mapping,
            ..HeldLz::new(Burrau {
                nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
                k_max: K_MAX.number(),
                gamma_k: GAMMA_K.number(),
            })
        };
        bare(&format!("(ν, K) {mapping:?}"), nu, HELD_LZ, nu.lz, disabled);
        let theta = HeldLz {
            mapping,
            ..HeldLz::new(BurrauThetaK {
                theta_range: theta_range(),
                k_max: K_MAX.number(),
                gamma_k: GAMMA_K.number(),
            })
        };
        bare(
            &format!("Φ_θ,K {mapping:?}"),
            theta,
            HELD_LZ,
            theta.lz,
            disabled,
        );
        let lz = HeldK {
            mapping,
            ..HeldK::new(
                BurrauThetaLz {
                    theta_range: theta_range(),
                    k: BURRAU_THETA_LZ_K.number(),
                    lz_fraction: BURRAU_THETA_LZ_FRACTION.number(),
                },
                K_MAX.number(),
                GAMMA_K.number(),
            )
        };
        bare(
            &format!("Φ_θ,L_z {mapping:?}"),
            lz,
            HELD_K,
            lz.chart.k,
            disabled,
        );
    }
}

#[test]
fn qa_nav_stop_at_edge_slice_held_is_system() {
    check(false);
}

negative_control!(
    qa_nav_stop_at_edge_slice_held_is_system,
    "the stop disabled, each slice taken whole, accepts a held coordinate of NaN, a centre that is not a system",
    expected = "which is not a system",
    check(true)
);
