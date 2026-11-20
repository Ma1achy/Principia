//! QA's tests for TASK-M2-38's `Φ` arm, written from REQ-CHART-060 ("a centre whose Φ fails counting as failing"),
//! not from the implementation.
//!
//! `Φ` is what the chart builds at its centre, the configuration the decode returns. The oracle here is that decode
//! itself, never the chart's own finiteness report: every centre the stop accepts must decode to finite masses,
//! positions and momenta. Pans from a valid centre to targets of every magnitude, up to `f64::MAX`, in seeded
//! directions, on charts whose `Φ` can overflow far out (the exponential shape sphere, whose point is built outside
//! `map`, at several extents, bare and tilted) and on the other charts the stop serves (the spherical shape sphere,
//! the invariant `(L_z, E)` and `(L_z, K)` charts linear and warped, the mass simplex). The arm binds: on the
//! exponential shape sphere some pan is refused or stopped on `Φ`, the target there decoding non-finite.
//! Control: the stop disabled (each pan taken whole) accepts a centre that decodes non-finite.

use kernel::chart::invariant::Invariant;
use kernel::chart::nav::{nav_stop, Check, Edit, Stop, View};
use kernel::chart::{
    Chart, InvariantLE, InvariantLK, Mapped, Mapping, MassSimplex, RestStart, ShapeMap,
    ShapeSphere, Tilt, TiltMap, TiltSpec, Tilted, ValidationResult,
};
use kernel::generated::links::{Defaults, LinkParams};
use validation::{negative_control, GAMMA_K, K_MAX, MASS_SIMPLEX_EPS_M};

const P: LinkParams<f64> = LinkParams::DEFAULT;

/// SplitMix64, seeded.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Whether the configuration the chart decodes at `(u, v)` is finite: every mass, position and momentum.
fn decodes_finite<C: Chart<Param = f64>>(chart: &C, u: f64, v: f64) -> bool {
    let ic = chart.decode::<Defaults, f64>(u, v).ic;
    ic.m.iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .all(|x| x.is_finite())
}

/// The magnitudes of the pans' offsets from the start: ordinary, and past where a square overflows (≈ 1.3·10¹⁵⁴).
const MAGNITUDES: [f64; 12] = [
    1.0,
    10.0,
    1e10,
    1e50,
    1e100,
    1e150,
    1e154,
    1e155,
    1e200,
    1e300,
    1e308,
    f64::MAX,
];

/// Pans from `start` toward targets at every magnitude in seeded directions, each through the stop (or with
/// `disabled` taken whole): every accepted centre must decode finite. Returns how many targets decode non-finite and
/// how many pans stopped on `Φ`.
fn pans<C: Chart<Param = f64>>(
    name: &str,
    view: &dyn Fn([f64; 2]) -> Option<View<C>>,
    start: [f64; 2],
    seed: u64,
    disabled: bool,
) -> (u32, u32) {
    println!("{name}: seed {seed:#x}");
    let v0 = view(start).expect("a view at the start");
    assert!(
        decodes_finite(&v0.chart, start[0], start[1]),
        "{name}: the start decodes non-finite"
    );
    assert!(
        matches!(
            v0.chart.validate_params(start[0], start[1], &P),
            ValidationResult::Valid
        ),
        "{name}: the start does not validate"
    );
    let mut rng = Rng(seed);
    let (mut bad_targets, mut phi_stops) = (0, 0);
    for &mag in &MAGNITUDES {
        for _ in 0..8 {
            let a = rng.unit() * std::f64::consts::TAU;
            // Scaled so that `start + offset` never overflows to infinity itself: the target is finite.
            let m = mag * 0.5;
            let to = [start[0] + m * a.cos(), start[1] + m * a.sin()];
            assert!(to.iter().all(|x| x.is_finite()));
            let target_bad = view(to).is_none_or(|t| !decodes_finite(&t.chart, to[0], to[1]));
            if target_bad {
                bad_targets += 1;
            }
            let s: Stop<[f64; 2]> = if disabled {
                Stop {
                    value: to,
                    lambda: 1.0,
                    steps: 0,
                    stopped: None,
                }
            } else {
                nav_stop(Edit::Path { from: start, to }, view, &P)
            };
            if s.stopped == Some(Check::Phi) {
                phi_stops += 1;
            }
            if s.stopped.is_none() {
                assert_eq!(
                    s.value, to,
                    "{name}: an unstopped pan to {to:?} moved elsewhere"
                );
            }
            let accepted = view(s.value).expect("a view at the accepted centre");
            assert!(
                decodes_finite(&accepted.chart, s.value[0], s.value[1]),
                "{name}: the stop accepted {:?} (pan to {to:?}, {:?}), which decodes non-finite",
                s.value,
                s.stopped
            );
        }
    }
    println!("{name}: {bad_targets} targets decode non-finite, {phi_stops} stopped on Φ");
    (bad_targets, phi_stops)
}

