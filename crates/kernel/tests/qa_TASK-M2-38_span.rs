//! QA's tests for TASK-M2-38's stop on an edit whose finite ends lie more than `f64::MAX` apart, written from
//! REQ-CHART-060 ("an edit that would cross must stop at the last valid value along its own path, found by bisection on
//! the edit's fraction λ ∈ [0, 1] … until the two ends are adjacent f64 values (at most 62 steps), the valid end
//! kept, and must report the layer and the constraint that stopped it"), not from the implementation.
//!
//! The probe is the shape sphere's inertia, a system at every positive value tried here and none at a negative one
//! (R-469), edited from a huge positive value to a huge negative one, the two finite and their difference past
//! `f64::MAX`. The path is the straight segment between them; it crosses zero at `λ* = from / (from − to)`, strictly
//! inside `(0, 1)`, and every value before the crossing is positive. The oracle is that geometry and the decode at the
//! accepted value, never the path the implementation computes:
//! - the accepted value decodes to a system and validates;
//! - it is the *last* valid value: two adjacent f64s of `[0, 1]` differ by at most `ε/2`, so the valid end of the
//!   bisection lies within one such step of the crossing, an inertia no more than `4ε·(|from| + |to|)` above zero
//!   (the step itself, and the rounding of a value of that magnitude, each bounded by `ε·(|from| + |to|)`);
//! - its `λ` is the fraction of the segment it sits at, `(from − value) / (from − to)`, formed on halved ends so
//!   nothing overflows, to `4ε`;
//! - at most 62 steps, the check named;
//! - on a two-component field, the component the edit leaves alone keeps its value bitwise.
//!
//! The premise binds: each start, and a ladder of inertias from it down to the bound, are asserted systems, so the
//! valid part of the path reaches the bound. Control: a field whose path forms `to − from` first, infinite here, puts
//! every interior value at −∞ and the stop stays where it began.

use kernel::chart::nav::{nav_stop, Edit, Field, Stop, View};
use kernel::chart::{Chart, ShapeMap, ShapeSphere, ValidationResult};
use kernel::generated::links::{Defaults, LinkParams};
use validation::negative_control;

const P: LinkParams<f64> = LinkParams::DEFAULT;

/// Whether the view's centre decodes to a system: masses positive and finite, positions and momenta finite, no
/// degenerate reason.
fn is_system<C: Chart<Param = f64>>(view: &View<C>) -> bool {
    let d = view
        .chart
        .decode::<Defaults, f64>(view.centre[0], view.centre[1]);
    d.degenerate.is_none()
        && d.ic.m.iter().all(|m| m.is_finite() && *m > 0.0)
        && d.ic
            .r
            .iter()
            .chain(&d.ic.p)
            .flatten()
            .all(|x| x.is_finite())
}

fn validates<C: Chart<Param = f64>>(view: &View<C>) -> bool {
    view.chart
        .validate_params(view.centre[0], view.centre[1], &P)
        == ValidationResult::Valid
}

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

fn view(map: ShapeMap, inertia: f64, phase: f64) -> View<ShapeSphere<f64>> {
    View {
        chart: ShapeSphere {
            inertia,
            phase,
            ..shape(map)
        },
        centre: [0.5, 0.5],
    }
}

/// A scalar field whose path is `from + λ·(to − from)` with the difference formed first (the control): −∞ at every
/// interior `λ` once the ends lie more than `f64::MAX` apart.
#[derive(Clone, Copy, Debug, PartialEq)]
struct DifferenceFirst(f64);

impl Field for DifferenceFirst {
    fn along(self, to: Self, lambda: f64) -> Self {
        if lambda == 0.0 {
            self
        } else if lambda == 1.0 {
            to
        } else {
            DifferenceFirst(self.0 + lambda * (to.0 - self.0))
        }
    }
}

/// The ends of the edits: each start a positive inertia, each target negative, their difference past `f64::MAX`.
const ENDS: [(f64, f64); 4] = [
    (1e307, -f64::MAX),
    (1e307, -1.75e308),
    (5e307, -1.4e308),
    (2e306, -f64::MAX),
];

