//! The occupant-comparison harness (`validation::compare`; integrator_contract Part 2b § "The comparison discipline is
//! part of the seam, NOT a convention", § "One thing deliberately NOT decided").
//!
//! - `compare_refuses_stepper_mismatch`: KDK × logH against RK4 × Heggie is refused with a stepper mismatch, and
//!   `compare` returns that error, not a result; each other single-field change is refused with its own variant; the
//!   matched arms are admitted. Control: an admission whose stepper check is removed.
//! - `compare_ramp_shared`: every panel of a comparison holds the ramp constants passed in, bit for bit, and places each
//!   error through them. Control: panels auto-ranged per arm.
//! - `compare_provenance_full`: each arm's provenance names its stepper, its regularisation with the time
//!   transformation, every profile field, its pair-table cell, every step-control value and the discretisation.
//!   Control: a provenance without the step control.
//! - `compare_better_verdict`: the "which would have been better" verdict on stated errors: better only by the factor
//!   10 and above the reference's uncertainty, tie, unresolved and not judged. Control: a verdict that ignores the
//!   reference's uncertainty.
//! - `diagnostic_same_discretisation`: a diagnostic re-run with the run's own arm is admitted and reproduces the run
//!   bitwise; one with a per-window rescaled `Δ_sync`, a window's horizon, a rescaled `η` or another occupant is
//!   refused, naming the field. Control: a diagnostic admission that does not check the discretisation.
//! - `logh_heggie_same_arms`: the logH-vs-Heggie harness admits RK4 × Heggie with RK4 × logH under one step control,
//!   and refuses another stepper or another step control on one arm, a shared non-RK4 stepper, and a pair that is not
//!   Heggie and logH. Then it runs the falsification measurement (pitfalls §4.4) and prints it, per IC and in tally; the
//!   only assertion on it is that every IC is counted once. Control: an admission whose stepper check is removed.

use kernel::occupant::{DebugOccupant, Occupant, Selection, State};
use kernel::regularisation::az::End;
use kernel::regularisation::pairs::{Choice, HeggieTime, LogHMode};
use validation::compare::{
    admit, admit_diagnostic, admit_logh_heggie, compare, panels, provenance, rerun, run, verdict,
    Arm, Discretisation, Mismatch, Panel, RampConstants, Refusal, StepControl, Verdict, BETTER_BY,
};
use validation::negative_control;

const RK4: Selection = Selection::Production(Occupant::Rk4);
const KDK: Selection = Selection::Production(Occupant::Kdk);
const HEGGIE: Choice = Choice::Heggie(HeggieTime::Eq22);
const LOGH: Choice = Choice::LogH(LogHMode::LogH);

/// The discretisation the tests run: the register's `Δ_sync`, 1/8, to `T = 2`, sixteen boundaries.
const GRID: Discretisation = Discretisation {
    delta_sync: 0.125,
    horizon: 2.0,
};

fn arm(stepper: Selection, regularisation: Choice) -> Arm {
    Arm {
        stepper,
        regularisation,
        step_control: StepControl::TEST,
        discretisation: GRID,
    }
}

/// Burrau's three bodies of masses 3, 4, 5 in the gauge `M = 1` (masses over 12, lengths over 5), as
/// `kernel/tests/sync_quantum.rs` takes them, with small momenta summing to zero, and `(a, b)` added to body 0's
/// momentum and taken from body 1's, so the total stays zero.
fn burrau(a: f64, b: f64) -> State<f64> {
    State {
        m: [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0],
        r: [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0)),
        p: [[0.03 + a, -0.01 + b], [-0.02 - a, 0.04 - b], [-0.01, -0.03]],
    }
}

/// A 6 × 6 grid of ICs: the perturbation `(a, b)` over `[−0.05, 0.05]²`.
fn ics() -> Vec<State<f64>> {
    let n = 6;
    let at = |i: usize| -0.05 + 0.1 * i as f64 / (n - 1) as f64;
    (0..n)
        .flat_map(|j| (0..n).map(move |i| burrau(at(i), at(j))))
        .collect()
}

// ── compare_refuses_stepper_mismatch ───────────────────────────────────────────────────────────────────────────

