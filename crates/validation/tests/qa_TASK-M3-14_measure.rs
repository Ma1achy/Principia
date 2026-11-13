//! qa for TASK-M3-14: the calibration measurement drops a sample whose shadow sum went non-finite.
//! - `qa_m314_measure_coincident_shadow` (REQ-INT-039, REQ-PAY-032; R-972 (a)): `validation::measure::benettin::measure`
//!   on a healthy figure-eight run whose shadow starts on the sample (`‖δ₀‖` far below half an ulp of the state, so
//!   the first renormalisation takes `ln 0`): the sum is non-finite, and `λ_T` reads NaN, at f32 and f64 in the
//!   absolute form; the same run at `‖δ₀‖ = 10⁻³` reads finite. Control: a read that keeps the sample, reading a
//!   non-finite sum as exponent 0, fails.
//!   *Changed for TASK-M3-72 (R-974; physics N6 on PR #493; applied per R-369):* the displacement form is no longer a
//!   coincident shadow at that `‖δ₀‖`: TASK-M3-14's stand-in formed `x + δ⃗` and lost the offset, while R-974's form
//!   holds `δ⃗₀ = ‖δ₀‖·u` exactly (dd_integrator §3.8, "Start"). Its healthy read at `‖δ₀‖ = 10⁻³` is asserted finite.
//!   *Was:* "and at f32 in the displacement form" among the non-finite runs.
//!   *Changed again for TASK-M3-72 (physics review of PR #517; §3.8, "(a1)'s precision floor"; applied per R-369):* the
//!   f32 displacement arm at `‖δ₀‖ = 10⁻²⁰` is dropped. Below (a1)'s floor the force difference is exactly 0 and the
//!   shadow drifts force-free, so its finite sum measures no exponent; the floor itself is tested in
//!   `crates/kernel/tests/qa_TASK-M3-72_floor.rs`. *Was:* the f32 displacement form's sum and `λ_T` asserted finite at
//!   both `‖δ₀‖ = 10⁻²⁰` and `10⁻³`.

use validation::measure::benettin::{figure_eight, measure, Ftle, Orbit};
use validation::negative_control;

use kernel::driver::benettin::{Absolute, Displacement};
use kernel::occupant::yoshida4::Yoshida4;

/// A read of `λ_T` from one measurement: the measurement's own, or a stand-in.
type Read = fn(&Ftle) -> f64;

/// Each absolute-form precision's measurement on `orbit` at `delta_0`, renormalised every macro-step.
fn runs(orbit: &Orbit, delta_0_f32: f64, delta_0_f64: f64) -> [(&'static str, Ftle); 2] {
    [
        (
            "f32 absolute",
            measure::<Yoshida4, f32, Absolute>(orbit, delta_0_f32, 1),
        ),
        (
            "f64 absolute",
            measure::<Yoshida4, f64, Absolute>(orbit, delta_0_f64, 1),
        ),
    ]
}

/// The f32 displacement form's measurement on `orbit` at `delta_0`, renormalised every macro-step.
fn displacement(orbit: &Orbit, delta_0: f64) -> Ftle {
    measure::<Yoshida4, f32, Displacement>(orbit, delta_0, 1)
}

fn check_coincident(read: Read) {
    let orbit = figure_eight(0.01);
    // ‖δ₀‖ far below half an ulp of the figure-eight's coordinates (order 0.1 to 1): 1e-20 at f32 (ulp(0.1) ≈ 7.5e-9),
    // 1e-40 at f64 (ulp(0.1) ≈ 1.4e-17).
    for (name, f) in runs(&orbit, 1e-20, 1e-40) {
        assert!(f.steps > 1, "fixture ({name}): {} macro-steps", f.steps);
        assert!(
            !f.s_sum.is_finite(),
            "fixture ({name}): the coincident shadow's sum {} is finite",
            f.s_sum
        );
        let l = read(&f);
        assert!(l.is_nan(), "{name}: a non-finite sum reads {l}, not NaN");
    }
    for (name, f) in runs(&orbit, 1e-3, 1e-3) {
        assert!(
            f.s_sum.is_finite(),
            "fixture ({name}): the healthy shadow's sum {} is not finite",
            f.s_sum
        );
        let l = read(&f);
        assert!(l.is_finite(), "{name}: a healthy run reads {l}");
    }
    // The displacement form's healthy read (R-974); below (a1)'s floor its finite sum measures nothing (§3.8).
    let f = displacement(&orbit, 1e-3);
    assert!(
        f.steps > 1,
        "fixture (f32 displacement): {} macro-steps",
        f.steps
    );
    assert!(
        f.s_sum.is_finite(),
        "fixture (f32 displacement, ‖δ₀‖ = 1e-3): the shadow's sum {} is not finite",
        f.s_sum
    );
    let l = read(&f);
    assert!(
        l.is_finite(),
        "f32 displacement, ‖δ₀‖ = 1e-3: a finite sum reads {l}"
    );
}

#[test]
fn qa_m314_measure_coincident_shadow() {
    check_coincident(|f| f.lambda);
}

negative_control!(
    qa_m314_measure_coincident_shadow,
    "a read that keeps a non-finite-sum sample as exponent 0 must fail",
    expected = "not NaN",
    check_coincident(|f| if f.s_sum.is_finite() { f.lambda } else { 0.0 })
);
