//! QA's tests for TASK-M2-38's `Φ` arm on chart-parameter edits and far pans, written from REQ-CHART-060 ("accept a
//! pan, slice, zoom, tilt, lock or chart-parameter edit only if the chart's centre … still passes validate's layers 2
//! and 3, a centre whose Φ fails counting as failing"), not from the implementation.
//!
//! The oracle is the decode at the accepted centre, never the chart's own report: what the stop accepts must decode to
//! a three-body system, every mass positive and finite, every position and momentum finite, and no `DEGENERATE`
//! reason (a degenerate decode is "not a system", R-469). Probes:
//! - **Chart-parameter edits on the shape sphere**: the inertia, the phase, the extent, the held mass and momentum
//!   blocks, each edited from a valid value toward ordinary, huge (up to `f64::MAX`), zero and negative targets.
//! - **Chart-parameter edits on the invariant charts**: `K_max` and `γ_K`, toward huge, zero and negative targets.
//! - **Far pans on the invariant charts** (`(L_z, E)` and `(L_z, K)`, bare, warped `Mapped` and under an untilted
//!   tilt), in seeded directions at magnitudes up to `f64::MAX`.
//!
//! The arm binds: some target on each family is not a system, and the stop is shown to refuse or stop there. Control:
//! the stop disabled (each edit taken whole) accepts a centre that is not a system.

use kernel::chart::invariant::Invariant;
use kernel::chart::nav::{nav_stop, Edit, Field, Stop, View};
use kernel::chart::{
    Chart, InvariantLE, InvariantLK, Mapped, Mapping, ShapeMap, ShapeSphere, Tilt, TiltMap,
    TiltSpec, Tilted, ValidationResult,
};
use kernel::generated::links::{Defaults, LinkParams};
use validation::{negative_control, GAMMA_K, K_MAX};

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

/// Whether a view exists and its centre validates at `P`.
fn validates<C: Chart<Param = f64>>(view: &Option<View<C>>) -> bool {
    view.as_ref().is_some_and(|w| {
        w.chart.validate_params(w.centre[0], w.centre[1], &P) == ValidationResult::Valid
    })
}

/// Edits from `from` to each of `targets`, each through the stop (or, `disabled`, taken whole): every accepted value
/// must be a system. Returns how many targets are not systems, and how many of those the stop held short of.
fn edits<V, C>(
    name: &str,
    view: &dyn Fn(V) -> Option<View<C>>,
    from: V,
    targets: &[V],
    disabled: bool,
) -> (u32, u32)
where
    V: Field + PartialEq + core::fmt::Debug,
    C: Chart<Param = f64>,
{
    assert!(
        is_system(view(from)),
        "{name}: the start {from:?} is not a system"
    );
    assert!(
        validates(&view(from)),
        "{name}: the start {from:?} does not validate"
    );
    let (mut bad, mut held) = (0, 0);
    for &to in targets {
        let s: Stop<V> = if disabled {
            Stop {
                value: to,
                lambda: 1.0,
                steps: 0,
                stopped: None,
            }
        } else {
            nav_stop(Edit::Path { from, to }, view, &P)
        };
        let target_bad = !is_system(view(to));
        if target_bad {
            bad += 1;
            if s.value != to {
                held += 1;
            }
        }
        if s.stopped.is_none() {
            assert_eq!(
                s.value, to,
                "{name}: an unstopped edit to {to:?} moved elsewhere"
            );
        } else {
            assert!(s.lambda < 1.0, "{name}: a stop at λ = 1 for {to:?}");
        }
        assert!(
            is_system(view(s.value)),
            "{name}: the stop accepted {:?} (edit to {to:?}, {:?}), which is not a system",
            s.value,
            s.stopped
        );
    }
    println!("{name}: {bad} targets not a system, {held} held short");
    (bad, held)
}

/// The scalar targets of a chart-parameter edit: ordinary, huge, tiny, zero, negative.
const SCALARS: [f64; 16] = [
    0.5,
    2.0,
    10.0,
    1e10,
    1e100,
    1e200,
    1e300,
    1e308,
    f64::MAX,
    1e-300,
    0.0,
    -1e-300,
    -1.0,
    -1e10,
    -1e308,
    -f64::MAX,
];

