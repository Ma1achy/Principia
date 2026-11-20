//! logH's arms of `hamiltonian-fd` and `separation-floor` (`validation::logh`): their verdicts at and past their
//! thresholds, their draws, their controls and their refusals, beside Heggie's and Aarseth–Zare's
//! (`tests/heggie_gates.rs`, which also runs the gates' fixtures, logH's among them). Each criterion was written
//! before its first run.

use std::path::{Path, PathBuf};

use kernel::occupant::rk4::Rk4;
use kernel::regularisation::az::End;
use kernel::regularisation::pairs::LogHMode;
use serde_json::json;
use validation::az::{radial, FLOOR_NAME};
use validation::decode_f32_eps::SplitMix;
use validation::gate::{self, Gate, GateConfig, Verdict};
use validation::heggie::{discrepancy, draw, uniform, FdOccupant, HamiltonianFdGate, FD_NAME};
use validation::logh::{
    floor_control_logh, kinetic, lambda, lh_floor_run, logh_at, omega, potential,
    propose_floor_logh, separations, FD_SEPARATION, FLOOR_BUDGET,
};
use validation::negative_control;

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch directory for reports.
fn out(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("logh-gates-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn config(name: &str, threshold: f64) -> GateConfig {
    let mut c = gate::config(&root(), name).expect("the gate's gate.json");
    c.threshold.value = threshold;
    c
}

/// logH's two finite-difference occupants, with their input names and modes.
const ARMS: [(&str, FdOccupant, LogHMode); 2] = [
    ("logh", FdOccupant::LogH, LogHMode::LogH),
    ("logh-ttl", FdOccupant::LogHTtl, LogHMode::Ttl),
];

// ── hamiltonian-fd ─────────────────────────────────────────────────────────────────────────────────────────────

/// A `hamiltonian-fd` input over `samples` states.
fn fd_input(occupant: &str, flip: bool, samples: u32) -> serde_json::Value {
    json!({ "description": "a test input", "occupant": occupant, "step": 1e-5, "samples": samples, "seed": 7, "flip": flip })
}

/// Each of logH's time modes against the finite difference, the criterion Heggie's arms are held to
/// (`hamiltonian_fd_gate`): over 8 states at step `1e-5` the discrepancy is below `1e-6` and a flipped sign reads
/// above `0.5`; the gate passes at the measured discrepancy and fails just below it; the flipped sign fails. `flip` is
/// the gate's input for the passing verdicts (the control flips it).
fn check_fd(flip: bool) {
    for (name, o, _) in ARMS {
        assert_eq!(FdOccupant::parse(name), Some(o));
        let d = discrepancy(o, 1e-5, 8, 7, false);
        let flipped = discrepancy(o, 1e-5, 8, 7, true);
        println!(
            "hamiltonian-fd: {name}: discrepancy {d:.3e} (criterion below 1e-6), flipped {flipped:.3e} (criterion \
             above 0.5)"
        );
        assert!(
            d < 1e-6 && flipped > 0.5,
            "{name}: {d:.3e}, flipped {flipped:.3e}"
        );
        let at = |threshold: f64, flip: bool| {
            HamiltonianFdGate
                .run(&config(FD_NAME, threshold), "t", &fd_input(name, flip, 8))
                .map(|r| r.verdict)
        };
        assert_eq!(at(d, flip), Ok(Verdict::Pass), "{name}: at the discrepancy");
        assert_eq!(
            at(d.next_down(), flip),
            Ok(Verdict::Fail),
            "{name}: below it"
        );
        assert_eq!(at(1e-6, true), Ok(Verdict::Fail), "{name}: flipped");
    }
}

#[test]
fn hamiltonian_fd_logh() {
    check_fd(false);
}

negative_control!(
    hamiltonian_fd_logh,
    "the gate's input with a sign flipped",
    expected = "at the discrepancy",
    check_fd(true)
);

/// logH's draws: positions on `[−1.2, 1.2]` with every pair at least [`FD_SEPARATION`] apart, momenta on `[−1, 1]`,
/// and the last value on its mode's range, `E = −B` on `[−3, −0.5]` (logH) or `W` on `[0.5, 3]` (TTL), each range's
/// two halves reached; the masses are the draw's first three values; the logH draw's Hamiltonian is `Λ`, which is
/// `ln` of the kernel's drift denominator less `ln` of its kick denominator at that state, to `1e-12`; and the TTL
/// draw carries its own finite differences, seven momentum rates (`p`'s six, then `W`'s), and its two denominators are
/// the kernel's `W` and `Ω`. `swap` reads each draw in the other mode (the control).
fn check_draws(swap: bool) {
    for (name, occupant, mode) in ARMS {
        let read_as = match (mode, swap) {
            (LogHMode::LogH, false) | (LogHMode::Ttl, true) => LogHMode::LogH,
            _ => LogHMode::Ttl,
        };
        let last = match read_as {
            LogHMode::LogH => (-3.0, -0.5),
            LogHMode::Ttl => (0.5, 3.0),
        };
        let ranges = [(-1.2, 1.2), (-1.0, 1.0), last];
        let mut seen = [(f64::INFINITY, f64::NEG_INFINITY); 3];
        let mut rng = SplitMix::new(11);
        for _ in 0..2000 {
            let mut masses_rng = rng.clone();
            let masses = [0; 3].map(|_| uniform(&mut masses_rng, 0.3, 3.0));
            let d = draw(occupant, &mut rng);
            assert_eq!(d.x.len(), 13);
            for (i, &v) in d.x.iter().enumerate() {
                let g = (i / 6).min(2);
                assert!(
                    v >= ranges[g].0 && v <= ranges[g].1,
                    "{name}: x[{i}] = {v} is off its range"
                );
                seen[g] = (seen[g].0.min(v), seen[g].1.max(v));
            }
            let least = separations(&d.x).into_iter().fold(f64::INFINITY, f64::min);
            assert!(least >= FD_SEPARATION, "{name}: a pair {least} apart");
            let (system, q, p) = logh_at(&d.x, masses, mode);
            let g = kernel::physics::PairGeom::new(&q.r);
            let (dq, dp, _) = (d.rates)(&d.x);
            match mode {
                LogHMode::LogH => {
                    assert!(d.differenced.is_none(), "logH's draw is a Hamiltonian's");
                    assert_eq!((d.hamiltonian)(&d.x), lambda(&d.x, masses));
                    let kernels = system.drift_raw(&p).ln() - system.kick_raw(&g).ln();
                    assert!(
                        (lambda(&d.x, masses) - kernels).abs() < 1e-12,
                        "Λ is not ln(K + B) − ln U of the kernel's denominators"
                    );
                    assert_eq!((dq.len(), dp.len()), (6, 6));
                }
                LogHMode::Ttl => {
                    assert!(d.differenced.is_some(), "TTL's draw has no Hamiltonian");
                    assert_eq!(
                        (d.hamiltonian)(&d.x),
                        kinetic(&d.x, masses) - potential(&d.x, masses)
                    );
                    assert_eq!(system.drift_raw(&p), d.x[12], "the drift divides by W");
                    assert!(
                        (system.kick_raw(&g) / omega(&d.x, masses) - 1.0).abs() < 1e-12,
                        "the kick does not divide by Ω"
                    );
                    assert_eq!((dq.len(), dp.len()), (6, 7), "W's rate after p's");
                }
            }
        }
        for (g, (lo, hi)) in seen.iter().enumerate() {
            let quarter = (ranges[g].1 - ranges[g].0) / 4.0;
            assert!(
                *lo < ranges[g].0 + quarter && *hi > ranges[g].1 - quarter,
                "{name}: group {g} spans {lo}..{hi}"
            );
        }
    }
}

#[test]
fn logh_fd_draws() {
    check_draws(false);
}

negative_control!(
    logh_fd_draws,
    "each draw read in the other time mode: the logH draw's energy on W's range",
    expected = "is off its range",
    check_draws(true)
);

// ── separation-floor's logH arm ────────────────────────────────────────────────────────────────────────────────

/// logH's floor control, in each time mode: a `TINY` ten times the passage's least pre-floor drift denominator floors
/// steps, the reference's none, another is refused; the proposal carries logH's table, both modes and both steppers,
/// and the passage with the predictive limit off; the runner gives each control its expected outcome.
fn check_floor(tiny: &str) {
    for mode in [LogHMode::LogH, LogHMode::Ttl] {
        let (md, floored) =
            floor_control_logh("t", "a test input", tiny, mode).expect("the control");
        println!("{md}");
        assert!(
            floored,
            "{mode:?}: the floor binds above the least value: {md}"
        );
        let (_, floored) = floor_control_logh("t", "", "reference", mode).expect("the control");
        assert!(!floored, "{mode:?}: the reference's TINY floors a step");
        assert!(floor_control_logh("t", "", "other", mode)
            .unwrap_err()
            .contains("no TINY `other`"));
    }
    let proposal = propose_floor_logh();
    for part in [
        "## logH's arm",
        "| the radial collision passage | logH | KDK | 0.02 | f64 |",
        "| the radial collision passage | TTL | RK4 | 0.02 | f32 |",
        "| Burrau's (3, 4, 5) from rest | TTL | KDK | 0.02 | f64 |",
        "| the radial collision passage | logH | KDK | 0 | f64 |",
    ] {
        assert!(proposal.contains(part), "no `{part}` in {proposal}");
    }
    gate::run(&root(), FLOOR_NAME, &out("floor")).expect("each control its expected outcome");
}

#[test]
fn separation_floor_logh() {
    check_floor("above");
}

negative_control!(
    separation_floor_logh,
    "the reference's TINY as the control that must bind",
    expected = "the floor binds above the least value",
    check_floor("reference")
);

/// The floor runs' step budget ends a stalled march: logH × RK4 on the radial collision passage with the predictive
/// limit off stalls at the collision (measured), and ends exhausted one step past `budget`. `budget` is the one
/// checked against (the control offers twice the runs' own).
fn check_budget(budget: u32) {
    let run = lh_floor_run::<Rk4, f64>(&radial(), LogHMode::LogH, None, Some(0.0));
    println!(
        "separation-floor, logH × RK4 on the radial passage, the limit off: {:?} at t = {:.4} after {} steps (criterion: \
         Exhausted after {} steps)",
        run.end,
        run.t,
        run.steps,
        budget + 1
    );
    assert_eq!(
        (run.end, run.steps),
        (End::Exhausted, budget + 1),
        "the stalled march does not end on the runs' budget"
    );
}

#[test]
fn separation_floor_logh_budget() {
    check_budget(FLOOR_BUDGET);
}

negative_control!(
    separation_floor_logh_budget,
    "a budget twice the runs' own",
    expected = "the stalled march does not end on the runs' budget",
    check_budget(2 * FLOOR_BUDGET)
);
