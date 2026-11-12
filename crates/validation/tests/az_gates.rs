//! Aarseth–Zare's gates (TASK-M3-68): `lc-inverse` (REQ-INT-044), `az-final-step` (REQ-INT-049, REQ-INT-114) and the
//! `separation-floor` measurement (REQ-INT-105): their verdicts at and past their thresholds, their refusals, their
//! flags, and the runner on their fixtures. Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use kernel::regularisation::az::End;
use serde_json::json;
use validation::az::{
    closure, convergence, decide_sync, doubly_degenerate, figure_eight, lc_error, orbit,
    propose_final_step_with, propose_floor, propose_sync_with, radial, round_up, AzFinalStepGate,
    LcInverseGate, SeparationFloorGate, SyncRow, FINAL_STEP_NAME, FLOOR_NAME, LC_NAME, SYNC_ERROR,
};
use validation::gate::{self, Gate, GateConfig, GateSpec, MeasurementGate, Verdict};
use validation::negative_control;

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch directory for reports.
fn out(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("az-gates-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn config(name: &str, threshold: f64) -> GateConfig {
    let mut c = gate::config(&root(), name).expect("the gate's gate.json");
    c.threshold.value = threshold;
    c
}

/// The `lc-inverse` verdict of `form` at 179.9° under `threshold` (in units in `R`'s last place).
fn lc_verdict(form: &str, threshold: f64) -> Result<Verdict, String> {
    let input = json!({ "description": "a test input", "form": form, "angle": 179.9 });
    LcInverseGate
        .run(&config(LC_NAME, threshold), "t", &input)
        .map(|r| r.verdict)
}

/// The conditioned inverse within one unit in `R`'s last place at both widths, the requirement's figures (relative
/// `≈ 5.96e-8` and `≈ 1e-16`), and the unstable form far beyond it (`2.2e-2`, `4.0e-12`); the verdict at the larger
/// width's error passes and just below it fails; an unknown form is refused; the runner gives each fixture its
/// expected outcome.
fn check_lc(conditioned: &str) {
    let (c32, c64) = (lc_error::<f32>(179.9, true), lc_error::<f64>(179.9, true));
    let (u32_, u64_) = (lc_error::<f32>(179.9, false), lc_error::<f64>(179.9, false));
    assert!(
        c32.ulps <= 1.0 && c64.ulps <= 1.0,
        "conditioned: {c32:?}, {c64:?}"
    );
    assert!(
        c32.relative < 6.0e-8 && c64.relative < 1.2e-16,
        "conditioned: {c32:?}, {c64:?}"
    );
    assert!(
        u32_.relative > 2.0e-2 && u64_.relative > 3.9e-12,
        "unstable: {u32_:?}, {u64_:?}"
    );
    assert_eq!(lc_error::<f32>(0.0, true).ulps, 0.0, "R = (1, 0) is exact");
    let worst = c32.ulps.max(c64.ulps);
    assert_eq!(
        lc_verdict(conditioned, worst),
        Ok(Verdict::Pass),
        "at the threshold, the conditioned form"
    );
    assert_eq!(
        lc_verdict(conditioned, worst.next_down()),
        Ok(Verdict::Fail),
        "below it"
    );
    assert_eq!(lc_verdict("unstable", 1.0), Ok(Verdict::Fail));
    assert!(lc_verdict("other", 1.0)
        .unwrap_err()
        .contains("no form `other`"));
    gate::run(&root(), LC_NAME, &out("lc")).expect("each input its expected outcome");
}

#[test]
fn lc_inverse_gate() {
    check_lc("conditioned");
}

negative_control!(
    lc_inverse_gate,
    "the unstable form judged as the conditioned one",
    expected = "at the threshold, the conditioned form",
    check_lc("unstable")
);

/// A small `az-final-step` input: two rungs, against a reference run at `η = 0.005`.
fn final_step(landing: bool, order_above: f64) -> serde_json::Value {
    json!({
        "description": "a test input", "eta": 0.01, "n": 8, "landing": landing,
        "ladder": [0.02, 0.01], "reference_eta": 0.005, "order_above": order_above
    })
}

/// The figure-eight's closure and convergence at the gate's verdict: at the measured closure and below the measured
/// order it passes; just below the closure, or at the order, it fails; the closure without the landing fix is first
/// order and larger; a short ladder or no interval is refused; the orbit fixture is `ledger`'s IC.
fn check_final_step(landing: bool) {
    let o = orbit().expect("the orbit fixture");
    assert_eq!(figure_eight::<f64>(&o).p[2], o.v3, "unit masses: p₃ = v₃");
    let c = closure(&o, 0.01, 8, landing);
    let conv = convergence(&o, &[0.02, 0.01], 0.005, 8, landing);
    let off = convergence(&o, &[0.02, 0.01], 0.005, 8, false);
    assert!(
        conv.order > 1.5,
        "the landing fix's order {} is better than first",
        conv.order
    );
    assert!(off.order < 1.5, "without it: {}", off.order);
    let below = conv.order.next_down();
    let at = |threshold: f64, order_above: f64| {
        AzFinalStepGate
            .run(
                &config(FINAL_STEP_NAME, threshold),
                "t",
                &final_step(landing, order_above),
            )
            .map(|r| r.verdict)
    };
    assert_eq!(at(c.closure, below), Ok(Verdict::Pass), "at the closure");
    assert_eq!(
        at(c.closure.next_down(), below),
        Ok(Verdict::Fail),
        "below the closure"
    );
    assert_eq!(
        at(c.closure, conv.order),
        Ok(Verdict::Fail),
        "at the order, not above it"
    );
    for bad in [
        json!({ "description": "", "eta": 0.01, "n": 8, "landing": true, "ladder": [0.01],
                "reference_eta": 0.005, "order_above": 1.5 }),
        json!({ "description": "", "eta": 0.01, "n": 0, "landing": true, "ladder": [0.02, 0.01],
                "reference_eta": 0.005, "order_above": 1.5 }),
    ] {
        assert!(
            AzFinalStepGate
                .run(&config(FINAL_STEP_NAME, 1.0), "t", &bad)
                .is_err(),
            "{bad}"
        );
    }
}

#[test]
fn az_final_step_gate() {
    check_final_step(true);
}

negative_control!(
    az_final_step_gate,
    "the gate's arms run without the landing fix",
    expected = "the landing fix's order",
    check_final_step(false)
);

/// The gates' flags: `az-final-step` takes `--propose` and `--propose-sync` alone, `separation-floor` `--propose`
/// alone, `lc-inverse` none.
fn check_flags(name: &str, args: &[&str], reads: &str) {
    let e = gate::run_flags(&root(), name, args).expect_err("the flags were not refused");
    assert!(e.contains(reads), "{e}");
}

#[test]
fn az_gate_flags_refused() {
    check_flags(
        FINAL_STEP_NAME,
        &["--nothing"],
        "usage: --propose | --propose-sync",
    );
    check_flags(FINAL_STEP_NAME, &["--propose", "x"], "unrecognised flags");
    check_flags(FLOOR_NAME, &["--propose-sync"], "usage: --propose");
    check_flags(LC_NAME, &["--propose"], "has none of its own");
}

negative_control!(
    az_gate_flags_refused,
    "a flag the gate takes",
    expected = "the flags were not refused",
    check_flags(FLOOR_NAME, &["--propose"], "")
);

/// The `separation-floor` controls: a `TINY` above the passage's least pre-floor value floors steps there and passes,
/// the reference's floors none and fails, another is refused; the runner gives each its expected outcome.
fn check_floor(tiny: &str) {
    let GateSpec::Measurement(c) = gate::spec(&root(), FLOOR_NAME).expect("gate.json") else {
        panic!("not a measurement");
    };
    let control = |t: &str| {
        SeparationFloorGate
            .control(
                &c,
                "t",
                &json!({ "description": "a test input", "tiny": t }),
            )
            .map(|m| m.verdict)
    };
    assert_eq!(
        control(tiny),
        Ok(Verdict::Pass),
        "the floor binds above the least value"
    );
    assert_eq!(control("reference"), Ok(Verdict::Fail));
    assert!(control("other").unwrap_err().contains("no TINY `other`"));
    assert_eq!(radial().state.m, [1.0 / 3.0; 3]);
    gate::run(&root(), FLOOR_NAME, &out("floor")).expect("each control its expected outcome");
}

#[test]
fn separation_floor_controls() {
    check_floor("above");
}

negative_control!(
    separation_floor_controls,
    "the reference's TINY as the control that must bind",
    expected = "the floor binds above the least value",
    check_floor("reference")
);

/// A table of REQ-INT-080's measurement at the quanta `0.40625` (the coarse grid) and `0.1`, the `η` ladder
/// `[0.02, 0.01]`: `(error, rate)` at 0.1 for each rung, and the coarse grid's.
fn table(tenth: [(f64, f64); 2], coarse: [(f64, f64); 2]) -> Vec<SyncRow> {
    let mut t = Vec::new();
    for (delta, rows) in [(0.1, tenth), (0.40625, coarse)] {
        for (eta, (error, rate)) in [0.02, 0.01].into_iter().zip(rows) {
            t.push(SyncRow {
                delta,
                eta,
                error,
                rate,
            });
        }
    }
    t
}

/// A decision on a table, a ladder and the coarse grid: `(η, Δ_sync)`.
type Decide = fn(&[SyncRow], &[f64], f64) -> (f64, f64);

/// REQ-INT-080's decision at its criteria's edges: `η` the largest rung at most `1e-9` at 0.1, the last where none
/// is; 0.1 kept at exactly 10× the coarse error and 1.1× its cost, and not past either, a zero coarse error read as
/// `1e-16`; the round-up of REQ-INT-114's threshold.
fn check_decisions(decide: Decide) {
    let ladder = [0.02, 0.01];
    let at = |tenth, coarse| decide(&table(tenth, coarse), &ladder, 0.40625);
    let (ce, cr) = (1e-11, 100.0);
    let c = (ce, cr);
    let keep = (10.0 * ce, 1.1 * cr);
    assert_eq!(
        at([keep, keep], [c, c]),
        (0.02, 0.1),
        "at the edges, 0.1 kept"
    );
    assert_eq!(
        at([(SYNC_ERROR, keep.1), keep], [c, c]),
        (0.02, 0.2),
        "η at 1e-9; the error past 10×"
    );
    assert_eq!(
        at([(keep.0.next_up(), keep.1), keep], [c, c]),
        (0.02, 0.2),
        "just past 10×"
    );
    assert_eq!(
        at([(keep.0, keep.1.next_up()), keep], [c, c]),
        (0.02, 0.2),
        "just past 1.1×"
    );
    assert_eq!(
        at([(SYNC_ERROR.next_up(), 1.0), keep], [c, c]),
        (0.01, 0.1),
        "the first rung at most 1e-9"
    );
    assert_eq!(
        at([(2e-9, 1.0), (2e-9, 1.0)], [c, c]),
        (0.01, 0.2),
        "none at 1e-9: the last rung"
    );
    let zero = (0.0, cr);
    assert_eq!(
        at([(10.0 * 1e-16, 1.0), keep], [zero, c]),
        (0.02, 0.1),
        "a zero coarse error, as 1e-16"
    );
    assert_eq!(
        at([((10.0 * 1e-16f64).next_up(), 1.0), keep], [zero, c]),
        (0.02, 0.2),
        "past 10 × 1e-16"
    );
    assert_eq!(
        decide(&table([keep, keep], [c, c]), &ladder, 0.2),
        (0.02, 0.2),
        "no coarse row: 0.2"
    );
    assert_eq!(format!("{:.1e}", round_up(2.0 * 5.0253e-8)), "1.1e-7");
    assert_eq!(format!("{:.1e}", round_up(2.34e3)), "2.4e3");
    assert_eq!(round_up(0.5), 0.5);
}

#[test]
fn az_sync_decision() {
    check_decisions(decide_sync);
}

negative_control!(
    az_sync_decision,
    "a decision that always keeps 0.1",
    expected = "η at 1e-9; the error past 10×",
    check_decisions(|t, l, c| (decide_sync(t, l, c).0, 0.1))
);

/// The proposals' texts: REQ-INT-114's names its threshold, the IC's own closure and both ladders; REQ-INT-080's its
/// criteria, its decision and the window's mapping; REQ-INT-105's its fixtures, the doubly degenerate march and the
/// reference's values.
fn check_proposals(final_step: &str, sync: &str, floor: &str) {
    let o = orbit().expect("the orbit");
    let c = closure(&o, 0.01, 8, true);
    for needle in [
        format!("**Proposed threshold:** {:.1e}", round_up(2.0 * c.closure)),
        "the IC's own closure, Cartesian RK4 at dt = T/4096".to_owned(),
        "## The convergence ladder, with the landing fix (errors against η = 0.005, landing on)"
            .to_owned(),
        "## The convergence ladder, without it".to_owned(),
        "| 0.02 | ".to_owned(),
    ] {
        assert!(
            final_step.contains(&needle),
            "REQ-INT-114's proposal lacks `{needle}`:\n{final_step}"
        );
    }
    for needle in [
        "**Proposed:** η = 0.02; Δ_sync = 0.2. The closure window's 0.4 time units are 2.00 boundaries at Δ_sync = 0.2",
        "(0.4/Δ_sync at each quantum: 0.40625: 0.98, 0.2: 2.00, 0.1: 4.00)",
        "steps per unit time against the 0.40625 grid's: 0.40625: 1.00×, 0.2: ",
        "| 0.1 | 0.01 | 6.300000 | ",
    ] {
        assert!(sync.contains(needle), "REQ-INT-080's proposal lacks `{needle}`:\n{sync}");
    }
    for needle in [
        "**Proposed:** TINY = 1e-37 at f32 and 1e-300 at f64",
        "| the radial collision passage | f32 | ",
        "| test 7's close-encounter hierarchy | f64 | ",
        "| Burrau's (3, 4, 5) from rest | f32 | ",
        "TINY·TINY at the reference's values: 0e0 at f32",
        "(NonFinite, 1, 1)",
    ] {
        assert!(
            floor.contains(needle),
            "REQ-INT-105's proposal lacks `{needle}`:\n{floor}"
        );
    }
    assert_eq!(doubly_degenerate(), (End::NonFinite, 1, 1));
}

/// The three proposals, at a small configuration for the first two.
fn proposals() -> (String, String, String) {
    (
        propose_final_step_with(0.01, 8, &[0.02, 0.01], 0.005, 4096).expect("REQ-INT-114's"),
        propose_sync_with(&[0.02, 0.01], &[0.40625, 0.2, 0.1], 0.005).expect("REQ-INT-080's"),
        propose_floor().expect("REQ-INT-105's"),
    )
}

#[test]
fn az_proposals() {
    let (f, s, t) = proposals();
    check_proposals(&f, &s, &t);
}

negative_control!(
    az_proposals,
    "REQ-INT-080's text in REQ-INT-114's place",
    expected = "REQ-INT-114's proposal lacks",
    {
        let (_, s, t) = proposals();
        check_proposals(&s, &s, &t)
    }
);