fn check_stepper_mismatch(admit: impl Fn(&[Arm]) -> Result<(), Refusal>) {
    let matched = [arm(RK4, HEGGIE), arm(RK4, LOGH)];
    assert_eq!(admit(&matched), Ok(()), "the matched arms are refused");

    let mismatched = [arm(RK4, HEGGIE), arm(KDK, LOGH)];
    let got = admit(&mismatched);
    println!("compare_refuses_stepper_mismatch: RK4 × Heggie vs KDK × logH: {got:?}");
    assert_eq!(
        got,
        Err(Refusal::StepperMismatch {
            arm: 1,
            first: RK4,
            other: KDK
        }),
        "admitted arms that differ in stepper"
    );

    let mut eta = arm(RK4, LOGH);
    eta.step_control.eta = 0.02;
    let mut sync = arm(RK4, LOGH);
    sync.discretisation.delta_sync = 0.0625;
    let mut horizon = arm(RK4, LOGH);
    horizon.discretisation.horizon = 4.0;
    let cases = [
        (
            [arm(RK4, HEGGIE), eta],
            Refusal::StepControlMismatch {
                arm: 1,
                field: "eta",
            },
        ),
        (
            [arm(RK4, HEGGIE), sync],
            Refusal::DiscretisationMismatch {
                arm: 1,
                field: "delta_sync",
            },
        ),
        (
            [arm(RK4, HEGGIE), horizon],
            Refusal::DiscretisationMismatch {
                arm: 1,
                field: "horizon",
            },
        ),
        (
            [arm(RK4, HEGGIE), arm(RK4, Choice::None)],
            Refusal::Unregularised { arm: 1 },
        ),
    ];
    for (arms, want) in cases {
        let got = admit(&arms);
        println!("compare_refuses_stepper_mismatch: {want:?}: {got:?}");
        assert_eq!(
            got,
            Err(want),
            "a single-field mismatch is not refused as such"
        );
    }
    assert_eq!(admit(&matched[..1]), Err(Refusal::TooFewArms(1)));
}

#[test]
fn compare_refuses_stepper_mismatch() {
    check_stepper_mismatch(admit);
    // `compare` itself returns the refusal, never a result.
    let got = compare(
        &[arm(RK4, HEGGIE), arm(KDK, LOGH)],
        &ics(),
        RampConstants::POSITION_ERROR,
    )
    .map(|c| c.arms.len());
    assert!(
        matches!(got, Err(Refusal::StepperMismatch { .. })),
        "compare returned {got:?}"
    );
}

negative_control!(
    compare_refuses_stepper_mismatch,
    "an admission whose stepper check is removed",
    expected = "admitted arms that differ in stepper",
    check_stepper_mismatch(|arms| {
        let same: Vec<Arm> = arms
            .iter()
            .map(|a| Arm {
                stepper: arms[0].stepper,
                ..*a
            })
            .collect();
        admit(&same)
    })
);

// ── compare_ramp_shared ────────────────────────────────────────────────────────────────────────────────────────

fn check_ramp_shared(make: impl Fn(&[Vec<f64>], RampConstants) -> Vec<Panel>) {
    let ramp = RampConstants::POSITION_ERROR;
    let c = compare(&[arm(RK4, HEGGIE), arm(RK4, LOGH)], &ics()[..4], ramp)
        .expect("matched arms are compared");
    assert_eq!(c.ramp, ramp);
    assert_eq!(
        c.panels,
        panels(&c.errors, ramp),
        "compare's panels are not the shared ramp's"
    );
    let drawn = make(&c.errors, ramp);
    for (p, row) in drawn.iter().zip(&c.errors) {
        println!(
            "compare_ramp_shared: panel {}: decades {} to {}, errors {row:?}, placed {:.4?}",
            p.arm, p.constants.lo_decade, p.constants.hi_decade, p.placed
        );
        assert!(
            p.constants.lo_decade.to_bits() == ramp.lo_decade.to_bits()
                && p.constants.hi_decade.to_bits() == ramp.hi_decade.to_bits(),
            "panel {}'s ramp constants {:?} differ from the shared ramp {ramp:?}",
            p.arm,
            p.constants
        );
        for (x, placed) in row.iter().zip(&p.placed) {
            assert_eq!(ramp.place(*x).to_bits(), placed.to_bits());
        }
    }
}

