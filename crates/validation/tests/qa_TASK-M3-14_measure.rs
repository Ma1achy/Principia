//! qa for TASK-M3-14: the calibration measurement drops a sample whose shadow sum went non-finite.
//! - `qa_m314_measure_coincident_shadow` (REQ-INT-039, REQ-PAY-032; R-972 (a)): `validation::measure::benettin::measure`
//!   on a healthy figure-eight run whose shadow starts on the sample (`‖δ₀‖` far below half an ulp of the state, so
//!   the first renormalisation takes `ln 0`): the sum is non-finite, and `λ_T` reads NaN, at f32 and f64 in the
//!   absolute form and at f32 in the displacement form; the same run at `‖δ₀‖ = 10⁻³` reads finite. Control: a read
//!   that keeps the sample, reading a non-finite sum as exponent 0, fails.

use validation::measure::benettin::{figure_eight, measure, Ftle, Orbit};
use validation::negative_control;

use kernel::driver::benettin::{Absolute, Displacement};
use kernel::occupant::yoshida4::Yoshida4;

/// A read of `λ_T` from one measurement: the measurement's own, or a stand-in.
type Read = fn(&Ftle) -> f64;

/// Each form and precision's measurement on `orbit` at `delta_0`, renormalised every macro-step.
fn runs(orbit: &Orbit, delta_0_f32: f64, delta_0_f64: f64) -> [(&'static str, Ftle); 3] {
    [
        (
            "f32 absolute",
            measure::<Yoshida4, f32, Absolute>(orbit, delta_0_f32, 1),
        ),
        (
            "f64 absolute",
            measure::<Yoshida4, f64, Absolute>(orbit, delta_0_f64, 1),
        ),
        (
            "f32 displacement",
            measure::<Yoshida4, f32, Displacement>(orbit, delta_0_f32, 1),
        ),
    ]
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
