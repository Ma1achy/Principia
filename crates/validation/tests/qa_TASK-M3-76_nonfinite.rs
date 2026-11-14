//! QA tests for TASK-M3-76 (REQ-INT-119; R-1008's convergence criterion): a rung whose error against the reference run
//! is not finite fails the convergence arm, whatever the finite rungs beside it would say. The arm passes only on an
//! order read within the margin; a non-finite error reads no order, so it is neither a pass, nor round-off (too few
//! rungs above the floor), nor a landing defect. Written from the requirement, independently of `az_diagnose_rungs`:
//! these cases put the non-finite rung beside rungs that, alone, would read round-off or a landing defect, and check
//! that finite extremes (an error of 0, of `f64::MAX`) are not taken for non-finite.

// The negative control's variant, compiled only under `controls`, rebuilds the arm from these without the check.
#[cfg(feature = "controls")]
use validation::az::{above_floor, diagnose, fitted_order};
use validation::az::{diagnose_rungs, Diagnosis, FloorCriterion};
use validation::negative_control;

/// A diagnosis of the ladder's rungs `(η, error)` against a floor, under a criterion, an expected order and a margin.
type Diagnoser = fn(&[(f64, f64)], f64, &FloorCriterion, f64, f64) -> Diagnosis;

fn fc() -> FloorCriterion {
    FloorCriterion {
        stated: 1e-14,
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

fn check(diag: Diagnoser) {
    let fc = fc();
    let floor = fc.stated;
    let d = |rungs: &[(f64, f64)]| diag(rungs, floor, &fc, 4.0, 0.5);

    // The baselines the non-finite rung is set beside, so each case below is a change of verdict, not a coincidence.
    let fourth = ladder(4, 1.0);
    assert!(
        matches!(d(&fourth), Diagnosis::Within { .. }),
        "a clean fourth-order ladder reads within: {:?}",
        d(&fourth)
    );
    let second = ladder(2, 1.0);
    assert!(
        matches!(d(&second), Diagnosis::LandingDefect { .. }),
        "a second-order ladder reads a landing defect: {:?}",
        d(&second)
    );
    // Every rung's error at the floor, below factor × floor: round-off.
    let low: Vec<(f64, f64)> = fourth.iter().map(|&(h, _)| (h, 1e-14)).collect();
    assert!(
        matches!(d(&low), Diagnosis::RoundOff { .. }),
        "a ladder at the floor reads round-off: {:?}",
        d(&low)
    );

    // A NaN rung beside a ladder that alone would read round-off: the arm fails as non-finite, not as round-off.
    let mut low_nan = low.clone();
    low_nan[0].1 = f64::NAN;
    assert_eq!(
        d(&low_nan),
        Diagnosis::NonFinite { rungs: 1 },
        "a NaN rung beside rungs at the floor is non-finite, not round-off"
    );

    // A NaN rung beside a second-order ladder: non-finite, not a landing defect.
    let mut second_nan = second.clone();
    second_nan[2].1 = f64::NAN;
    assert_eq!(
        d(&second_nan),
        Diagnosis::NonFinite { rungs: 1 },
        "a NaN rung beside a second-order ladder is non-finite, not a landing defect"
    );

    // The finest rung infinite beside a clean fourth-order ladder: still fails, never within.
    let mut fourth_inf = fourth.clone();
    fourth_inf[4].1 = f64::INFINITY;
    let v = d(&fourth_inf);
    assert_eq!(
        v,
        Diagnosis::NonFinite { rungs: 1 },
        "an infinite rung beside a clean ladder fails the arm"
    );
    assert!(!matches!(v, Diagnosis::Within { .. }), "never a pass");

    // Every rung non-finite: each counted.
    let all: Vec<(f64, f64)> = fourth.iter().map(|&(h, _)| (h, f64::NAN)).collect();
    assert_eq!(
        d(&all),
        Diagnosis::NonFinite { rungs: 5 },
        "every rung non-finite, all five counted"
    );

    // Finite extremes are not non-finite: an error of 0 is below the floor (round-off where too few remain), and
    // f64::MAX is finite, read by the fit.
    let mut zeroes = low.clone();
    zeroes[1].1 = 0.0;
    assert!(
        matches!(d(&zeroes), Diagnosis::RoundOff { .. }),
        "an error of 0 is finite, below the floor: {:?}",
        d(&zeroes)
    );
    let mut huge = fourth.clone();
    huge[0].1 = f64::MAX;
    assert!(
        !matches!(d(&huge), Diagnosis::NonFinite { .. }),
        "f64::MAX is finite: {:?}",
        d(&huge)
    );
}

#[test]
fn qa_nonfinite_rung_fails_the_arm() {
    check(diagnose_rungs);
}

negative_control!(
    qa_nonfinite_rung_fails_the_arm,
    "the floor's selection with no finiteness check: a NaN rung is dropped and the rest read round-off",
    expected = "a NaN rung beside rungs at the floor is non-finite, not round-off",
    check(|rungs, floor, fc, expected, margin| {
        let kept = above_floor(rungs, floor, fc.factor);
        diagnose(fitted_order(&kept), kept.len(), fc, expected, margin)
    })
);