#[test]
fn compare_ramp_shared() {
    check_ramp_shared(panels);
}

negative_control!(
    compare_ramp_shared,
    "panels auto-ranged per arm",
    expected = "differ from the shared ramp",
    check_ramp_shared(|errors, _| {
        errors
            .iter()
            .enumerate()
            .map(|(arm, row)| {
                let d: Vec<f64> = row
                    .iter()
                    .filter(|x| x.is_finite() && **x > 0.0)
                    .map(|x| x.log10())
                    .collect();
                let constants = RampConstants {
                    lo_decade: d.iter().copied().fold(f64::INFINITY, f64::min),
                    hi_decade: d.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                };
                Panel {
                    arm,
                    constants,
                    placed: row.iter().map(|&x| constants.place(x)).collect(),
                }
            })
            .collect()
    })
);

// ── compare_provenance_full ────────────────────────────────────────────────────────────────────────────────────

fn check_provenance(prov: impl Fn(&Arm) -> String) {
    for a in [
        arm(RK4, HEGGIE),
        arm(RK4, LOGH),
        arm(KDK, Choice::AarsethZare),
    ] {
        let text = prov(&a);
        println!("compare_provenance_full:\n{text}");
        let p = a.profile();
        let wants = [
            validation::pair_table::stepper_name(a.stepper).to_string(),
            validation::pair_table::label(a.regularisation).to_string(),
            format!("order {}", p.order),
            format!("force_evals {}", p.force_evals),
            format!("symplectic {}", p.symplectic),
            format!("step_reversible {}", p.step_reversible),
            format!("run_reversible {}", p.run_reversible),
            format!("owns_time_mapping {}", p.owns_time_mapping),
            "pair table: ".to_string(),
            format!("eta {:e}", a.step_control.eta),
            format!("tiny {:e}", a.step_control.tiny),
            format!("predictive_f {:e}", a.step_control.predictive_f),
            format!("step_budget_rate {:e}", a.step_control.step_budget_rate),
            format!("n_min {}", a.step_control.n_min),
            format!("clamp {}", a.step_control.clamp),
            format!("secant_tries {}", a.step_control.secant_tries),
            format!("bisect_tries {}", a.step_control.bisect_tries),
            format!("delta_sync {:e}", a.discretisation.delta_sync),
            format!("horizon {:e}", a.discretisation.horizon),
        ];
        for w in wants {
            assert!(text.contains(&w), "the provenance does not name `{w}`");
        }
    }
    // The occupant's profile is its pair's, not the stepper's: logH × KDK is symplectic in the logH mode, and every
    // regularised pair owns its time mapping.
    assert!(arm(KDK, LOGH).profile().symplectic);
    assert!(arm(RK4, HEGGIE).profile().owns_time_mapping);
}

#[test]
fn compare_provenance_full() {
    check_provenance(provenance);
}

negative_control!(
    compare_provenance_full,
    "a provenance without the step control",
    expected = "the provenance does not name `eta",
    check_provenance(|a| provenance(a)
        .lines()
        .filter(|l| !l.starts_with("step control"))
        .collect::<Vec<_>>()
        .join("\n"))
);

// ── compare_better_verdict ─────────────────────────────────────────────────────────────────────────────────────

