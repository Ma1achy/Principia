//! QA tests for TASK-M3-05 on the expected-colour side of the drift views, `Look::colour` (R-79, R-436; render contract
//! Part 5, "Presentation layer"), written from those texts and not from the implementation.
//! - A non-finite operand of `Look::Drift` draws the invalid hatch at every pixel, with or without a lit predicate:
//!   a finite `x` with a non-finite `reference`, an infinite `x`, and a NaN `x` whatever its bits
//!   (`qa_look_drift_non_finite_operand_draws_the_hatch`).
//! - Finite operands stay at their place on the `diverging` ramp, so the check above isn't met by hatching everything
//!   (`qa_look_drift_finite_operands_keep_their_place`).

use render::present::{self, Rgb};
use validation::golden_scene::Look;
use validation::negative_control;

/// The drift's floor in these tests; the place below is written out from it.
const EPS: f64 = 1e-6;
/// Past the hatch's and the suspect styling's periods, so that each stripe and each line is met.
const SIDE: u32 = 16;

/// The absence NaN (R-79), whose hatch `dbg_drift` draws by itself: the case the rest are not.
const ABSENT: u32 = 0x7fc0_0000;
/// NaNs other than the absence NaN: its sign set, a quiet payload, a signalling one, and both of the last with the sign.
const OTHER_NANS: [u32; 5] = [
    0xffc0_0000,
    0x7fc0_0001,
    0x7f80_0001,
    0xffc0_0001,
    0xff80_0001,
];

/// Every non-finite operand under test, by name.
fn non_finite() -> Vec<(String, f32)> {
    let mut all = vec![
        ("+inf".to_owned(), f32::INFINITY),
        ("-inf".to_owned(), f32::NEG_INFINITY),
        (format!("NaN {ABSENT:#010x}"), f32::from_bits(ABSENT)),
    ];
    all.extend(OTHER_NANS.map(|bits| (format!("NaN {bits:#010x}"), f32::from_bits(bits))));
    all
}

/// The cases as `(what, x, reference)`: each non-finite `reference` under finite `x`, of each sign and past the band;
/// each non-finite `x` over a finite `reference`; each non-finite `x` over each non-finite `reference`.
fn cases() -> Vec<(String, f32, f32)> {
    let mut all = Vec::new();
    for (name, v) in non_finite() {
        for x in [0.25_f32, -0.25, 0.0] {
            all.push((format!("x = {x}, reference {name}"), x, v));
        }
        for reference in [1.0_f32, 0.0] {
            all.push((format!("x {name}, reference = {reference}"), v, reference));
        }
        for (other, w) in non_finite() {
            all.push((format!("x {name}, reference {other}"), v, w));
        }
    }
    all
}

fn drift(x: f32, reference: f32, energy: bool, lz: bool) -> Look {
    Look::Drift {
        x,
        reference,
        lin_r: 1.0,
        eps: EPS,
        energy,
        lz,
    }
}

/// `colour` is the path under test. Every case is judged before the check fails, so the failure lists them all, each
/// at its first pixel off the hatch and once whatever its predicates.
fn check_non_finite(colour: impl Fn(Look, [u32; 2]) -> Rgb) {
    let mut failures = Vec::new();
    for (what, x, reference) in cases() {
        'case: for (energy, lz) in [(false, false), (true, false), (false, true), (true, true)] {
            for py in 0..SIDE {
                for px in 0..SIDE {
                    let got = colour(drift(x, reference, energy, lz), [px, py]);
                    let want = present::debug_invalid([f64::from(px) + 0.5, f64::from(py) + 0.5]);
                    if got != want {
                        failures.push(format!(
                            "{what}, energy {energy}, lz {lz}: pixel ({px}, {py}) is {got:?}, the hatch {want:?}"
                        ));
                        break 'case;
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "not invalid:\n{}", failures.join("\n"));
}

/// The look as it would be with no non-finite guard: the ramp, the suspect styling over it.
#[cfg(feature = "controls")]
fn unguarded(look: Look, [px, py]: [u32; 2]) -> Rgb {
    let Look::Drift {
        x,
        reference,
        lin_r,
        eps,
        energy,
        lz,
    } = look
    else {
        panic!("the control takes a drift look, not {look:?}");
    };
    let frag = [f64::from(px) + 0.5, f64::from(py) + 0.5];
    present::dbg_suspect(
        present::dbg_drift(
            present::dbg_drift_shown(x, present::DRIFT_SYMLOG, reference),
            present::DRIFT_SYMLOG,
            false,
            lin_r,
            eps,
            frag,
        ),
        x,
        energy,
        lz,
        frag,
    )
}

#[test]
fn qa_look_drift_non_finite_operand_draws_the_hatch() {
    check_non_finite(Look::colour);
}

negative_control!(
    qa_look_drift_non_finite_operand_draws_the_hatch,
    "the ramp and the suspect styling with no non-finite guard",
    expected = "not invalid",
    check_non_finite(unguarded)
);

/// `symlog`'s place on `diverging` for a drift past the band, written out from render contract Part 5.
fn place(x: f64) -> f64 {
    let s = (1.0 + x.abs() / EPS).ln();
    let signed = if x < 0.0 { -s } else { s };
    0.5 + 0.5 * signed / (1.0 + signed.abs())
}

/// `colour` is the path under test. No predicate is lit, so each pixel shows the ramp alone.
fn check_finite(colour: impl Fn(Look, [u32; 2]) -> Rgb) {
    let mut failures = Vec::new();
    for (x, reference) in [(0.25_f32, 1.0_f32), (-0.25, 1.0), (0.25, 0.0)] {
        let want = present::drift_ramp(place(f64::from(x)), false);
        for pixel in [[0, 0], [1, 0], [0, 1], [SIDE - 1, SIDE - 1]] {
            let got = colour(drift(x, reference, false, false), pixel);
            if got.iter().zip(want).any(|(g, w)| (g - w).abs() > 1e-12) {
                failures.push(format!(
                    "x = {x}, reference = {reference}: pixel {pixel:?} is {got:?}, its place {want:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "off its place:\n{}",
        failures.join("\n")
    );
}

#[test]
fn qa_look_drift_finite_operands_keep_their_place() {
    check_finite(Look::colour);
}

negative_control!(
    qa_look_drift_finite_operands_keep_their_place,
    "a guard that hatches every sample",
    expected = "off its place",
    check_finite(|_, [px, py]| present::debug_invalid([f64::from(px) + 0.5, f64::from(py) + 0.5]))
);