/// What REQ-CHART-060 asks of the stop of an inertia edit `from → to` across zero.
fn assert_last_valid(name: &str, map: ShapeMap, from: f64, to: f64, phase: f64, s: &Stop<f64>) {
    let bound = 4.0 * f64::EPSILON * from.abs() + 4.0 * f64::EPSILON * to.abs();
    assert!(
        s.stopped.is_some(),
        "{name}: the edit {from:e} → {to:e} ends at a negative inertia and was not stopped"
    );
    assert!(
        s.steps <= 62,
        "{name}: {} bisection steps, more than 62",
        s.steps
    );
    let at = view(map, s.value, phase);
    assert!(
        is_system(&at) && validates(&at),
        "{name}: the stop accepted inertia {:e}, which is not a system",
        s.value
    );
    assert!(
        s.value > 0.0 && s.value <= bound,
        "{name}: the edit {from:e} → {to:e} stopped at inertia {:e} (λ = {:e}), not the last valid value: every \
         inertia down to {bound:e} is on the path and a system",
        s.value,
        s.lambda
    );
    let fraction = (from / 2.0 - s.value / 2.0) / (from / 2.0 - to / 2.0);
    assert!(
        (s.lambda - fraction).abs() <= 4.0 * f64::EPSILON,
        "{name}: the stop reports λ = {:e}, its value {:e} sits at {fraction:e} of the path",
        s.lambda,
        s.value
    );
    assert!(
        s.lambda > 0.0 && s.lambda < 1.0,
        "{name}: a stop at λ = {}",
        s.lambda
    );
}

fn check(difference_first: bool) {
    for map in [ShapeMap::Spherical, ShapeMap::Exponential] {
        for (from, to) in ENDS {
            let name = format!("{map:?} inertia {from:e} → {to:e}");
            assert!(
                from.is_finite() && to.is_finite() && (to - from).is_infinite(),
                "{name}: the ends are not finite and more than f64::MAX apart: the case tells nothing"
            );
            let bound = 4.0 * f64::EPSILON * from.abs() + 4.0 * f64::EPSILON * to.abs();
            // The premise: the valid part of the path reaches the bound.
            let mut rung = from;
            while rung > bound {
                let at = view(map, rung, 0.2);
                assert!(
                    is_system(&at) && validates(&at),
                    "{name}: inertia {rung:e}, between the start and the bound, is not a system"
                );
                rung /= 8.0;
            }
            let at = view(map, bound, 0.2);
            assert!(
                is_system(&at) && validates(&at),
                "{name}: the bound {bound:e} is not a system"
            );
            assert!(
                !is_system(&view(map, to, 0.2)),
                "{name}: the negative inertia asked for is a system: the edit crosses no edge"
            );

            let s: Stop<f64> = if difference_first {
                let s = nav_stop(
                    Edit::Path {
                        from: DifferenceFirst(from),
                        to: DifferenceFirst(to),
                    },
                    |x: DifferenceFirst| Some(view(map, x.0, 0.2)),
                    &P,
                );
                Stop {
                    value: s.value.0,
                    lambda: s.lambda,
                    steps: s.steps,
                    stopped: s.stopped,
                }
            } else {
                nav_stop(
                    Edit::Path { from, to },
                    |x: f64| Some(view(map, x, 0.2)),
                    &P,
                )
            };
            println!(
                "{name}: stopped at {:e} (λ = {:e}, {} steps, {:?}), bound {bound:e}",
                s.value, s.lambda, s.steps, s.stopped
            );
            assert_last_valid(&name, map, from, to, 0.2, &s);

            if !difference_first {
                // The same edit as one of two components, the phase left alone.
                let phase = 0.2f64;
                let pair = nav_stop(
                    Edit::Path {
                        from: [from, phase],
                        to: [to, phase],
                    },
                    |x: [f64; 2]| Some(view(map, x[0], x[1])),
                    &P,
                );
                assert_eq!(
                    pair.value[1].to_bits(),
                    phase.to_bits(),
                    "{name}: the phase, which the edit leaves alone, moved"
                );
                let scalar = Stop {
                    value: pair.value[0],
                    lambda: pair.lambda,
                    steps: pair.steps,
                    stopped: pair.stopped,
                };
                assert_last_valid(
                    &format!("{name} (with the phase)"),
                    map,
                    from,
                    to,
                    phase,
                    &scalar,
                );

                // A lock across the same span is refused, the value unchanged.
                let lock = nav_stop(
                    Edit::Lock { from, to },
                    |x: f64| Some(view(map, x, 0.2)),
                    &P,
                );
                assert_eq!(
                    lock.value.to_bits(),
                    from.to_bits(),
                    "{name}: a refused lock moved"
                );
                assert!(
                    lock.stopped.is_some(),
                    "{name}: a lock on a negative inertia was taken"
                );
            }
        }
    }
}

#[test]
fn qa_nav_stop_at_edge_span_past_f64_max() {
    check(false);
}

negative_control!(
    qa_nav_stop_at_edge_span_past_f64_max,
    "a path that forms the difference of its ends first leaves an edit across more than f64::MAX where it began",
    expected = "not the last valid value",
    check(true)
);