fn check_verdict(v: impl Fn(f64, f64, f64) -> Verdict) {
    let cases = [
        // (e_a, e_b, reference uncertainty, verdict)
        (1e-8, 1e-6, 1e-12, Verdict::A),
        (1e-6, 1e-8, 1e-12, Verdict::B),
        (1e-8, 5e-8, 1e-12, Verdict::Tie),
        (1e-8, 1e-7, 1e-12, Verdict::A),
        // Both below ten times the reference's uncertainty: unresolved, whatever their ratio.
        (1e-14, 5e-12, 1e-12, Verdict::Unresolved),
        // The better arm's error below the reference's uncertainty: the ratio is taken against the uncertainty, so the
        // verdict is unresolved below ten times it and better at or above it, never a tie.
        (1e-14, 5e-11, 1e-11, Verdict::Unresolved),
        (1e-14, 1e-10, 1e-11, Verdict::A),
        (1e-9, 1e-14, 1e-11, Verdict::B),
        (f64::NAN, 1e-6, 1e-12, Verdict::NotJudged),
        (1e-8, 1e-6, f64::NAN, Verdict::NotJudged),
    ];
    for (ea, eb, u, want) in cases {
        let got = v(ea, eb, u);
        println!("compare_better_verdict: e_a {ea:e}, e_b {eb:e}, u {u:e}: {got:?}");
        assert_eq!(
            got, want,
            "e_a {ea:e}, e_b {eb:e}, u {u:e} is not judged as defined"
        );
    }
    assert_eq!(BETTER_BY, 10.0);
}

#[test]
fn compare_better_verdict() {
    check_verdict(verdict);
}

negative_control!(
    compare_better_verdict,
    "a verdict that ignores the reference's uncertainty",
    expected = "is not judged as defined",
    check_verdict(|ea, eb, u| if u.is_finite() {
        verdict(ea, eb, 0.0)
    } else {
        verdict(ea, eb, u)
    })
);

// ── diagnostic_same_discretisation ─────────────────────────────────────────────────────────────────────────────

fn check_diagnostic(admit: impl Fn(&Arm, &Arm) -> Result<(), Refusal>) {
    let under_test = arm(RK4, HEGGIE);
    assert_eq!(admit(&under_test, &under_test), Ok(()));

    // A persistence check over windows of 0.5 that rescales the grid to 8 boundaries per window, `n_sync` per window:
    // Δ_sync 0.0625. Then one that ends its re-run at its window, a horizon of its own; one that rescales the step
    // factor per window; and one on another occupant.
    let window = 0.5;
    let rescaled_grid = Arm {
        discretisation: Discretisation {
            delta_sync: window / 8.0,
            ..GRID
        },
        ..under_test
    };
    let window_horizon = Arm {
        discretisation: Discretisation {
            horizon: window,
            ..GRID
        },
        ..under_test
    };
    let rescaled_eta = Arm {
        step_control: StepControl {
            eta: StepControl::TEST.eta * window / GRID.horizon,
            ..StepControl::TEST
        },
        ..under_test
    };
    let cases = [
        (rescaled_grid, "delta_sync", 0.125, 0.0625),
        (window_horizon, "horizon", 2.0, 0.5),
        (rescaled_eta, "eta", 0.01, 0.0025),
    ];
    for (diagnostic, field, run, other) in cases {
        let got = admit(&under_test, &diagnostic);
        println!("diagnostic_same_discretisation: {field}: {got:?}");
        assert_eq!(
            got,
            Err(Refusal::Diagnostic(Mismatch {
                field,
                run,
                diagnostic: other
            })),
            "admitted a diagnostic with a rescaled {field}"
        );
    }
    let other = admit(&under_test, &arm(RK4, LOGH));
    assert!(
        matches!(
            other,
            Err(Refusal::Diagnostic(Mismatch {
                field: "regularisation",
                ..
            }))
        ),
        "admitted a diagnostic on another occupant: {other:?}"
    );
}

#[test]
fn diagnostic_same_discretisation() {
    check_diagnostic(admit_diagnostic);

    // The admitted re-run reproduces the run under test bitwise: the same discretisation is the same march.
    let under_test = arm(RK4, HEGGIE);
    let ics = &ics()[..3];
    let run_states: Vec<_> = ics.iter().map(|ic| run(&under_test, ic)).collect();
    let again = rerun(&under_test, &under_test, ics).expect("the run's own arm is admitted");
    for (a, b) in run_states.iter().zip(&again) {
        println!(
            "diagnostic_same_discretisation: run {:?} at {} in {} steps; re-run {:?} at {} in {} steps",
            a.end, a.boundary, a.steps, b.end, b.boundary, b.steps
        );
        assert_eq!(a.end, End::Marching);
        assert_eq!(a.boundary, 16);
        let bits = |t: &validation::compare::Trajectory| {
            t.state
                .r
                .iter()
                .chain(&t.state.p)
                .flatten()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>()
        };
        assert_eq!(bits(a), bits(b), "the re-run is not the run, bitwise");
    }
    let rescaled = Arm {
        discretisation: Discretisation {
            delta_sync: 0.0625,
            ..GRID
        },
        ..under_test
    };
    assert!(
        rerun(&under_test, &rescaled, ics).is_err(),
        "rerun ran a rescaled grid"
    );
}

