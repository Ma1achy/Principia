//! QA tests for TASK-M3-78's fix round (the head after qa's approval of e7beb39b), written from REQ-VAL-357,
//! REQ-VAL-170's Heggie and Aarseth–Zare arms and REQ-VAL-080, not from the implementation:
//!
//! - the finite-difference discrepancy at a state keeps a non-finite component wherever it sits, first, middle or
//!   last, never dropping it from the reduction (REQ-VAL-080: no silent skip of a non-finite sample);
//! - REQ-VAL-357's proposal states a minimum only with its evidence: over a ladder holding no sub-ladder of two rungs
//!   or more starting at the gate's `η` or finer, `radial-collision --propose` proposes no minimum at all, rather than
//!   one it measured nothing at.
//!
//! Each test registers its negative control (R-176).

use validation::decode_f32_eps::SplitMix;
use validation::heggie::{discrepancy_at, draw, propose_regions, Draw, FdOccupant, REGION_ETA};
use validation::negative_control;

/// A draw whose analytic rates, as the flat vector `(∂Γ/∂p…, −∂Γ/∂q…, dt/dτ)`, have component `at` set to `value`.
fn with_component(d: Draw, at: usize, value: f64) -> Draw {
    let Draw {
        x,
        hamiltonian,
        rates,
        differenced,
    } = d;
    Draw {
        x,
        hamiltonian,
        differenced,
        rates: Box::new(move |y| {
            let (dq, dp, dt) = rates(y);
            let n = dq.len();
            let mut all: Vec<f64> = dq.into_iter().chain(dp).chain([dt]).collect();
            all[at] = value;
            let last = all.len() - 1;
            (all[..n].to_vec(), all[n..last].to_vec(), all[last])
        }),
    }
}

/// For each occupant, a NaN put into one analytic rate, first, middle or last (`dt/dτ`), makes the discrepancy at
/// that state NaN, flipped or not: a `0.0`-seeded `f64::max` fold drops it wherever it sits. `value` is what is put
/// there (the control puts a finite, wrong rate).
fn check_nan_kept(value: f64) {
    let mut rng = SplitMix::new(0x0078_0080);
    for o in FdOccupant::ALL {
        let base = draw(o, &mut rng);
        let len = base.x.len();
        for at in [0, len / 2, len - 1] {
            let d = with_component(draw(o, &mut rng), at, value);
            for flip in [false, true] {
                let r = discrepancy_at(&d, 1e-5, flip);
                assert!(
                    r.is_nan(),
                    "{o:?}: rate {at} of {len} set to {value}, flip {flip}: the discrepancy is not NaN: {r:e}"
                );
            }
        }
    }
}

#[test]
fn qa_fd_nan_rate_kept() {
    check_nan_kept(f64::NAN);
}

negative_control!(
    qa_fd_nan_rate_kept,
    "a finite wrong rate, 1e6, put where the NaN goes",
    expected = "the discrepancy is not NaN",
    check_nan_kept(1e6)
);

/// `--propose` over `ladder` with the gate's `η` proposes no minimum: it says none holds and states no number.
fn check_no_vacuous_minimum(ladder: &[f64]) {
    let out = propose_regions(ladder, REGION_ETA);
    assert!(
        !out.contains("**Proposed:**"),
        "a minimum proposed with no sub-ladder measured at it: {out}"
    );
    assert!(out.contains("No minimum holds"), "{out}");
}

#[test]
fn qa_radial_no_vacuous_minimum() {
    // The gate's η is the finest rung: no sub-ladder of two rungs or more starts at it or finer.
    check_no_vacuous_minimum(&[0.02, REGION_ETA]);
}

negative_control!(
    qa_radial_no_vacuous_minimum,
    "the contract's seven-rung ladder, whose minimum stands on measured sub-ladders",
    expected = "a minimum proposed with no sub-ladder measured at it",
    check_no_vacuous_minimum(&validation::heggie::REGION_LADDER)
);
