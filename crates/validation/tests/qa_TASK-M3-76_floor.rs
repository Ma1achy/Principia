//! QA tests for TASK-M3-76 (REQ-INT-119; R-1008's rider, the second reference run telling round-off from a defect): the
//! convergence arm's round-off floor is the larger of the stated floor and the measured one (the distance from the
//! reference run to the second reference run), and a measured floor that is not finite fails the arm before any rung
//! is selected, read neither as a pass (a NaN falling back to the stated floor) nor as round-off (an infinite floor
//! dropping every rung). Written from the requirement, independently of `az_diagnose_floor`: the non-finite floor is
//! set beside ladders that would otherwise read within, a landing defect or round-off, and beside a non-finite rung;
//! and each of the two floors is shown to bind where it is the larger.

// The negative controls' variants, compiled only under `controls`, rebuild the arm from this without the check.
#[cfg(feature = "controls")]
use validation::az::diagnose_rungs;
use validation::az::{diagnose_floor, Diagnosis, FloorCriterion};
use validation::negative_control;

/// A diagnosis of the ladder's rungs `(η, error)` against a measured floor, under a criterion, an expected order and a
/// margin.
type Diagnoser = fn(&[(f64, f64)], f64, &FloorCriterion, f64, f64) -> Diagnosis;

fn fc(stated: f64) -> FloorCriterion {
    FloorCriterion {
        stated,
        second_eta: 1e-5,
        factor: 10.0,
        min_rungs: 3,
        defect_below: 3.0,
    }
}

/// Rungs `(h, c·h^p)` over the ladder `[0.02, 0.01, 0.005, 0.002, 0.001]`.
fn ladder(p: i32, c: f64) -> Vec<(f64, f64)> {
    [0.02, 0.01, 0.005, 0.002, 0.001]
        .iter()
        .map(|&h: &f64| (h, c * h.powi(p)))
        .collect()
}

fn is_fail(d: Diagnosis) -> bool {
    !matches!(d, Diagnosis::Within { .. } | Diagnosis::RoundOff { .. })
}

fn check_nonfinite_floor(diag: Diagnoser) {
    let fc = fc(1e-15);
    let d = |rungs: &[(f64, f64)], measured: f64| diag(rungs, measured, &fc, 4.0, 0.5);

    // Baselines with a finite measured floor, so each case below is a change of verdict.
    let fourth = ladder(4, 1.0);
    let second = ladder(2, 1.0);
    let low: Vec<(f64, f64)> = fourth.iter().map(|&(h, _)| (h, 1e-15)).collect();
    assert!(
        matches!(d(&fourth, 1e-16), Diagnosis::Within { .. }),
        "a clean fourth-order ladder reads within: {:?}",
        d(&fourth, 1e-16)
    );
    assert!(
        matches!(d(&second, 1e-16), Diagnosis::LandingDefect { .. }),
        "a second-order ladder reads a landing defect: {:?}",
        d(&second, 1e-16)
    );
    assert!(
        matches!(d(&low, 1e-16), Diagnosis::RoundOff { .. }),
        "a ladder at the floor reads round-off: {:?}",
        d(&low, 1e-16)
    );

    // A NaN measured floor beside a ladder that would pass: the arm fails, never falls back to the stated floor.
    let v = d(&fourth, f64::NAN);
    assert!(
        matches!(v, Diagnosis::NonFiniteFloor { measured } if measured.is_nan()),
        "a NaN measured floor beside a clean ladder fails the arm: {v:?}"
    );
    // An infinite measured floor beside the same ladder: not round-off (every rung dropped), a failure.
    let v = d(&fourth, f64::INFINITY);
    assert!(
        matches!(v, Diagnosis::NonFiniteFloor { measured } if measured == f64::INFINITY),
        "an infinite measured floor beside a clean ladder fails the arm, not round-off: {v:?}"
    );
    // A non-finite floor beside a second-order ladder and beside a ladder at the floor: still the floor's failure.
    for rungs in [&second, &low] {
        for m in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let v = d(rungs, m);
            assert!(
                matches!(v, Diagnosis::NonFiniteFloor { .. }),
                "a non-finite measured floor {m} fails the arm whatever the rungs: {v:?}"
            );
        }
    }
    // A non-finite floor beside a non-finite rung: a failure, never a pass nor round-off.
    let mut fourth_nan = fourth.clone();
    fourth_nan[1].1 = f64::NAN;
    for m in [f64::NAN, f64::INFINITY] {
        let v = d(&fourth_nan, m);
        assert!(
            is_fail(v)
                && matches!(
                    v,
                    Diagnosis::NonFiniteFloor { .. } | Diagnosis::NonFinite { .. }
                ),
            "a non-finite floor beside a NaN rung fails the arm as non-finite: {v:?}"
        );
    }
    // Finite extremes are not non-finite: a measured floor of 0 leaves the stated floor, under which the ladder passes.
    assert!(
        matches!(d(&fourth, 0.0), Diagnosis::Within { .. }),
        "a measured floor of 0 is finite: {:?}",
        d(&fourth, 0.0)
    );
}

/// The floor is the larger of the stated and the measured: each binds where it is the larger. Over the fourth-order
/// ladder (errors 1.6e-7, 1e-8, 6.25e-10, 1.6e-11, 1e-12), a floor of 1e-10 keeps only the two rungs above 1e-9, too few
/// to fit (round-off); a floor of 1e-20 keeps all five, which read order 4.
fn check_larger_floor(diag: Diagnoser) {
    let fourth = ladder(4, 1.0);
    let d = |stated: f64, measured: f64| diag(&fourth, measured, &fc(stated), 4.0, 0.5);
    assert!(
        matches!(d(1e-20, 1e-20), Diagnosis::Within { .. }),
        "both floors low: the ladder reads within: {:?}",
        d(1e-20, 1e-20)
    );
    assert_eq!(
        d(1e-10, 1e-20),
        Diagnosis::RoundOff { kept: 2 },
        "the stated floor binds where it is the larger"
    );
    assert_eq!(
        d(1e-20, 1e-10),
        Diagnosis::RoundOff { kept: 2 },
        "the measured floor binds where it is the larger"
    );
}

#[test]
fn qa_nonfinite_measured_floor_fails_the_arm() {
    check_nonfinite_floor(diagnose_floor);
}

negative_control!(
    qa_nonfinite_measured_floor_fails_the_arm,
    "the measured floor taken as the larger with the stated one, unchecked: a NaN falls back to the stated floor",
    expected = "a NaN measured floor beside a clean ladder fails the arm",
    check_nonfinite_floor(|rungs, measured, fc, expected, margin| {
        diagnose_rungs(rungs, measured.max(fc.stated), fc, expected, margin)
    })
);

#[test]
fn qa_floor_is_the_larger_of_stated_and_measured() {
    check_larger_floor(diagnose_floor);
}

negative_control!(
    qa_floor_is_the_larger_of_stated_and_measured,
    "the measured floor alone, the stated floor ignored",
    expected = "the stated floor binds where it is the larger",
    check_larger_floor(|rungs, measured, fc, expected, margin| {
        if measured.is_finite() {
            diagnose_rungs(rungs, measured, fc, expected, margin)
        } else {
            Diagnosis::NonFiniteFloor { measured }
        }
    })
);
