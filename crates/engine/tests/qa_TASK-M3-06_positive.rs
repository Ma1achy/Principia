//! QA's tests for TASK-M3-06, fix round 2 (physics review of PR #348, F3 and F4), each with its negative control
//! (R-176): `tau`, `eps_E` and `eps_L` must be finite and positive. Part 7's escape test `|Δn̂| < tau` is strict, so
//! at `tau = 0` escape never fires; `eps_E` and `eps_L` are the floors under the relative drifts'
//! denominators, `δE = ΔE / max(ε_E, |E₀|)` (dd_integrator §3.5), so a floor not positive gives `0/0` at `E₀ = 0`.
//!
//! Each of `+0`, `-0` and a negative value (the least subnormal included) is refused through `dispatch` itself, which
//! launches nothing and names the value, at the default horizon and the longest accepted one; the least positive
//! value is accepted and launches once. `r_coll = 0` and `r_close = 0` stay accepted (Part 7's "refuses to
//! regularise"; a length, not a threshold): the new refusal does not spill onto them.

use std::cell::Cell;

use engine::contract::sim_uniforms::SimUniforms;
use engine::dispatch::validate::{dispatch, DispatchError};
use validation::negative_control;

/// A dispatch under test, counting its launches.
type CountedDispatch = dyn Fn(&SimUniforms, &Cell<u32>) -> Result<(), DispatchError>;

/// `dispatch`, counting its launches.
fn counted(u: &SimUniforms, launched: &Cell<u32>) -> Result<(), DispatchError> {
    dispatch(u, |_| launched.set(launched.get() + 1))
}

/// The default sim key at the horizon `t`.
fn at(t: f64) -> SimUniforms {
    SimUniforms::default().with_horizon(t)
}

type Set = fn(SimUniforms, f64) -> SimUniforms;

/// The settling threshold and the two drift floors, each with its name as a refusal gives it.
const POSITIVE: [(&str, Set); 3] = [
    ("tau", |u, x| SimUniforms { tau: x, ..u }),
    ("eps_E", |u, x| SimUniforms { eps_e: x, ..u }),
    ("eps_L", |u, x| SimUniforms { eps_l: x, ..u }),
];

/// `u` dispatched: refused, nothing launched, the reason naming `name`.
fn assert_refused(rule: &CountedDispatch, u: &SimUniforms, name: &str, x: f64) {
    let launched = Cell::new(0);
    let got = rule(u, &launched);
    assert!(
        got.is_err() && launched.get() == 0,
        "{name} = {x:?} at T = {} is dispatched {} time(s): {got:?}",
        u.t_horizon,
        launched.get()
    );
    let reason = got.unwrap_err().to_string();
    assert!(
        reason.contains(name),
        "{name} = {x:?}'s refusal does not name it: {reason}"
    );
}

/// `u` dispatched: accepted and launched exactly once.
fn assert_launches_once(rule: &CountedDispatch, u: &SimUniforms, what: &str) {
    let launched = Cell::new(0);
    let got = rule(u, &launched);
    assert!(
        got.is_ok() && launched.get() == 1,
        "{what} at T = {} is not launched once ({} launch(es)): {got:?}",
        u.t_horizon,
        launched.get()
    );
}

/// `tau`, `eps_E` and `eps_L` at `+0`, `-0` and negative values are refused and launch nothing; the least positive
/// value and a large one are accepted.
fn check_zero_and_negative_refused(rule: &CountedDispatch) {
    let refused = [0.0, -0.0, -f64::from_bits(1), -1e-6, -1.0];
    let accepted = [f64::from_bits(1), f64::MIN_POSITIVE, 1e-6, 1.0];
    for base in [SimUniforms::default(), at(200.0)] {
        for (name, set) in POSITIVE {
            for x in refused {
                assert_refused(rule, &set(base, x), name, x);
            }
            for x in accepted {
                assert_launches_once(rule, &set(base, x), &format!("{name} = {x:?}"));
            }
        }
    }
}

#[test]
fn qa_positive_thresholds_zero_and_negative_refused() {
    check_zero_and_negative_refused(&counted);
}

negative_control!(
    qa_positive_thresholds_zero_and_negative_refused,
    "a dispatch that refuses only non-finite tau, eps_E and eps_L launches tau = 0, escape silently off",
    expected = "tau = 0.0 at T = 50 is dispatched 1 time(s)",
    check_zero_and_negative_refused(&|u, launched| {
        let d = SimUniforms::default();
        // The rule before F3 and F4: a finite value, however small or signed, passes.
        let fix = |x: f64, def: f64| if x.is_finite() { def } else { x };
        counted(
            &SimUniforms {
                tau: fix(u.tau, d.tau),
                eps_e: fix(u.eps_e, d.eps_e),
                eps_l: fix(u.eps_l, d.eps_l),
                ..*u
            },
            launched,
        )
    })
);

negative_control!(
    qa_positive_thresholds_signed_zero_floor,
    "a dispatch that refuses eps_E = -0 but launches eps_E = +0, a 0/0 drift at E0 = 0",
    expected = "eps_E = 0.0 at T = 50 is dispatched 1 time(s)",
    check_zero_and_negative_refused(&|u, launched| {
        let d = SimUniforms::default();
        let eps_e = if u.eps_e.to_bits() == 0 {
            d.eps_e
        } else {
            u.eps_e
        };
        counted(&SimUniforms { eps_e, ..*u }, launched)
    })
);

/// `r_coll = 0` and `r_close = 0` (±0) stay accepted and launch once: the positivity rule is the threshold's and the
/// floors', not the lengths'.
fn check_lengths_zero_accepted(rule: &CountedDispatch) {
    let lengths: [(&str, Set); 2] = [
        ("r_coll", |u, x| SimUniforms { r_coll: x, ..u }),
        ("r_close", |u, x| SimUniforms { r_close: x, ..u }),
    ];
    for base in [SimUniforms::default(), at(200.0)] {
        for (name, set) in lengths {
            for x in [0.0, -0.0] {
                assert_launches_once(rule, &set(base, x), &format!("{name} = {x:?}"));
            }
        }
    }
}

#[test]
fn qa_lengths_zero_still_accepted() {
    check_lengths_zero_accepted(&counted);
}

negative_control!(
    qa_lengths_zero_still_accepted,
    "a dispatch that applies the positivity rule to r_coll refuses Part 7's r_coll = 0",
    expected = "r_coll = 0.0 at T = 50 is not launched once",
    check_lengths_zero_accepted(&|u, launched| {
        if u.r_coll == 0.0 {
            counted(&SimUniforms { tau: 0.0, ..*u }, launched)
        } else {
            counted(u, launched)
        }
    })
);
