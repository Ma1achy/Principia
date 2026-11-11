//! QA's tests for TASK-M2-38's stop on the invariant charts under R-471's linear option, written from REQ-CHART-060
//! ("accept a pan … or chart-parameter edit only if the chart's centre … still passes validate's layers 2 and 3, a
//! centre whose Φ fails counting as failing") and R-471 ("a linear mapping is an option per axis, its infeasible
//! pixels 'not a system' and the navigation stop keeping the centre feasible"), not from the implementation.
//!
//! The oracle is the decode at the accepted centre, never the chart's own report: what the stop accepts must decode to
//! a three-body system, every mass positive and finite, every position and momentum finite, and no `DEGENERATE`
//! reason, and must validate. Probes, on `Mapped` `(L_z, E)` and `(L_z, K)` with every per-axis mix that has a linear
//! axis (R-471: the option is per axis): far pans in seeded directions at magnitudes up to `f64::MAX`, straight up,
//! down and sideways, and `K_max` / `γ_K` edits toward huge, zero and negative targets.
//!
//! The arm binds: some target is not a system and the stop is shown to hold short of it. Control: the stop disabled
//! (each edit taken whole) accepts a centre that is not a system.

use kernel::chart::invariant::Invariant;
use kernel::chart::nav::{nav_stop, Edit, Field, Stop, View};
use kernel::chart::{Chart, InvariantLE, InvariantLK, Mapped, Mapping, ValidationResult};
use kernel::generated::links::{Defaults, LinkParams};
use validation::{negative_control, GAMMA_K, K_MAX};

const P: LinkParams<f64> = LinkParams::DEFAULT;

/// The per-axis mixes with at least one linear axis.
const MIXES: [[Mapping; 2]; 3] = [
    [Mapping::Linear, Mapping::Linear],
    [Mapping::Warped, Mapping::Linear],
    [Mapping::Linear, Mapping::Warped],
];

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

/// Whether a view exists and its centre decodes to a system.
fn is_system<C: Chart<Param = f64>>(view: &Option<View<C>>) -> bool {
    let Some(View {
        chart,
        centre: [u, v],
    }) = view
    else {
        return false;
    };
    let d = chart.decode::<Defaults, f64>(*u, *v);
    d.degenerate.is_none()
        && d.ic.m.iter().all(|m| m.is_finite() && *m > 0.0)
        && d.ic
            .r
            .iter()
            .chain(&d.ic.p)
            .flatten()
            .all(|x| x.is_finite())
}

fn validates<C: Chart<Param = f64>>(view: &Option<View<C>>) -> bool {
    view.as_ref().is_some_and(|w| {
        w.chart.validate_params(w.centre[0], w.centre[1], &P) == ValidationResult::Valid
    })
}

/// Edits from `from` to each target through the stop (or taken whole when `disabled`): every accepted value must be
/// a system and validate. Returns (targets not a system, of those held short).
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
    assert!(is_system(&view(from)), "{name}: the start is not a system");
    assert!(
        validates(&view(from)),
        "{name}: the start does not validate"
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
        if !is_system(&view(to)) {
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
            assert!(
                s.steps <= 62,
                "{name}: {} bisection steps for {to:?}",
                s.steps
            );
        }
        let accepted = view(s.value);
        assert!(
            is_system(&accepted),
            "{name}: the stop accepted {:?} (edit to {to:?}, {:?}), which is not a system",
            s.value,
            s.stopped
        );
        assert!(
            validates(&accepted),
            "{name}: the stop accepted {:?} (edit to {to:?}), which does not validate",
            s.value
        );
    }
    println!("{name}: {bad} targets not a system, {held} held short");
    (bad, held)
}

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, 0.6],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

const MAGS: [f64; 9] = [0.5, 2.0, 10.0, 1e10, 1e100, 1e200, 1e300, 1e308, f64::MAX];

const SCALARS: [f64; 12] = [
    2.0,
    1e10,
    1e100,
    1e300,
    f64::MAX,
    1e-300,
    0.0,
    -1e-300,
    -1.0,
    -1e10,
    -1e308,
    -f64::MAX,
];

fn pan_targets(seed: u64) -> Vec<[f64; 2]> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for &m in &MAGS {
        for _ in 0..4 {
            let a = rng.unit() * core::f64::consts::TAU;
            out.push([0.5 + 0.5 * m * a.cos(), 0.5 + 0.5 * m * a.sin()]);
        }
        out.push([0.5, 0.5 + 0.5 * m]);
        out.push([0.5, 0.5 - 0.5 * m]);
        out.push([0.5 + 0.5 * m, 0.5]);
    }
    out.push([0.5, f64::MAX]);
    out
}

fn run<I: Copy>(
    label: &str,
    make: impl Fn(Invariant<f64>) -> I,
    seed: u64,
    disabled: bool,
) -> (u32, u32)
where
    Mapped<I, Defaults>: Chart<Param = f64> + Copy,
{
    let mut total = (0, 0);
    for (k, mix) in MIXES.iter().enumerate() {
        let name = format!("{label} {mix:?}");
        let chart = Mapped::<I, Defaults>::new(make(invariant()), *mix);
        let s = seed + k as u64;
        println!("{name}: pan seed {s:#x}");
        let mut add = |r: (u32, u32)| total = (total.0 + r.0, total.1 + r.1);
        add(edits(
            &format!("{name}: pan"),
            &|c| Some(View { chart, centre: c }),
            [0.5, 0.5],
            &pan_targets(s),
            disabled,
        ));
        let base = invariant();
        let at = |inv: Invariant<f64>| {
            Some(View {
                chart: Mapped::<I, Defaults>::new(make(inv), *mix),
                centre: [0.5, 0.5],
            })
        };
        add(edits(
            &format!("{name}: K_max"),
            &|x: f64| at(Invariant { k_max: x, ..base }),
            base.k_max,
            &SCALARS,
            disabled,
        ));
        add(edits(
            &format!("{name}: γ_K"),
            &|x: f64| at(Invariant { gamma_k: x, ..base }),
            base.gamma_k,
            &SCALARS,
            disabled,
        ));
    }
    total
}

fn check(disabled: bool) {
    let le = run("(L_z, E)", InvariantLE, 0x0038_1E00, disabled);
    let lk = run("(L_z, K)", InvariantLK, 0x0038_1C00, disabled);
    for (name, (bad, held)) in [("(L_z, E)", le), ("(L_z, K)", lk)] {
        // R-471: under the linear option a far pan reaches infeasible or overflowing centres.
        assert!(
            bad > 0,
            "{name}: no target is not a system: the probe never reached the edge"
        );
        if !disabled {
            assert!(held > 0, "{name}: the stop never held an edit short");
        }
    }
}

#[test]
fn qa_nav_stop_at_edge_linear_centre_is_a_system() {
    check(false);
}

negative_control!(
    qa_nav_stop_at_edge_linear_centre_is_a_system,
    "the stop disabled, each edit taken whole, accepts a centre that is not a system on a linear invariant chart",
    expected = "which is not a system",
    check(true)
);