negative_control!(
    diagnostic_same_discretisation,
    "a diagnostic admission that does not check the discretisation",
    expected = "admitted a diagnostic with a rescaled delta_sync",
    check_diagnostic(|run, diag| admit_diagnostic(
        &Arm {
            discretisation: diag.discretisation,
            ..*run
        },
        diag
    ))
);

// ── logh_heggie_same_arms ──────────────────────────────────────────────────────────────────────────────────────

fn check_logh_heggie(admit: impl Fn([Arm; 2]) -> Result<[Arm; 2], Refusal>) {
    let (hg, lh) = (arm(RK4, HEGGIE), arm(RK4, LOGH));
    assert_eq!(admit([hg, lh]), Ok([hg, lh]));
    assert_eq!(
        admit([lh, hg]),
        Ok([hg, lh]),
        "the arms are ordered Heggie, logH"
    );

    let got = admit([hg, arm(KDK, LOGH)]);
    println!("logh_heggie_same_arms: RK4 × Heggie vs KDK × logH: {got:?}");
    assert!(
        matches!(got, Err(Refusal::StepperMismatch { .. })),
        "admitted logH and Heggie on different steppers: {got:?}"
    );
    let mut eta = lh;
    eta.step_control.eta = 0.005;
    let got = admit([hg, eta]);
    println!("logh_heggie_same_arms: η 0.01 vs 0.005: {got:?}");
    assert_eq!(
        got,
        Err(Refusal::StepControlMismatch {
            arm: 1,
            field: "eta"
        })
    );
    let mut budget = lh;
    budget.step_control.predictive_f = 0.0;
    assert_eq!(
        admit([hg, budget]),
        Err(Refusal::StepControlMismatch {
            arm: 1,
            field: "predictive_f"
        })
    );
    let euler = Selection::Debug(DebugOccupant::Euler);
    assert_eq!(
        admit([arm(KDK, HEGGIE), arm(KDK, LOGH)]),
        Err(Refusal::NotRk4(KDK))
    );
    assert_eq!(
        admit([arm(euler, HEGGIE), arm(euler, LOGH)]),
        Err(Refusal::NotRk4(euler))
    );
    assert_eq!(
        admit([hg, arm(RK4, Choice::AarsethZare)]),
        Err(Refusal::NotLogHHeggie)
    );
}

#[test]
fn logh_heggie_same_arms() {
    check_logh_heggie(admit_logh_heggie);

    // The falsification measurement (pitfalls §4.4), reported, not judged: logH has no coordinate transformation at all,
    // so if the re-registration mechanism is real it should match or beat Heggie.
    let arms = admit_logh_heggie([arm(RK4, HEGGIE), arm(RK4, Choice::LogH(LogHMode::LogH))])
        .expect("the matched arms are admitted");
    let ics = ics();
    let c = compare(&arms, &ics, RampConstants::POSITION_ERROR).expect("admitted arms compare");
    println!("logh_heggie_same_arms:\n{}", c.report());
    let better = c.better(0, 1);
    let tally = better.tally();
    assert_eq!(
        tally.iter().sum::<usize>(),
        ics.len(),
        "every IC is counted once"
    );
    for row in &c.runs {
        assert_eq!(row.len(), ics.len());
    }
}

negative_control!(
    logh_heggie_same_arms,
    "an admission whose stepper check is removed",
    expected = "admitted logH and Heggie on different steppers",
    check_logh_heggie(|arms| admit_logh_heggie(arms.map(|a| Arm {
        stepper: if matches!(a.regularisation, Choice::LogH(_) | Choice::Heggie(_)) {
            arms[0].stepper
        } else {
            a.stepper
        },
        ..a
    })))
);
