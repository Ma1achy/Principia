//! The steppers' emitter (`ledger::gen::steppers`; dd_integrator §3.2; REQ-INT-037):
//! - `steppers_generated_is_emitted`: the checked-in `crates/kernel/src/generated/steppers.rs` is the emitter's output,
//!   byte for byte, and the driver's emitters write it.
//! - `steppers_kicks`: the kick sequence of a drift sequence is `cᵢ = (d_{i−1} + dᵢ)/2` with `d₀ = d_{n+1} = 0`, its
//!   first, middle and last entries.
//! - `steppers_coefficients_derived`: each emitted coefficient is its register weight or the f64 expression dd §3.2
//!   writes, and the file holds each at f64 and at f32, the f64 value rounded once.

use ledger::constants::{YOSHIDA4_W1, YOSHIDA6_W1, YOSHIDA6_W2, YOSHIDA6_W3};
use ledger::gen::steppers::{coefficients, emit, kicks, steppers_file, STEPPERS_PATH};
use validation::negative_control;

/// The checked-in file.
fn on_disk() -> String {
    std::fs::read_to_string(format!(
        "{}/../../{STEPPERS_PATH}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("read the generated steppers file")
}

/// Checks that `file` is the emitter's output, and that the driver's emitters write the one path.
fn check_emitted(file: &str) {
    let emitted = emit(&[], &[]);
    assert_eq!(emitted.len(), 1, "the emitter writes one file");
    assert_eq!(
        emitted[0].path.to_str(),
        Some(STEPPERS_PATH),
        "the emitted path"
    );
    assert!(
        emitted[0].contents == file,
        "the checked-in steppers.rs is not the emitter's output"
    );
    let all = ledger::gen::generate(&ledger::layout(), ledger::gen::EMITTERS).expect("generates");
    assert!(
        all.iter().any(|g| g.path.to_str() == Some(STEPPERS_PATH)),
        "the driver's emitters do not write {STEPPERS_PATH}"
    );
}

#[test]
fn steppers_generated_is_emitted() {
    check_emitted(&on_disk());
}

negative_control!(
    steppers_generated_is_emitted,
    "a checked-in file with one digit of a weight changed",
    expected = "the checked-in steppers.rs is not the emitter's output",
    check_emitted(&on_disk().replace("1.3512071919596575", "1.3512071919596576"))
);

/// Checks `kicks` against the sequence written out for `d`.
fn check_kicks(d: &[f64], want: &[f64]) {
    assert_eq!(kicks(d), want, "the kicks of {d:?}");
}

#[test]
fn steppers_kicks() {
    check_kicks(&[1.0], &[0.5, 0.5]);
    check_kicks(&[2.0, 4.0, 8.0], &[1.0, 3.0, 6.0, 4.0]);
}

negative_control!(
    steppers_kicks,
    "a kick sequence whose last entry is the last drift, not half of it",
    expected = "the kicks of",
    check_kicks(&[2.0, 4.0, 8.0], &[1.0, 3.0, 6.0, 8.0])
);

/// The coefficients the emitter must write: each name with its f64 value, from the register's weights.
fn derived() -> Vec<(&'static str, f64)> {
    let w1 = YOSHIDA4_W1.number();
    let w0 = 1.0 - 2.0 * w1;
    let (v1, v2, v3) = (
        YOSHIDA6_W1.number(),
        YOSHIDA6_W2.number(),
        YOSHIDA6_W3.number(),
    );
    let v0 = 1.0 - 2.0 * (v1 + v2 + v3);
    vec![
        ("YOSHIDA4_W1", w1),
        ("YOSHIDA4_W0", w0),
        ("YOSHIDA4_KICK_1", w1 / 2.0),
        ("YOSHIDA4_KICK_2", (w1 + w0) / 2.0),
        ("YOSHIDA6_W1", v1),
        ("YOSHIDA6_W2", v2),
        ("YOSHIDA6_W3", v3),
        ("YOSHIDA6_W0", v0),
        ("YOSHIDA6_KICK_1", v3 / 2.0),
        ("YOSHIDA6_KICK_2", (v3 + v2) / 2.0),
        ("YOSHIDA6_KICK_3", (v2 + v1) / 2.0),
        ("YOSHIDA6_KICK_4", (v1 + v0) / 2.0),
    ]
}

/// Checks the emitted coefficients against `want`, by name and bits, and that `file` declares each and gives it at
/// f64 and at f32.
fn check_derived(want: &[(&str, f64)], file: &str) {
    let got = coefficients();
    assert_eq!(got.len(), want.len(), "the coefficients emitted");
    for (k, &(name, v)) in got.iter().zip(want) {
        assert_eq!(k.name, name, "the coefficients' order");
        assert_eq!(
            k.value.to_bits(),
            v.to_bits(),
            "{name} is not dd §3.2's expression in f64"
        );
        assert!(
            file.contains(&format!("    const {name}: Self;\n")),
            "{name} is not declared"
        );
        assert!(
            file.contains(&format!("    const {name}: f64 = {v:?};\n")),
            "{name} at f64 is not {v:?}"
        );
        assert!(
            file.contains(&format!("    const {name}: f32 = {:?};\n", v as f32)),
            "{name} at f32 is not {:?}",
            v as f32
        );
    }
}

#[test]
fn steppers_coefficients_derived() {
    check_derived(&derived(), &steppers_file(&coefficients()));
}

negative_control!(
    steppers_coefficients_derived,
    "Yoshida-6's w₀ taken as 1 − (w₁ + w₂ + w₃)",
    expected = "YOSHIDA6_W0 is not dd §3.2's expression in f64",
    {
        let mut want = derived();
        want[7].1 = 1.0 - (YOSHIDA6_W1.number() + YOSHIDA6_W2.number() + YOSHIDA6_W3.number());
        check_derived(&want, &steppers_file(&coefficients()))
    }
);
