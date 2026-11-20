//! QA's tests for TASK-M2-82's re-check (REQ-CHART-173; chart_composition § 18.7; R-918, R-919 F18; RQ-757 decided
//! per R-369 as option (a)), written from the requirement and RQ-757's outcome, not from the implementation. Every test
//! is named `trajectory_interface_qa_*`, so the task's acceptance filter `cargo test -p engine trajectory_interface`
//! runs it.
//!
//! - `_inputs_give_a_grid`: the Lagrange fixture's inputs are accepted exactly where QA's own reading of the member
//!   makes them one (masses finite and positive, an elliptic `e` in `(0, 1)`, the divisions finite and positive), and
//!   every accepted fixture gives the requirement's interface: a finite, positive quantum, a trajectory over a horizon
//!   of a few quanta, and finite rows, the same bits for the same inputs from a second construction;
//! - `_homothetic_holds_last_finite_row`: RQ-757's option (a), the collapse's terminal holds the state at the last
//!   boundary before `t_c`, so every row from the terminal's index on is that row's bits;
//! - controls (R-176): an inadmissible input taken for admissible, a row not finite, and the held state compared with a
//!   row two boundaries before the terminal.

use engine::trajectory::{same_bits, Lagrange, Member, Provider, Refused, Trajectory};
use kernel::decode::Ic;
use validation::negative_control;

fn inputs() -> ledger::fixtures::Lagrange {
    ledger::fixtures::lagrange()
}

/// QA's reading of a Lagrange member's inputs: the masses finite and positive, the divisions of its span finite and
/// positive, and an elliptic member's `e` strictly between the circle (0) and the radial orbit (1).
fn admissible(m: [f64; 3], member: Member, divisions: f64) -> bool {
    let positive = |x: f64| x.is_finite() && x > 0.0;
    let e_ok = match member {
        Member::Elliptic { e } => e.is_finite() && e > 0.0 && e < 1.0,
        Member::Circular | Member::Homothetic => true,
    };
    m.iter().all(|&x| positive(x)) && positive(divisions) && e_ok
}

/// Every component of `s` is finite.
fn check_finite(s: &Ic<f64>, what: &str) {
    let finite = s.m.iter().all(|x| x.is_finite())
        && s.r
            .iter()
            .chain(&s.p)
            .all(|v| v[0].is_finite() && v[1].is_finite());
    assert!(finite, "{what} is not finite: {s:?}");
}

/// `try_new` accepts `(m, member, divisions)` exactly where they are admissible, refuses with `Refused::Inputs`
/// otherwise, and an accepted fixture gives a grid of finite rows, the same bits from a second construction.
fn check_inputs(m: [f64; 3], member: Member, divisions: f64) {
    let what = format!("{m:?}, {member:?}, {divisions}");
    match (
        admissible(m, member, divisions),
        Lagrange::try_new(m, member, divisions),
    ) {
        (false, Err(e)) => assert_eq!(e, Refused::Inputs, "{what}: refused as {e:?}"),
        (false, Ok(_)) => panic!("{what}: inadmissible inputs are accepted"),
        (true, Err(e)) => panic!("{what}: admissible inputs is refused as {e:?}"),
        (true, Ok(p)) => {
            let q = p.quantum();
            assert!(
                q.is_finite() && q > 0.0,
                "{what}: the quantum {q} is not finite and positive"
            );
            let start = p.start();
            check_finite(&start, &format!("{what}: the start"));
            let t = Trajectory::new(&p, start, 4.5 * q)
                .unwrap_or_else(|e| panic!("{what}: the trajectory is refused: {e:?}"));
            assert_eq!(t.last(), 4, "{what}: the horizon 4.5Δ ends at boundary 4");
            let again =
                Lagrange::try_new(m, member, divisions).expect("accepted once, accepted again");
            let u = Trajectory::new(&again, again.start(), 4.5 * again.quantum())
                .expect("the same trajectory");
            for k in 0..=4 {
                let row = t
                    .state(k)
                    .unwrap_or_else(|e| panic!("{what}: boundary {k} refused: {e:?}"));
                check_finite(&row, &format!("{what}: row {k}"));
                assert!(
                    same_bits(&row, &u.state(k).expect("the same boundary")),
                    "{what}: row {k} differs between two constructions from the same inputs"
                );
            }
        }
    }
}

