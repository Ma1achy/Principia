//! qa for TASK-M3-72: the pre-registered (a1)/(a2) criterion (the task file's § "The (a1)/(a2) criterion", written
//! before any run; R-649, R-974; REQ-VAL-038), checked rule by rule on synthetic inputs, so the choice the
//! `shadow-form` gate reports is the criterion's and nothing else.
//! - `qa_m372_criterion_rules`: rule 1, where exactly one of (a1) and (a2) stays finite on Burrau, that one, whatever
//!   the worst gaps; rule 2, otherwise (a2) where `W(a1) − W(a2) ≥ 1/T` (the boundary itself included); rule 3,
//!   otherwise (a1), both worst gaps infinite included; (a1) infinite and (a2) finite chooses (a2) by rule 2. Control:
//!   a choice that skips rule 1 and reads the gaps alone fails.
//! - `qa_m372_worst_gap`: `W` is the largest `|g|` over the rows, `g = λ_T(f32) − λ_T(f64)` in the form's own column,
//!   a negative gap counted by its magnitude, and a gap that is not finite (an f32 or f64 `λ_T` NaN or infinite)
//!   counts as infinite. Control: a worst gap read as the largest signed gap fails.
//! - `qa_m372_form_words`: the fixtures' form words parse to their forms, and a word naming no form ("displacement",
//!   the name TASK-M3-14's stand-in had) is refused. Control: accepting it fails.

use validation::measure::benettin::{choose, worst_gap, Form, FormRow};
use validation::negative_control;

/// The horizon the criterion's worst gaps are read over, `T` = 30 (the task file).
const T: f64 = 30.0;

type Choose = fn([f64; 2], [bool; 2], f64) -> Form;

fn check_rules(choose: Choose) {
    let step = 1.0 / T;
    // Rule 1: exactly one finite on Burrau.
    assert_eq!(
        choose([0.0, 5.0], [false, true], T),
        Form::CancellationFree,
        "rule 1: only (a2) finite on Burrau, (a2) chosen whatever the gaps"
    );
    assert_eq!(
        choose([5.0, 0.0], [true, false], T),
        Form::Plain,
        "rule 1: only (a1) finite on Burrau, (a1) chosen whatever the gaps"
    );
    // Rule 2, both finite and both not finite on Burrau alike ("otherwise").
    for burrau in [[true, true], [false, false]] {
        assert_eq!(
            choose([step, 0.0], burrau, T),
            Form::CancellationFree,
            "rule 2: W(a1) − W(a2) = 1/T exactly chooses (a2) (Burrau {burrau:?})"
        );
        assert_eq!(
            choose([2.0 * step, 0.0], burrau, T),
            Form::CancellationFree,
            "rule 2: W(a1) − W(a2) > 1/T chooses (a2) (Burrau {burrau:?})"
        );
        assert_eq!(
            choose([f64::INFINITY, 0.1], burrau, T),
            Form::CancellationFree,
            "rule 2: W(a1) infinite, W(a2) finite chooses (a2) (Burrau {burrau:?})"
        );
        // Rule 3.
        assert_eq!(
            choose([0.5 * step, 0.0], burrau, T),
            Form::Plain,
            "rule 3: a gain below 1/T chooses (a1) (Burrau {burrau:?})"
        );
        assert_eq!(
            choose([0.1, 0.4], burrau, T),
            Form::Plain,
            "rule 3: (a2) worse chooses (a1) (Burrau {burrau:?})"
        );
        assert_eq!(
            choose([f64::INFINITY, f64::INFINITY], burrau, T),
            Form::Plain,
            "rule 3: both worst gaps infinite choose (a1) (Burrau {burrau:?})"
        );
    }
    // The task file's recorded run: W(a1) 0.00017, W(a2) 0.00011, both finite on Burrau: (a1).
    assert_eq!(
        choose([0.00017, 0.00011], [true, true], T),
        Form::Plain,
        "the recorded run chooses (a1)"
    );
}

#[test]
fn qa_m372_criterion_rules() {
    check_rules(choose);
}

/// A choice by the gaps alone, rule 1 skipped.
#[cfg(feature = "controls")]
fn gaps_only(w: [f64; 2], _burrau: [bool; 2], horizon: f64) -> Form {
    if w[0] - w[1] >= 1.0 / horizon {
        Form::CancellationFree
    } else {
        Form::Plain
    }
}

negative_control!(
    qa_m372_criterion_rules,
    "a choice that skips the Burrau rule must fail",
    expected = "rule 1",
    check_rules(gaps_only)
);

/// A row whose gap in the form `form` is `f32 − f64`, the other forms' gaps zero.
fn row(form: Form, f32_l: f64, f64_l: f64) -> FormRow {
    let k = Form::ALL.iter().position(|&f| f == form).unwrap();
    let mut lambdas = [[0.25, 0.25]; 3];
    lambdas[k] = [f32_l, f64_l];
    FormRow {
        orbit: "synthetic",
        known: 0.25,
        lambdas,
    }
}

type Worst = fn(&[FormRow], Form) -> f64;

fn check_worst(worst: Worst) {
    for form in Form::ALL {
        let rows = [
            row(form, 0.30, 0.20),
            row(form, 0.10, 0.40),
            row(form, 0.25, 0.25),
        ];
        let w = worst(&rows, form);
        assert!(
            (w - 0.30).abs() < 1e-12,
            "{}: W = {w}, not the largest |gap| 0.30 (a negative gap counts by its magnitude)",
            form.word()
        );
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for rows in [
                [row(form, 0.30, 0.20), row(form, bad, 0.4)],
                [row(form, 0.30, 0.20), row(form, 0.4, bad)],
            ] {
                let w = worst(&rows, form);
                assert!(
                    w == f64::INFINITY,
                    "{}: a gap with a λ_T of {bad} gives W = {w}, not infinite",
                    form.word()
                );
            }
        }
    }
}

#[test]
fn qa_m372_worst_gap() {
    check_worst(worst_gap);
}

/// The largest signed gap.
#[cfg(feature = "controls")]
fn signed(rows: &[FormRow], form: Form) -> f64 {
    rows.iter()
        .map(|r| r.gap(form))
        .fold(f64::NEG_INFINITY, f64::max)
}

negative_control!(
    qa_m372_worst_gap,
    "a worst gap read as the largest signed gap must fail",
    expected = "not the largest |gap|",
    check_worst(signed)
);

type Parse = fn(&str) -> Result<Form, String>;

fn check_words(parse: Parse) {
    assert_eq!(parse("absolute"), Ok(Form::Absolute));
    assert_eq!(parse("plain"), Ok(Form::Plain));
    assert_eq!(parse("cancellation-free"), Ok(Form::CancellationFree));
    for word in ["displacement", "", "Plain", "a1"] {
        assert!(
            parse(word).is_err(),
            "the form word `{word}` names no form and is accepted"
        );
    }
}

#[test]
fn qa_m372_form_words() {
    check_words(Form::parse);
}

/// A parse that reads every unknown word as the absolute form.
#[cfg(feature = "controls")]
fn lenient(word: &str) -> Result<Form, String> {
    Ok(Form::parse(word).unwrap_or(Form::Absolute))
}

negative_control!(
    qa_m372_form_words,
    "a parse that accepts a word naming no form must fail",
    expected = "names no form and is accepted",
    check_words(lenient)
);
