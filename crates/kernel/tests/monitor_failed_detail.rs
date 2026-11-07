//! The monitor's failure mark at its edges (`kernel::driver::{monitor::latched, wrapper::monitored}`; payload §2,
//! `sim_failed`'s `detail`; TASK-M3-05):
//! - `invariant_monitor_failed_detail_walks_every_component`: a failed run's detail reads each of `r`'s and `p`'s twelve
//!   components, and the walk finishes, within a bound of its own: a loop that stops advancing fails this test where
//!   it would otherwise run to the mutants job's timeout.
//! - `invariant_monitor_latch_keeps_its_value_at_a_tie`: a tie keeps the held maximum, on its bits.

use std::sync::mpsc;
use std::time::Duration;

use kernel::decode::Ic;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::monitor::latched;
use kernel::driver::wrapper::monitored;
use kernel::occupant::kdk::Kdk;
use kernel::payload::{sd_detail, SimStateBase};
use validation::drift::{integrate_fixture, Fixture};
use validation::negative_control;

/// How long the walk may take: it is a short march and some bit tests, so this is only ever reached by a loop that never
/// ends. nextest's three runs of a failing test (`.cargo/mutants.toml`) stay inside the per-mutant timeout's 60 s floor.
const WALK_BOUND: Duration = Duration::from_secs(15);

/// Runs `f` on a thread of its own and panics, naming the bound, if it is still running after `bound`. A panic in `f`
/// is `f`'s own. A `f` that never ends leaves its thread behind, which the test process's exit ends.
fn within(bound: Duration, f: impl FnOnce() + Send + 'static) {
    let (done, finished) = mpsc::channel();
    let walk = std::thread::spawn(move || {
        f();
        let _ = done.send(());
    });
    if let Err(mpsc::RecvTimeoutError::Timeout) = finished.recv_timeout(bound) {
        panic!("did not finish within {bound:?}");
    }
    if let Err(payload) = walk.join() {
        std::panic::resume_unwind(payload);
    }
}

/// A finite run whose energy is non-finite at f32: momenta whose squares overflow, so the monitor latches a NaN `ΔE`.
fn failed_run() -> Fixture {
    Fixture {
        name: "momenta whose squares overflow at f32",
        ic: Ic {
            m: [0.25, 0.25, 0.5],
            r: [[-0.5, -0.25], [0.5, -0.25], [0.0, 0.25]],
            p: [[3e19, 0.0], [-3e19, 0.0], [0.0, 0.0]],
        },
        horizon: 0.002,
    }
}

/// `mark`'s detail with each component of `r`, then of `p`, taken non-finite alone: 0 for a NaN, 1 for a `±∞`, and for
/// a finite state 2; and 0 for a NaN beside a `±∞`, whichever comes first in the walk.
fn check_walk(mark: fn(SimStateBase, &WrapperCadence<f32>) -> SimStateBase) {
    let (out, cadence) = integrate_fixture::<Kdk, f32>(&failed_run());
    let detail = |o: SimStateBase| sd_detail(mark(o, &cadence).packed_a);
    assert_eq!(detail(out), 2, "a finite state");
    for i in 0..3 {
        for j in 0..2 {
            for (name, v, want) in [
                ("+inf", f32::INFINITY, 1),
                ("-inf", f32::NEG_INFINITY, 1),
                ("NaN", f32::NAN, 0),
            ] {
                let mut in_r = out;
                in_r.r[i][j] = v;
                assert_eq!(detail(in_r), want, "{name} in r[{i}][{j}]");
                let mut in_p = out;
                in_p.p[i][j] = v;
                assert_eq!(detail(in_p), want, "{name} in p[{i}][{j}]");
            }
        }
    }
    let mut nan_first = out;
    nan_first.r[0][0] = f32::NAN;
    nan_first.p[2][1] = f32::INFINITY;
    assert_eq!(detail(nan_first), 0, "a NaN before an `∞`");
    let mut inf_first = out;
    inf_first.r[0][0] = f32::INFINITY;
    inf_first.p[2][1] = f32::NAN;
    assert_eq!(detail(inf_first), 0, "a NaN after an `∞`");
}

#[test]
fn invariant_monitor_failed_detail_walks_every_component() {
    within(WALK_BOUND, || check_walk(monitored));
}

negative_control!(
    invariant_monitor_failed_detail_walks_every_component,
    "a walk that never reads `p`",
    expected = "+inf in p[0][0]",
    within(WALK_BOUND, || check_walk(|mut o, c| {
        o.p = [[0.0; 2]; 3];
        monitored(o, c)
    }))
);

/// `within` lets a walk that ends finish.
#[test]
fn invariant_monitor_walk_bound_passes_a_walk_that_ends() {
    within(WALK_BOUND, || ());
}

negative_control!(
    invariant_monitor_walk_bound_passes_a_walk_that_ends,
    "a walk that never ends fails in the bound's time, not the timeout's",
    expected = "did not finish within",
    within(Duration::from_millis(50), || loop {
        std::thread::park();
    })
);

/// The latch at a tie: the held maximum stays, told from the new value by the sign of zero, the one tie with two
/// values, and on either sign.
fn check_tie(latch: fn(f64, f64) -> f64) {
    assert_eq!(
        latch(0.0, -0.0).to_bits(),
        0.0_f64.to_bits(),
        "a tie replaced the held +0"
    );
    assert_eq!(
        latch(-0.0, 0.0).to_bits(),
        (-0.0_f64).to_bits(),
        "a tie replaced the held -0"
    );
}

#[test]
fn invariant_monitor_latch_keeps_its_value_at_a_tie() {
    check_tie(latched);
}

negative_control!(
    invariant_monitor_latch_keeps_its_value_at_a_tie,
    "a latch that takes the new value at a tie",
    expected = "a tie replaced the held +0",
    check_tie(|a, b| if b >= a { b } else { a })
);
