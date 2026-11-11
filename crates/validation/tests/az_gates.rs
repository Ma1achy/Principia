//! Aarseth–Zare's gates (TASK-M3-68): `lc-inverse` (REQ-INT-044), `az-final-step` (REQ-INT-049, REQ-INT-114) and the
//! `separation-floor` measurement (REQ-INT-105): their verdicts at and past their thresholds, their refusals, their
//! flags, and the runner on their fixtures. Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use serde_json::json;
use validation::az::{
    closure, convergence, figure_eight, lc_error, orbit, radial, AzFinalStepGate, LcInverseGate,
    SeparationFloorGate, FINAL_STEP_NAME, FLOOR_NAME, LC_NAME,
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
