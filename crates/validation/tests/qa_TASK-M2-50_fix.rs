//! QA tests for TASK-M2-50's fix round, written from REQ-VAL-193's statement ("its max ‖Δz‖ reported until
//! REQ-VAL-121 calibrates its bound and gated at that bound after") and its verify ("once REQ-VAL-121's bound for the
//! arm is recorded, the arm fails above it"), end to end through the gate runner, `gate::run`, the path
//! `cargo xtask gate inspector-handles` takes: a copy of the repository's `requirements.yaml` and the committed
//! `inspector-handles/gate.json` with only `randomise_bound`'s value changed, and one small input with the Randomise
//! arm and no leak, so REQ-VAL-021's own arm passes and any failure is the Randomise arm's.
//!
//! - a bound recorded far below any rounding error (10⁻³⁰) fails the gate; the control: the same input under the
//!   committed null bound is report-only and passes, so an expected `fail` is wrong;
//! - a bound recorded far above the arm (1) passes; the control: the 10⁻³⁰ bound with an expected `pass` is wrong.
//!
//! Each test has its negative control (R-176).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use validation::gate;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs `inspector-handles` on a scratch root named `tag`: the committed `gate.json` with `randomise_bound.value` set
/// to `bound`, and one input with the Randomise arm, expecting `expected`. Returns the runner's result.
fn run_with(tag: &str, bound: Value, expected: &str) -> Result<(), String> {
    let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m2_50_fix_{tag}"));
    let _ = std::fs::remove_dir_all(&copy);
    let dir = copy.join("fixtures/gates/inspector-handles");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(copy.join("plan")).unwrap();
    std::fs::copy(
        root().join("plan/requirements.yaml"),
        copy.join("plan/requirements.yaml"),
    )
    .unwrap();
    let committed = root().join("fixtures/gates/inspector-handles/gate.json");
    let mut config: Value =
        serde_json::from_str(&std::fs::read_to_string(committed).unwrap()).unwrap();
    assert_eq!(
        config["randomise_bound"]["requirement"], "REQ-VAL-121",
        "the committed gate.json names no REQ-VAL-121 bound for the Randomise arm"
    );
    config["randomise_bound"]["value"] = bound;
    config["inputs"] = json!(["arm.json"]);
    std::fs::write(dir.join("gate.json"), config.to_string()).unwrap();
    let input = json!({
        "description": "qa: the Randomise arm under a set bound, REQ-VAL-021's arm leak-free",
        "seed": 1,
        "ics": 200,
        "radius": 4.0,
        "span": 1.0,
        "octaves": 4.0,
        "leak": "none",
        "randomise": { "seeds": [1, 2], "ics": 400, "fed": false },
        "expected": expected,
    });
    std::fs::write(dir.join("arm.json"), input.to_string()).unwrap();
    gate::run(&copy, "inspector-handles", &copy.join("out"))
}

/// A recorded bound below the arm's worst fails the gate; `bound` is the recorded value.
fn check_fails_above(tag: &str, bound: Value) {
    let r = run_with(tag, bound, "fail");
    assert!(
        r.is_ok(),
        "the arm above its recorded bound does not fail the gate: {r:?}"
    );
}

#[test]
fn qa_randomise_arm_fails_above_recorded_bound() {
    check_fails_above("low", json!(1e-30));
}

negative_control!(
    qa_randomise_arm_fails_above_recorded_bound,
    "with the bound null the arm is reported only and must not fail the gate",
    expected = "the arm above its recorded bound does not fail the gate",
    check_fails_above("null", Value::Null)
);

/// A recorded bound above the arm's worst passes the gate; `bound` is the recorded value.
fn check_passes_below(tag: &str, bound: Value) {
    let r = run_with(tag, bound, "pass");
    assert!(
        r.is_ok(),
        "the arm within its recorded bound fails the gate: {r:?}"
    );
}

#[test]
fn qa_randomise_arm_passes_within_recorded_bound() {
    check_passes_below("high", json!(1.0));
}

negative_control!(
    qa_randomise_arm_passes_within_recorded_bound,
    "a bound below every rounding error must fail the arm",
    expected = "the arm within its recorded bound fails the gate",
    check_passes_below("low_pass", json!(1e-30))
);
