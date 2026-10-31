//! QA's tests for TASK-M3-06: `dt_macro` at the f32 kernel, each with its negative control (R-176). The dispatch
//! requires `dt_macro` finite and positive, and `wrapper_uniforms_f32` hands the kernel that value rounded once to
//! f32: REQ-PAY-040's exact step indices and REQ-INT-081's `horizon_steps` mean nothing if the step the kernel takes
//! is infinite or 0. So what dispatch accepts is exactly what stays a time step after the rounding, wherever the
//! rounding's edges fall, which is not at `f32::MAX` and the least subnormal themselves: round-to-nearest carries
//! values past them, to `f32::MAX + ½ulp` (a tie, going to infinity) and `2⁻¹⁵⁰` (a tie, going to 0).
//!
//! The horizon and window are chosen so the step count is 1 and neither is what refuses: the default ones for a
//! large step, the step's own size for a small one.

use engine::contract::sim_uniforms::SimUniforms;
use engine::dispatch::validate::{validate, DispatchError};
use validation::negative_control;

/// A dispatch under test: what it hands the f32 kernel, or its refusal.
type Rule = dyn Fn(&SimUniforms) -> Result<f32, DispatchError>;

/// The real dispatch.
fn real(u: &SimUniforms) -> Result<f32, DispatchError> {
    validate(u).map(|v| v.wrapper_uniforms_f32().dt_macro)
}

/// The default sim key at step `dt`, with one macro-step to the horizon.
fn one_step(dt: f64) -> SimUniforms {
    let d = SimUniforms::default();
    if dt >= d.t_horizon {
        SimUniforms { dt_macro: dt, ..d }
    } else {
        SimUniforms {
            dt_macro: dt,
            t_horizon: dt,
            escape_window: dt,
            ..d
        }
    }
}

/// The largest f64 below `x`, for `x` finite and positive.
fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

/// The smallest f64 above `x`, for `x` finite and positive.
fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

/// `dt` is refused as a `dt_macro`.
fn assert_refused(rule: &Rule, dt: f64, side: &str) {
    let got = rule(&one_step(dt));
    assert!(
        matches!(got, Err(DispatchError::DtMacro { .. })),
        "{side}: dt_macro {dt:e} is not refused: {got:?}"
    );
}

/// `dt` is accepted, and the kernel is handed `want`.
fn assert_handed(rule: &Rule, dt: f64, want: f32) {
    let got = rule(&one_step(dt));
    assert_eq!(got, Ok(want), "dt_macro {dt:e}");
}

/// The large edge: past `f32::MAX + ½ulp` the kernel's step is infinite.
fn check_large_edge(rule: &Rule) {
    let tie = f64::from(f32::MAX) + 2f64.powi(103);
    assert_handed(rule, f64::from(f32::MAX), f32::MAX);
    assert_handed(rule, below(tie), f32::MAX);
    assert_refused(rule, tie, "large");
    assert_refused(rule, above(tie), "large");
    assert_refused(rule, 2f64.powi(128), "large");
    assert_refused(rule, f64::MAX, "large");
}

/// The small edge: at or under `2⁻¹⁵⁰` the kernel's step is 0.
fn check_small_edge(rule: &Rule) {
    let tie = 2f64.powi(-150);
    assert_handed(rule, 2f64.powi(-149), f32::from_bits(1));
    assert_handed(rule, above(tie), f32::from_bits(1));
    assert_refused(rule, tie, "small");
    assert_refused(rule, below(tie), "small");
    assert_refused(rule, 1e-300, "small");
    assert_refused(rule, f64::from_bits(1), "small");
}

/// Over every binary exponent of f64, subnormals included, and three mantissas each: dispatch accepts exactly the
/// values that are still a time step at f32, and what it hands the kernel is that value rounded once.
fn check_every_exponent(rule: &Rule) {
    let mut accepted = 0u32;
    let mut refused = 0u32;
    let values = (0..=2046u64)
        .flat_map(|exp| [0u64, 1, (1 << 52) - 1].map(move |m| f64::from_bits((exp << 52) | m)));
    for dt in values.filter(|&dt| dt > 0.0) {
        let rounded = dt as f32;
        if rounded.is_finite() && rounded > 0.0 {
            assert_handed(rule, dt, rounded);
            accepted += 1;
        } else {
            assert_refused(rule, dt, "sweep");
            refused += 1;
        }
    }
    // The sweep reaches both sides of the rounding, so neither arm is vacuous.
    assert!(
        accepted > 500 && refused > 3000,
        "the sweep is thin: {accepted} accepted, {refused} refused"
    );
}

#[test]
fn qa_dt_macro_f32_edges() {
    check_large_edge(&real);
    check_small_edge(&real);
}

/// A rule refusing only what fails at f64, then handing on the rounding: the controls' rule.
#[cfg(feature = "controls")]
fn f64_only(u: &SimUniforms) -> Result<f32, DispatchError> {
    if !(u.dt_macro.is_finite() && u.dt_macro > 0.0) {
        return Err(DispatchError::DtMacro {
            dt_macro: u.dt_macro,
        });
    }
    Ok(u.dt_macro as f32)
}

negative_control!(
    qa_dt_macro_f32_edges,
    "a dispatch checking dt_macro at f64 only hands the kernel an infinite step past the tie",
    expected = "large: dt_macro",
    check_large_edge(&f64_only)
);

mod refuses_at_the_written_limit {
    use super::*;
    negative_control!(
        qa_dt_macro_f32_edges,
        "a dispatch refusing above f32::MAX itself refuses values that round to f32::MAX, a step the kernel can take",
        expected = "dt_macro",
        check_large_edge(&|u: &SimUniforms| {
            if u.dt_macro > f64::from(f32::MAX) {
                return Err(DispatchError::DtMacro {
                    dt_macro: u.dt_macro,
                });
            }
            f64_only(u)
        })
    );
}

mod only_the_large_edge {
    use super::*;
    negative_control!(
        qa_dt_macro_f32_edges,
        "a dispatch guarding infinity alone hands the kernel a step of 0, where no time passes",
        expected = "small: dt_macro",
        check_small_edge(&|u: &SimUniforms| {
            if (u.dt_macro as f32).is_infinite() {
                return Err(DispatchError::DtMacro {
                    dt_macro: u.dt_macro,
                });
            }
            f64_only(u)
        })
    );
}

#[test]
fn qa_dt_macro_f32_every_exponent() {
    check_every_exponent(&real);
}

negative_control!(
    qa_dt_macro_f32_every_exponent,
    "a dispatch checking dt_macro at f64 only accepts the first value the kernel cannot step by",
    expected = "sweep: dt_macro",
    check_every_exponent(&f64_only)
);

/// An ordinary key's step is not disturbed by the check: the default's `10⁻³`, and `T/65535` at the cap.
fn check_ordinary(rule: &Rule) {
    let d = SimUniforms::default();
    assert_eq!(rule(&d), Ok(1e-3f64 as f32), "the default key");
    let cap = d.with_horizon(200.0);
    assert_eq!(rule(&cap), Ok((200.0f64 / 65535.0) as f32), "T = 200");
}

#[test]
fn qa_dt_macro_f32_ordinary_keys_unchanged() {
    check_ordinary(&real);
}

negative_control!(
    qa_dt_macro_f32_ordinary_keys_unchanged,
    "a dispatch refusing every step below f32's normal range's top would refuse the default key",
    expected = "the default key",
    check_ordinary(&|u: &SimUniforms| Err(DispatchError::DtMacro {
        dt_macro: u.dt_macro
    }))
);