fn shape(map: ShapeMap) -> ShapeSphere<f64> {
    ShapeSphere {
        map,
        n0: [0.0, 0.6, 0.8],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 0.8, -0.6],
        scale: 0.7,
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

/// Seeded vector targets of dimension `N` at every magnitude of [`SCALARS`]' positive half, offset from `from`.
fn vectors<const N: usize>(from: [f64; N], rng: &mut Rng) -> Vec<[f64; N]> {
    let mut out = Vec::new();
    for &mag in &SCALARS[..9] {
        for _ in 0..4 {
            let dir: [f64; N] = core::array::from_fn(|_| rng.unit() * 2.0 - 1.0);
            let norm = dir.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
            // Halved so that `from + offset` is finite itself.
            out.push(core::array::from_fn(|i| {
                from[i] + 0.5 * mag * dir[i] / norm
            }));
        }
    }
    out
}

fn shape_params(disabled: bool) {
    let mut rng = Rng(0x0000_38A5);
    println!("shape sphere parameter edits: seed {:#x}", rng.0);
    let mut total = (0, 0);
    let mut add = |(b, h): (u32, u32)| {
        total.0 += b;
        total.1 += h;
    };
    for map in [ShapeMap::Exponential, ShapeMap::Spherical] {
        let base = shape(map);
        let at = move |c: ShapeSphere<f64>| {
            Some(View {
                chart: c,
                centre: [0.5, 0.5],
            })
        };
        let inertia = edits(
            &format!("{map:?}: inertia"),
            &|x: f64| at(ShapeSphere { inertia: x, ..base }),
            base.inertia,
            &SCALARS,
            disabled,
        );
        // R-469: an inertia of zero or below is no configuration, and toward f64::MAX the configuration overflows.
        assert!(inertia.0 > 0, "{map:?}: no inertia target is not a system");
        add(inertia);
        add(edits(
            &format!("{map:?}: phase"),
            &|x: f64| at(ShapeSphere { phase: x, ..base }),
            base.phase,
            &SCALARS,
            disabled,
        ));
        add(edits(
            &format!("{map:?}: extent"),
            &|x: f64| at(ShapeSphere { scale: x, ..base }),
            base.scale,
            &SCALARS,
            disabled,
        ));
        let targets = vectors(base.z_mass, &mut rng);
        add(edits(
            &format!("{map:?}: held mass block"),
            &|x: [f64; 2]| at(ShapeSphere { z_mass: x, ..base }),
            base.z_mass,
            &targets,
            disabled,
        ));
        let targets = vectors(base.z_mom, &mut rng);
        add(edits(
            &format!("{map:?}: held momentum block"),
            &|x: [f64; 4]| at(ShapeSphere { z_mom: x, ..base }),
            base.z_mom,
            &targets,
            disabled,
        ));
    }
    if !disabled {
        assert!(
            total.1 > 0,
            "the stop never held a shape-sphere parameter edit short"
        );
    }
}

/// `K_max` and `γ_K` edits on one invariant chart, `make` building it from the edited registers.
fn invariant_chart_params<C: Chart<Param = f64>>(
    name: &str,
    make: &dyn Fn(Invariant<f64>) -> C,
    disabled: bool,
) -> (u32, u32) {
    let base = invariant();
    let at = |inv: Invariant<f64>| {
        Some(View {
            chart: make(inv),
            centre: [0.5, 0.5],
        })
    };
    let a = edits(
        &format!("{name}: K_max"),
        &|x: f64| at(Invariant { k_max: x, ..base }),
        base.k_max,
        &SCALARS,
        disabled,
    );
    let b = edits(
        &format!("{name}: γ_K"),
        &|x: f64| at(Invariant { gamma_k: x, ..base }),
        base.gamma_k,
        &SCALARS,
        disabled,
    );
    (a.0 + b.0, a.1 + b.1)
}

fn invariant_params(disabled: bool) {
    let runs = [
        invariant_chart_params("(L_z, E)", &|i| InvariantLE(i), disabled),
        invariant_chart_params("(L_z, K)", &|i| InvariantLK(i), disabled),
        invariant_chart_params(
            "warped (L_z, E)",
            &|i| Mapped::<_, Defaults>::new(InvariantLE(i), [Mapping::Warped; 2]),
            disabled,
        ),
        invariant_chart_params(
            "warped (L_z, K)",
            &|i| Mapped::<_, Defaults>::new(InvariantLK(i), [Mapping::Warped; 2]),
            disabled,
        ),
    ];
    let bad: u32 = runs.iter().map(|r| r.0).sum();
    let held: u32 = runs.iter().map(|r| r.1).sum();
    println!("invariant parameter edits: {bad} targets not a system, {held} held short");
}

/// Far pans of the centre on one invariant chart in seeded directions, magnitudes up to `f64::MAX`.
fn invariant_pans<C: Chart<Param = f64>>(
    name: &str,
    view: &dyn Fn([f64; 2]) -> Option<View<C>>,
    seed: u64,
    disabled: bool,
) -> (u32, u32) {
    let mut rng = Rng(seed);
    println!("{name}: seed {seed:#x}");
    let mut targets = vectors([0.5, 0.5], &mut rng);
    // Straight up the vertical axis, where `K` grows without bound.
    targets.extend(SCALARS[..9].iter().map(|&m| [0.5, 0.5 + 0.5 * m]));
    edits(name, view, [0.5, 0.5], &targets, disabled)
}

fn pans(disabled: bool) {
    let le = InvariantLE(invariant());
    let lk = InvariantLK(invariant());
    let spec = TiltSpec {
        tau1: 0.0,
        tau2: 0.0,
        map: TiltMap::Sheet,
        x: 0,
        y: 0,
    };
    let runs = [
        invariant_pans(
            "pan (L_z, E)",
            &|c| {
                Some(View {
                    chart: le,
                    centre: c,
                })
            },
            0x60A1,
            disabled,
        ),
        invariant_pans(
            "pan (L_z, K)",
            &|c| {
                Some(View {
                    chart: lk,
                    centre: c,
                })
            },
            0x60A2,
            disabled,
        ),
        invariant_pans(
            "pan warped (L_z, E)",
            &|c| {
                Some(View {
                    chart: Mapped::<_, Defaults>::new(le, [Mapping::Warped; 2]),
                    centre: c,
                })
            },
            0x60A3,
            disabled,
        ),
        invariant_pans(
            "pan warped (L_z, K)",
            &|c| {
                Some(View {
                    chart: Mapped::<_, Defaults>::new(lk, [Mapping::Warped; 2]),
                    centre: c,
                })
            },
            0x60A4,
            disabled,
        ),
        invariant_pans(
            "pan untilted (L_z, E)",
            &|c| {
                Tilted::new(le, Tilt::new(&spec, c, [2.0, 2.0])).map(|t| View {
                    chart: t,
                    centre: c,
                })
            },
            0x60A5,
            disabled,
        ),
    ];
    let bad: u32 = runs.iter().map(|r| r.0).sum();
    let held: u32 = runs.iter().map(|r| r.1).sum();
    // R-469: the invariant construction past the overflow edge is no system; a pan straight up reaches it.
    assert!(
        bad > 0,
        "no invariant pan target is not a system: the probe never reached the edge"
    );
    if !disabled {
        assert!(held > 0, "the stop never held an invariant pan short");
    }
}

fn check(disabled: bool) {
    shape_params(disabled);
    invariant_params(disabled);
    pans(disabled);
}

#[test]
fn qa_nav_stop_at_edge_centre_is_a_system() {
    check(false);
}

negative_control!(
    qa_nav_stop_at_edge_centre_is_a_system,
    "the stop disabled, each edit taken whole, accepts a centre that is not a system",
    expected = "which is not a system",
    check(true)
);