#[test]
fn trajectory_interface_qa_inputs_give_a_grid() {
    let good = inputs().masses;
    let masses_sets: Vec<[f64; 3]> = {
        let mut v = vec![good, [1.0 / 3.0; 3], [0.98, 0.01, 0.01]];
        for bad in [0.0, -0.0, -0.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for i in 0..3 {
                let mut m = good;
                m[i] = bad;
                v.push(m);
            }
        }
        v
    };
    let members = [
        Member::Circular,
        Member::Homothetic,
        Member::Elliptic {
            e: inputs().eccentricity,
        },
        Member::Elliptic { e: 1e-300 },
        Member::Elliptic { e: 0.999 },
        Member::Elliptic { e: 0.0 },
        Member::Elliptic { e: -0.0 },
        Member::Elliptic { e: 1.0 },
        Member::Elliptic { e: -0.25 },
        Member::Elliptic { e: 2.0 },
        Member::Elliptic { e: f64::NAN },
        Member::Elliptic { e: f64::INFINITY },
    ];
    let divisions = [
        8.0,
        inputs().per_collapse,
        0.5,
        1e6,
        0.0,
        -0.0,
        -8.0,
        f64::NAN,
        f64::INFINITY,
    ];
    for m in &masses_sets {
        for member in members {
            for d in divisions {
                check_inputs(*m, member, d);
            }
        }
    }
}

negative_control!(
    trajectory_interface_qa_inputs_give_a_grid,
    "a radial e = 1, inadmissible for an elliptic member, taken as admissible fails",
    expected = "is refused as",
    {
        let what = "e = 1 taken as admissible";
        match Lagrange::try_new(inputs().masses, Member::Elliptic { e: 1.0 }, 8.0) {
            Ok(_) => {}
            Err(e) => panic!("{what}: admissible inputs is refused as {e:?}"),
        }
    }
);

mod finite_control {
    use super::*;
    negative_control!(
        trajectory_interface_qa_inputs_give_a_grid,
        "a row with a NaN component fails the finiteness check",
        expected = "is not finite",
        {
            let mut s = Lagrange::new(inputs().masses, Member::Circular, 8.0).start();
            s.p[1][0] = f64::NAN;
            check_finite(&s, "a poisoned row")
        }
    );
}

/// Every row from `index` to `last` is row `index − 1`'s bits: the terminal holds the last finite row before `t_c`.
fn check_holds_row_before(index: u64, last: u64, row: impl Fn(u64) -> Ic<f64>, before: u64) {
    let held = row(before);
    for k in index..=last {
        assert!(
            same_bits(&row(k), &held),
            "row {k} is not the row before the terminal, {before}"
        );
    }
}

#[test]
fn trajectory_interface_qa_homothetic_holds_last_finite_row() {
    let f = inputs();
    for divisions in [f.per_collapse, f.per_collapse_on_boundary, 1.5, 0.75] {
        let h = Lagrange::new(f.masses, Member::Homothetic, divisions);
        let s = h.start();
        let tc = h.collapse().expect("the homothetic member collapses");
        let dt = h.quantum();
        // QA's own first boundary at or after t_c, within a relative margin for the quantum's rounding.
        let mut c = 0u64;
        while (c as f64) * dt < tc * (1.0 - 1e-12) {
            c += 1;
        }
        let t = Trajectory::new(&h, s, (c as f64 + 3.5) * dt).expect("the trajectory");
        let term = t
            .terminal()
            .expect("the collapse is a terminal within the horizon");
        assert_eq!(
            term.index, c,
            "the terminal at the first boundary at or after t_c"
        );
        assert!(c >= 1, "t_c lies past the start");
        let row = |k: u64| t.state(k).expect("a boundary within the horizon");
        check_holds_row_before(c, t.last(), row, c - 1);
        check_finite(&row(c), "the held state");
        if c >= 2 {
            assert!(
                !same_bits(&row(c - 1), &row(c - 2)),
                "the rows before the terminal still move"
            );
        }
    }
}

negative_control!(
    trajectory_interface_qa_homothetic_holds_last_finite_row,
    "the held state compared with the row two before the terminal fails",
    expected = "is not the row before the terminal",
    {
        let f = inputs();
        let h = Lagrange::new(f.masses, Member::Homothetic, f.per_collapse);
        let t = Trajectory::new(&h, h.start(), 9.5 * h.quantum()).expect("the trajectory");
        let c = t.terminal().expect("a terminal").index;
        check_holds_row_before(c, t.last(), |k| t.state(k).unwrap(), c - 2)
    }
);
