//! BG-4, the butterflies' periods by closure (`validation::gate::butterfly_periods`). Each test registers the control
//! that must make it fail.
//!
//! - `butterfly_periods_known`: the method finds the period of the atlas's refined figure-eight, of the same rounded
//!   to six decimals (with its input box), and of the atlas's two refined butterflies, each within half its own
//!   uncertainty. Control: each period handed off by that uncertainty, above and below, is rejected.
//! - `butterfly_periods_decide`: the nearer printed period is read only when the uncertainty undercuts their
//!   difference. Control: an uncertainty half the difference, which is read.
//! - `butterfly_periods_scored`: the target scores through the replication command, its control fired, and prints its
//!   measurement. Control: a measurement sitting on the period plus its uncertainty, whose control cannot fire.
//!
//! **Pass criteria**, written before the run: `|t − T| < u/2` on each known orbit, `u` every term of the module's
//! note; `T ± u` rejected on each; the decision undecided at `u ≥ |T_a − T_b|`. The comparison with the printed
//! periods is a measurement and is only printed.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use validation::gate::butterfly_periods::{
    check_known, control_fires, decide, known, Decision, Known, Measured, BG4, ID,
};
use validation::gate::replication::command;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The known orbits, measured once per process.
fn measured() -> &'static [(Known, Measured)] {
    static KNOWN: OnceLock<Vec<(Known, Measured)>> = OnceLock::new();
    KNOWN.get_or_init(|| known(&root()).expect("the known orbits measure"))
}

// ── The method on known periods ──────────────────────────────────────────────────────────────────────────────────

/// The check on every known orbit, each period moved by `shift` times its measurement's uncertainty.
fn check_shifted(shift: f64) {
    for (k, m) in measured() {
        check_known(&k.start.name, m, k.period + shift * m.u());
    }
}

#[test]
fn butterfly_periods_known() {
    for (k, m) in measured() {
        println!(
            "{}: t = {:.12}, T = {:.12}, |t − T| = {:.2e}, u/2 = {:.2e}; {}",
            k.start.name,
            m.t,
            k.period,
            (m.t - k.period).abs(),
            m.u() / 2.0,
            m.terms()
        );
    }
    check_shifted(0.0);
}

/// Every known orbit's period off by its uncertainty, below and above, each rejected: the last one's rejection is the
/// control's panic, every other one's is asserted.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn known_control() {
    let n = measured().len();
    for (i, (k, m)) in measured().iter().enumerate() {
        let name = &k.start.name;
        for shift in [-1.0, 1.0] {
            let period = k.period + shift * m.u();
            if i + 1 == n && shift > 0.0 {
                check_known(name, m, period);
            } else {
                let rejected = std::panic::catch_unwind(|| check_known(name, m, period)).is_err();
                assert!(rejected, "{name}: the period {shift:+} u was not rejected");
            }
        }
    }
}

negative_control!(
    butterfly_periods_known,
    "each known period off by the method's uncertainty must be rejected",
    expected = "the method missed the known period",
    known_control()
);

// ── The decision ─────────────────────────────────────────────────────────────────────────────────────────────────

const PERIODS: [f64; 2] = [6.235641, 6.234671];

fn check_undecided(u: f64) {
    assert_eq!(
        decide(6.234671, u, PERIODS),
        Decision::Undecided,
        "read which period is nearer with an uncertainty {u:e} that does not undercut their difference"
    );
}

#[test]
fn butterfly_periods_decide() {
    let gap = (PERIODS[0] - PERIODS[1]).abs();
    check_undecided(gap);
    check_undecided(2.0 * gap);
    assert_eq!(decide(6.234672, 1e-5, PERIODS), Decision::Nearer(1, true));
    assert_eq!(decide(6.235640, 1e-5, PERIODS), Decision::Nearer(0, true));
    assert_eq!(decide(6.234700, 1e-5, PERIODS), Decision::Nearer(1, false));
}

negative_control!(
    butterfly_periods_decide,
    "an uncertainty half the difference reads the nearer period",
    expected = "does not undercut their difference",
    check_undecided((PERIODS[0] - PERIODS[1]).abs() / 2.0)
);

// ── The target ───────────────────────────────────────────────────────────────────────────────────────────────────

fn check_control(m: &Measured, period: f64) {
    assert!(
        control_fires(m, period),
        "its control did not fire: the period ± u is not rejected"
    );
}

#[test]
fn butterfly_periods_scored() {
    for (k, m) in measured() {
        check_control(m, k.period);
    }
    let out = std::env::temp_dir().join(format!("bg4-{}", std::process::id()));
    // The command errs where the scoring errs or the target's control did not fire.
    let text = command(&[&BG4], &root(), &out, &["--target", ID]).expect("the command reports");
    println!("{text}");
    assert!(text.starts_with(&format!("{ID}: ")), "{text}");
}

negative_control!(
    butterfly_periods_scored,
    "a measurement on the period plus its uncertainty cannot reject the period ± u",
    expected = "its control did not fire",
    {
        let (k, m) = &measured()[0];
        check_control(&m.shifted(k.period + m.u() - m.t), k.period);
    }
);
