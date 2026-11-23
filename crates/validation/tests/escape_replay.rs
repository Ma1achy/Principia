//! The escape rule replayed on the harness march (`validation::escape_checks::replay`; TASK-M3-34), and the legacy
//! `t = 30` truth:
//! - `escape_replay_marks_are_the_kernels`: the replay's window marks at the register's window are the kernel's
//!   `is_mark`, on the sync grid and on a macro-step grid. Control: a window of 0.41.
//! - `escape_replay_is_the_detector`: on a `config stability` pixel that fires and on the Burrau anchor's march, the
//!   replay at the register's rule fires where the detector fired, with its escaper and `detail`. Control: the replay
//!   at another window.
//! - `escape_legacy_truth_takes_the_pair_mass`: prin-rs's legacy test reads the pair's mass alone: a body unbound by
//!   it and bound by the total mass is an escaper of the legacy truth. Control: the total mass in its place.
//! - `escape_burrau_label_band`: the Burrau label's firing time against `tau`, as the PR reports it: 157.6 of S&P's
//!   units at `1e-3`, in `[50, 100]` at `3e-3`, naming the lightest body. Control: `2e-3`, which fires at 123.
//!
//! **Domain**: f64, Heggie × RK4 as the field backend marches it; the population measurements are the gates'
//! (`cargo xtask gate tau-gap`, `escape-revalidation`, `escape-precision`, `escape-window-ladder`).

use validation::escape_checks::march::{self, Snapshot};
use validation::escape_checks::replay::{self, Rule};
use validation::escape_checks::revalidation::{self, burrau_firing, Legacy};
use validation::escape_checks::{config_stability, GroundTruthOccupant};
use validation::ground_truth::tiers::BURRAU_WINDOW;
use validation::negative_control;

fn check_marks(window: f64) {
    for quantum in [0.125, 1e-3, 0.0015808469148463273] {
        for index in 0..20_000u32 {
            assert_eq!(
                replay::is_mark(index, quantum, window),
                kernel::detect::escape::is_mark(index, quantum),
                "the replay's mark at index {index} of a grid of {quantum} is not the kernel's"
            );
        }
    }
}

#[test]
fn escape_replay_marks_are_the_kernels() {
    check_marks(Rule::register().window);
}

negative_control!(
    escape_replay_marks_are_the_kernels,
    "a window of 0.41 marks other sampling points",
    expected = "is not the kernel's",
    check_marks(0.41)
);

fn check_detector(window: f64) {
    let pop = config_stability(16, 50.0);
    let (ic, key) = (pop.ics[104], pop.keys[104]);
    let run = march::march_past::<GroundTruthOccupant>(&ic, &key, 60_000);
    let (bic, brun, _) = revalidation::burrau();
    for (name, ic, run) in [("config pixel 104", &ic, &run), ("Burrau", bic, brun)] {
        let own = run.firing.expect("the march fires").index;
        let again = replay::replay(
            run,
            ic,
            Rule {
                window,
                ..Rule::register()
            },
        )
        .map(|f| f.index);
        println!("{name}: the detector fired at index {own}, the replay at {again:?}");
        replay::reproduces(run, ic).expect("the replay at the register's rule");
        assert_eq!(
            again,
            Some(own),
            "{name}: the replay does not fire where the detector did"
        );
    }
}

#[test]
fn escape_replay_is_the_detector() {
    check_detector(Rule::register().window);
}

negative_control!(
    escape_replay_is_the_detector,
    "the replay at another window",
    expected = "the replay does not fire where the detector did",
    check_detector(0.8)
);

/// Body 2 at distance 10 from the other two (unit masses at ±0.5), receding at speed `v`: unbound by the pair's mass
/// 2 for `½v² > 2/10`, by the total mass 3 only for `½v² > 3/10`. `v = 0.7` lies between.
fn between() -> (Snapshot, [f64; 3]) {
    let m = [1.0, 1.0, 1.0];
    (
        Snapshot {
            t: 30.0,
            r: [[-0.5, 0.0], [0.5, 0.0], [0.0, 10.0]],
            p: [[0.0, 0.0], [0.0, 0.0], [0.0, 0.7]],
        },
        m,
    )
}

fn check_legacy(escapes: fn(&[f64; 3], &Snapshot, usize) -> bool) {
    let (s, m) = between();
    assert!(
        escapes(&m, &s, 2),
        "the legacy truth does not call body 2 unbound and receding"
    );
    let e_rel = kernel::detect::escape::e_rel(&m, &s.r, &s.p)[2];
    assert!(e_rel < 0.0, "E_rel with the total mass is {e_rel}");
    // Approaching instead of receding: not an escaper.
    let back = Snapshot {
        p: [[0.0, 0.0], [0.0, 0.0], [0.0, -0.7]],
        ..s
    };
    assert!(
        !escapes(&m, &back, 2),
        "an approaching body is called an escaper"
    );
    assert_eq!(Legacy::parse("E02"), Ok(Legacy::Escapes(0b101)));
    assert_eq!(Legacy::Escapes(0b101).word(), "E02");
}

/// The legacy test read with the total mass, the control.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn total_mass(m: &[f64; 3], s: &Snapshot, b: usize) -> bool {
    let e = kernel::detect::escape::e_rel(m, &s.r, &s.p)[b];
    e > 0.0 && revalidation::legacy_escapes(m, s, b)
}

#[test]
fn escape_legacy_truth_takes_the_pair_mass() {
    check_legacy(revalidation::legacy_escapes);
}

negative_control!(
    escape_legacy_truth_takes_the_pair_mass,
    "the legacy test with the total mass",
    expected = "does not call body 2 unbound and receding",
    check_legacy(total_mass)
);

fn check_band(tau: f64) {
    let at = burrau_firing(Rule::at(1e-3)).expect("Burrau fires at 1e-3");
    println!("Burrau at tau 1e-3: {at:?}");
    assert!(
        (at.0 - 157.576).abs() < 0.01 && at.1 == 0,
        "Burrau at 1e-3 fires at {at:?}, not 157.576 naming body 0"
    );
    let f = burrau_firing(Rule::at(tau)).expect("Burrau fires");
    println!("Burrau at tau {tau:e}: {f:?}");
    assert!(
        (BURRAU_WINDOW[0]..=BURRAU_WINDOW[1]).contains(&f.0) && f.1 == 0,
        "Burrau at tau {tau:e} fires at {f:?}, outside {BURRAU_WINDOW:?}"
    );
}

#[test]
fn escape_burrau_label_band() {
    check_band(3e-3);
}

negative_control!(
    escape_burrau_label_band,
    "tau 2e-3 fires at 123",
    expected = "outside",
    check_band(2e-3)
);