fn shape(map: ShapeMap, scale: f64) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0: [0.0, 0.6, 0.8],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 0.8, -0.6],
        scale,
        inertia: 1.3,
        phase: 0.2,
        z_mass: [0.1, -0.2],
        z_mom: [0.3, 0.0, -0.25, 0.1],
    }
}

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, 0.6],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

fn plain<C: Copy>(chart: C) -> impl Fn([f64; 2]) -> Option<View<C>> {
    move |c| Some(View { chart, centre: c })
}

fn check(disabled: bool) {
    let mut seed = 0x0000_60F1;
    // The exponential shape sphere: its point is built outside `map` and overflows far out; the arm must bind.
    for scale in [0.3, std::f64::consts::FRAC_PI_4, 2.5] {
        let chart = shape(ShapeMap::Exponential, scale);
        let (bad, phi) = pans(
            &format!("exponential shape sphere, extent {scale}"),
            &plain(chart),
            [0.5, 0.5],
            seed,
            disabled,
        );
        seed += 1;
        assert!(
            bad > 0,
            "no target decodes non-finite: the probe never reached the overflow"
        );
        if !disabled {
            assert!(phi > 0, "no pan stopped on Φ");
        }
        for (what, tau1, map) in [
            ("untilted", 0.0, TiltMap::Sheet),
            ("tilted, sheet", 25.0, TiltMap::Sheet),
            ("tilted, lean", -15.0, TiltMap::Lean),
        ] {
            let spec = TiltSpec {
                tau1,
                tau2: 0.0,
                map,
                x: 2,
                y: 2,
            };
            let view = move |c: [f64; 2]| {
                Tilted::new(chart, Tilt::new(&spec, c, [2.0, 2.0])).map(|t| View {
                    chart: t,
                    centre: c,
                })
            };
            pans(
                &format!("exponential shape sphere, extent {scale}, {what}"),
                &view,
                [0.5, 0.5],
                seed,
                disabled,
            );
            seed += 1;
        }
    }
    pans(
        "spherical shape sphere",
        &plain(shape(ShapeMap::Spherical, 1.0)),
        [0.4, 0.55],
        seed,
        disabled,
    );
    seed += 1;
    for mapping in [Mapping::Linear, Mapping::Warped] {
        pans(
            &format!("(L_z, E) {mapping:?}"),
            &plain(Mapped::<_, Defaults>::new(
                InvariantLE(invariant()),
                [mapping; 2],
            )),
            [0.5, 0.5],
            seed,
            disabled,
        );
        seed += 1;
        pans(
            &format!("(L_z, K) {mapping:?}"),
            &plain(Mapped::<_, Defaults>::new(
                InvariantLK(invariant()),
                [mapping; 2],
            )),
            [0.5, 0.5],
            seed,
            disabled,
        );
        seed += 1;
    }
    let simplex = RestStart::new(MassSimplex {
        nu0: 0.5,
        eps_m: MASS_SIMPLEX_EPS_M.number(),
    });
    pans("mass simplex", &plain(simplex), [0.4, 0.3], seed, disabled);
}

#[test]
fn qa_nav_stop_at_edge_phi_decodes_finite() {
    check(false);
}

negative_control!(
    qa_nav_stop_at_edge_phi_decodes_finite,
    "the stop disabled, each pan taken whole, accepts a centre whose Φ is not finite",
    expected = "decodes non-finite",
    check(true)
);
