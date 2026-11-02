//! QA's tests for TASK-M3-06: `dt_macro` at the f32 kernel, each with its negative control (R-176). The dispatch
//! requires `dt_macro` finite and positive, and `wrapper_uniforms_f32` hands the kernel that value rounded once to
//! f32: REQ-PAY-040's exact step indices and REQ-INT-081's `horizon_steps` mean nothing if the step the kernel takes
//! is infinite or 0. So what dispatch accepts stays a time step after the rounding, wherever the rounding's edges
//! fall. The large edge is not at `f32::MAX` itself: round-to-nearest carries values past it, to `f32::MAX + ½ulp` (a
//! tie, going to infinity).
//!
//! The small edge is the substep's: the kernel steps by `dt_macro / N_sub`, smallest at `N_max`, and that value at f32
//! must be a normal f32, since a backend that flushes subnormal results reads a subnormal substep as 0 where the CPU
//! keeps it (the refusal stands by decision under R-369, recorded on the PR). So the floor moves with `N_max`:
//! `2⁻¹²⁶ · N_max`, `2⁻¹²⁰` at the default 64.
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
    one_step_at(SimUniforms::default().n_max, dt)
}

/// [`one_step`] under `n_max`, with the table the host generator produces for it.
fn one_step_at(n_max: u32, dt: f64) -> SimUniforms {
    let d = SimUniforms::default();
    let d = d
        .with_substep_law(d.r_sub, d.gamma_sub, n_max)
        .expect("n_max is within the table");
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

/// The small edge at the default `N_max = 64`. The smallest value whose exact `dt / 64` is normal is `2⁻¹²⁰`, and it
/// is handed on, as are the values rounding up to it, `2⁻¹²⁰ − 2⁻¹⁴⁵` (a tie, going to the even mantissa) among them.
/// `2⁻¹²⁰ − 2⁻¹⁴³`, two f32 ulps down, has an exact and subnormal quotient, and is refused, and so is everything under
/// it down to the former edges: `2⁻¹⁴⁹`, the least subnormal, and `2⁻¹⁵⁰`, which rounds to 0.
///
/// `2⁻¹²⁰ − 2⁻¹⁴⁴`, the one f32 between, is pinned neither way: its exact quotient is subnormal, and round-to-nearest
/// carries it up to `2⁻¹²⁶`. Which side it belongs on is reported to the orchestrator, not settled here.
fn check_small_edge(rule: &Rule) {
    let floor = 2f64.powi(-120);
    let tie = floor - 2f64.powi(-145);
    assert_handed(rule, floor, 2f32.powi(-120));
    assert_handed(rule, above(floor), 2f32.powi(-120));
    assert_handed(rule, tie, 2f32.powi(-120));
    assert_handed(rule, above(tie), 2f32.powi(-120));
    assert_refused(rule, floor - 2f64.powi(-143), "small");
    assert_refused(rule, 2f64.powi(-121), "small");
    assert_refused(rule, 2f64.powi(-126), "small");
    assert_refused(rule, 2f64.powi(-149), "small");
    assert_refused(rule, above(2f64.powi(-150)), "small");
    assert_refused(rule, 2f64.powi(-150), "small");
    assert_refused(rule, below(2f64.powi(-150)), "small");
    assert_refused(rule, 1e-300, "small");
    assert_refused(rule, f64::from_bits(1), "small");
}

/// The floor follows `N_max`, not a fixed count: `2⁻¹²⁶ · N_max` is handed on and a lower f32 refused, at `N_max` 1
/// (the substep is the macro-step), 63 (no power of two, so the quotient is a real division) and 256 (the table's
/// length), where `2⁻¹¹⁹`, admitted at 64, is under the floor. The large edge does not move with it.
fn check_substep_rule(rule: &Rule) {
    let least_normal = 2f64.powi(-126);
    let handed = |n: u32, dt: f64| {
        assert_eq!(
            rule(&one_step_at(n, dt)),
            Ok(dt as f32),
            "N_max {n}: dt_macro {dt:e} is not handed on"
        );
    };
    let refused = |n: u32, dt: f64| {
        let got = rule(&one_step_at(n, dt));
        assert!(
            matches!(got, Err(DispatchError::DtMacro { .. })),
            "N_max {n}: dt_macro {dt:e} is not refused: {got:?}"
        );
    };
    handed(1, least_normal);
    refused(1, least_normal - 2f64.powi(-149));
    refused(1, 2f64.powi(-149));
    handed(63, 63.0 * least_normal);
    handed(63, 64.0 * least_normal);
    refused(63, 62.0 * least_normal);
    handed(256, 256.0 * least_normal);
    refused(256, 255.0 * least_normal);
    refused(256, 2f64.powi(-119));
    handed(256, f64::from(f32::MAX));
}

/// Over every binary exponent of f64, subnormals included, and three mantissas each: dispatch accepts exactly the
/// values that stay finite at f32 with a normal smallest substep, and what it hands the kernel is that value rounded
/// once. None of these values is the f32 [`check_small_edge`] leaves unpinned.
fn check_every_exponent(rule: &Rule) {
    let mut accepted = 0u32;
    let mut refused = 0u32;
    let values = (0..=2046u64)
        .flat_map(|exp| [0u64, 1, (1 << 52) - 1].map(move |m| f64::from_bits((exp << 52) | m)));
    let n_max = SimUniforms::default().n_max as f32;
    for dt in values.filter(|&dt| dt > 0.0) {
        let rounded = dt as f32;
        if rounded.is_finite() && (rounded / n_max).is_normal() {
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

mod only_a_nonzero_step {
    use super::*;
    negative_control!(
        qa_dt_macro_f32_edges,
        "a dispatch refusing only a step of 0 at f32 hands the kernel a substep a flushing backend reads as 0",
        expected = "small: dt_macro",
        check_small_edge(&|u: &SimUniforms| {
            if u.dt_macro as f32 == 0.0 {
                return Err(DispatchError::DtMacro {
                    dt_macro: u.dt_macro,
                });
            }
            f64_only(u)
        })
    );
}

mod quotient_at_f64 {
    use super::*;
    negative_control!(
        qa_dt_macro_f32_edges,
        "a dispatch taking the quotient before the rounding refuses values the kernel receives as the floor",
        expected = "dt_macro",
        check_small_edge(&|u: &SimUniforms| {
            if u.dt_macro / f64::from(u.n_max) < f64::from(f32::MIN_POSITIVE) {
                return Err(DispatchError::DtMacro {
                    dt_macro: u.dt_macro,
                });
            }
            f64_only(u)
        })
    );
}

#[test]
fn qa_dt_macro_f32_smallest_substep_follows_n_max() {
    check_substep_rule(&real);
}

negative_control!(
    qa_dt_macro_f32_smallest_substep_follows_n_max,
    "a dispatch with its floor fixed at the default N_max refuses a normal substep at N_max = 1",
    expected = "N_max 1: dt_macro",
    check_substep_rule(&|u: &SimUniforms| {
        if ((u.dt_macro as f32) / 64.0).is_normal() {
            return f64_only(u);
        }
        Err(DispatchError::DtMacro {
            dt_macro: u.dt_macro,
        })
    })
);

mod macro_step_alone {
    use super::*;
    negative_control!(
        qa_dt_macro_f32_smallest_substep_follows_n_max,
        "a dispatch asking only that dt_macro be normal hands N_max = 63 a subnormal substep",
        expected = "N_max 63: dt_macro",
        check_substep_rule(&|u: &SimUniforms| {
            if (u.dt_macro as f32).is_normal() {
                return f64_only(u);
            }
            Err(DispatchError::DtMacro {
                dt_macro: u.dt_macro,
            })
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
