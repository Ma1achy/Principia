//! Heggie's gates (TASK-M3-08): `radial-collision` (REQ-VAL-045), `hamiltonian-fd` (REQ-VAL-170, REQ-VAL-171) and
//! `separation-floor`'s Heggie arm (REQ-INT-105): their verdicts at and past their thresholds, their controls, their
//! refusals, their flags, and the runner on their fixtures. Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use serde_json::json;
use validation::az::FLOOR_NAME;
use validation::gate::{self, Gate, GateConfig, Verdict};
use validation::heggie::{
    decide_fd, discrepancy, distinct, falls, floor_control_heggie, passage, propose_floor_heggie,
    relative, round_up_one, FdOccupant, HamiltonianFdGate, Occupant, RadialCollisionGate, Reader,
    D_MIN_BELOW, FD_NAME, RADIAL_NAME,
};
use validation::negative_control;

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch directory for reports.
fn out(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("heggie-gates-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn config(name: &str, threshold: f64) -> GateConfig {
    let mut c = gate::config(&root(), name).expect("the gate's gate.json");
    c.threshold.value = threshold;
    c
}

// ── radial-collision ───────────────────────────────────────────────────────────────────────────────────────────

/// A `radial-collision` input at `η = 0.01` on the fixtures' four-rung halving ladder.
fn radial_input(occupant: &str, reader: &str) -> serde_json::Value {
    json!({
        "description": "a test input", "occupant": occupant, "reader": reader, "eta": 0.01,
        "ladder": [0.01, 0.005, 0.0025, 0.00125]
    })
}

/// `falls` at its edge: the finest rung `2^(n − 1)` times below the coarsest falls, a hair less does not, and a ladder of
/// fewer than two rungs never does.
fn check_falls(f: impl Fn(&[f64]) -> bool) {
    assert!(
        f(&[4.0, 2.0, 1.0]),
        "a fall of exactly 2^(n − 1) does not read as falling"
    );
    assert!(
        !f(&[4.0f64.next_down(), 2.0, 1.0]),
        "a fall short of 2^(n − 1) reads as falling"
    );
    assert!(f(&[2.0, 1.0]), "two rungs, a fall of 2");
    assert!(!f(&[1.0]), "one rung");
    assert!(!f(&[]), "no rung");
}

#[test]
fn radial_collision_falls() {
    check_falls(falls);
}

negative_control!(
    radial_collision_falls,
    "a fall read against 2^n, one rung too many",
    expected = "a fall of exactly 2^(n − 1) does not read as falling",
    check_falls(|x: &[f64]| x.len() > 1 && x[0] >= x[x.len() - 1] * 2f64.powi(x.len() as i32))
);

/// Both occupants' passages go below `1e-10` and hold the regularised drift below `1e-12`, Heggie's Cartesian read-out
/// after the passage differing from it, and the read-outs' arm Heggie's alone; the gate passes just above the read drift
/// and fails at it; the Cartesian reader fails; an unknown occupant or reader and a short ladder are refused; the runner
/// gives each fixture its expected outcome. `reader` is the gate's reader for the passing verdicts (the control points
/// it at the Cartesian).
fn check_radial(reader: &str) {
    for (name, occupant) in [
        ("heggie", Occupant::Heggie),
        ("aarseth-zare", Occupant::AarsethZare),
    ] {
        let p = passage(occupant, 0.01);
        println!("radial-collision: {name}: {p:?}");
        assert!(p.d_min < D_MIN_BELOW, "{name}: d_min {:.3e}", p.d_min);
        assert!(
            p.regularised < 1e-12,
            "{name}: the regularised drift {:.3e}",
            p.regularised
        );
        if occupant == Occupant::Heggie {
            assert_ne!(
                p.cartesian, p.regularised,
                "{name}: the Cartesian read-out is the regularised drift"
            );
        }
        assert_eq!(Reader::Regularised.read(&p), p.regularised);
        assert_eq!(Reader::Cartesian.read(&p), p.cartesian);
        let at = |threshold: f64, reader: &str| {
            RadialCollisionGate
                .run(
                    &config(RADIAL_NAME, threshold),
                    "t",
                    &radial_input(name, reader),
                )
                .map(|r| r.verdict)
        };
        let read = p.regularised;
        assert_eq!(
            at(read.next_up(), reader),
            Ok(Verdict::Pass),
            "{name}: just above the read drift"
        );
        assert_eq!(
            at(read, reader),
            Ok(Verdict::Fail),
            "{name}: at the read drift"
        );
        assert_eq!(
            at(1.0, "cartesian"),
            Ok(Verdict::Fail),
            "{name}: the Cartesian reader"
        );
    }
    let refused = |input: serde_json::Value, reads: &str| {
        let e = RadialCollisionGate
            .run(&config(RADIAL_NAME, 1e-12), "t", &input)
            .expect_err("refused");
        assert!(e.contains(reads), "{e}");
    };
    refused(radial_input("logh", "regularised"), "no occupant `logh`");
    refused(radial_input("heggie", "other"), "no reader `other`");
    refused(
        json!({ "description": "", "occupant": "heggie", "reader": "regularised", "eta": 0.01, "ladder": [0.01] }),
        "two rungs or more",
    );
    assert!(
        distinct(true, Occupant::Heggie) && distinct(true, Occupant::AarsethZare),
        "differing read-outs"
    );
    assert!(
        distinct(false, Occupant::AarsethZare),
        "Aarseth–Zare's read-outs may coincide"
    );
    assert!(
        !distinct(false, Occupant::Heggie),
        "Heggie's read-outs coincide and pass"
    );
    assert_eq!(relative(3.0, 2.0), 0.5);
    assert_eq!(relative(1.0, -2.0), 1.5);
    gate::run(&root(), RADIAL_NAME, &out("radial")).expect("each input its expected outcome");
}

#[test]
fn radial_collision_gate() {
    check_radial("regularised");
}

negative_control!(
    radial_collision_gate,
    "the gate's reader pointed at the Cartesian read-out",
    expected = "just above the read drift",
    check_radial("cartesian")
);

// ── hamiltonian-fd ─────────────────────────────────────────────────────────────────────────────────────────────

/// A `hamiltonian-fd` input over 8 states.
fn fd_input(occupant: &str, flip: bool, samples: u32) -> serde_json::Value {
    json!({ "description": "a test input", "occupant": occupant, "step": 1e-5, "samples": samples, "seed": 7, "flip": flip })
}

/// Each occupant's rates against the finite difference: the gate passes at the measured discrepancy and fails just
/// below it; the flipped sign fails; an unknown occupant and no samples are refused; the proposal's decision takes the
/// least row (the first on a tie) at ten times its discrepancy rounded up to one digit; the runner gives each fixture its
/// expected outcome. `flip` is the gate's input for the passing verdicts (the control flips it).
fn check_fd(flip: bool) {
    for (name, o) in [
        ("heggie-eq20", FdOccupant::HeggieEq20),
        ("heggie-eq22", FdOccupant::HeggieEq22),
        ("aarseth-zare", FdOccupant::AarsethZare),
    ] {
        assert_eq!(FdOccupant::parse(name), Some(o));
        let d = discrepancy(o, 1e-5, 8, 7, false);
        let flipped = discrepancy(o, 1e-5, 8, 7, true);
        println!("hamiltonian-fd: {name}: {d:.3e}, flipped {flipped:.3e}");
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
    assert_eq!(FdOccupant::parse("logh"), None);
    for (input, reads) in [
        (fd_input("logh", false, 8), "no occupant `logh`"),
        (fd_input("heggie-eq20", false, 0), "no samples"),
    ] {
        let e = HamiltonianFdGate
            .run(&config(FD_NAME, 1.0), "t", &input)
            .expect_err("refused");
        assert!(e.contains(reads), "{e}");
    }
    let near = |x: f64, y: f64| ((x - y) / y).abs() < 1e-12;
    assert!(
        near(round_up_one(1.2e-9), 2e-9) && near(round_up_one(0.31), 0.4),
        "rounded up to one digit"
    );
    assert_eq!(round_up_one(3.0), 3.0, "a single digit kept");
    let (step, tolerance) =
        decide_fd(&[(1e-3, 4e-8), (1e-4, 2.4e-9), (1e-5, 2.4e-9), (1e-6, 5e-9)]);
    assert!(
        step == 1e-4 && near(tolerance, 3e-8),
        "the least row, the first on a tie: {step:e}, {tolerance:e}"
    );
    let (step, tolerance) = decide_fd(&[(1e-3, 1.2e-9), (1e-4, 2.2e-9)]);
    assert!(
        step == 1e-3 && near(tolerance, 2e-8),
        "the first row least: {step:e}, {tolerance:e}"
    );
    gate::run(&root(), FD_NAME, &out("fd")).expect("each input its expected outcome");
}

#[test]
fn hamiltonian_fd_gate() {
    check_fd(false);
}

negative_control!(
    hamiltonian_fd_gate,
    "the gate's input with a sign flipped",
    expected = "at the discrepancy",
    check_fd(true)
);

/// The gate's flags: `hamiltonian-fd` takes `--propose` alone and prints its proposal; `radial-collision` has none.
fn check_flags(name: &str, args: &[&str], reads: &str) {
    let e = gate::run_flags(&root(), name, args).expect_err("the flags were not refused");
    assert!(e.contains(reads), "{e}");
}

#[test]
fn heggie_gate_flags_refused() {
    check_flags(FD_NAME, &["--nothing"], "usage: --propose");
    check_flags(FD_NAME, &["--propose", "x"], "unrecognised flags");
    check_flags(RADIAL_NAME, &["--propose"], "has none of its own");
}

negative_control!(
    heggie_gate_flags_refused,
    "a flag the gate takes",
    expected = "the flags were not refused",
    check_flags(FD_NAME, &["--propose"], "")
);

// ── separation-floor's Heggie arm ──────────────────────────────────────────────────────────────────────────────

/// Heggie's floor control: a `TINY` ten times the passage's least pre-floor `dt/dτ` floors steps, the reference's none,
/// another is refused; the proposal carries Heggie's table; the runner gives each control its expected outcome.
fn check_floor(tiny: &str) {
    let (md, floored) = floor_control_heggie("t", "a test input", tiny).expect("the control");
    assert!(floored, "the floor binds above the least value: {md}");
    let (_, floored) = floor_control_heggie("t", "", "reference").expect("the control");
    assert!(!floored, "the reference's TINY floors a step");
    assert!(floor_control_heggie("t", "", "other")
        .unwrap_err()
        .contains("no TINY `other`"));
    let proposal = propose_floor_heggie();
    assert!(
        proposal.contains("## Heggie's arm") && proposal.contains("| Eq. 22 | f64 |"),
        "{proposal}"
    );
    gate::run(&root(), FLOOR_NAME, &out("floor")).expect("each control its expected outcome");
}

#[test]
fn separation_floor_heggie() {
    check_floor("above");
}

negative_control!(
    separation_floor_heggie,
    "the reference's TINY as the control that must bind",
    expected = "the floor binds above the least value",
    check_floor("reference")
);
