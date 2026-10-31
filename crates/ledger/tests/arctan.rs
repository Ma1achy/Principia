//! The arctangent's emitter (`ledger::gen::arctan`; RQ-628, decided by the orchestrator per R-369, option (a)):
//! - `arctan_generated_is_emitted`: the checked-in `crates/kernel/src/generated/arctan.rs` is the emitter's output,
//!   byte for byte, and the driver's emitters write it.
//! - `arctan_numbers`: each emitted number is what it names: `atan(k/8)` in Q0.64, `π/2` and `π` in Q2.62, within
//!   f64's own error of f64's `atan` and `π` (the integer sums are far finer, so the f64 value is the coarser of the
//!   two); `atan 0 = 0` and `ATAN_EIGHTHS[8] = π/4` exactly, the generator's `π/4`; and `1/(2j + 3)` in Q0.32, rounded
//!   to nearest.

use ledger::constants::ARCTAN_SUM_BITS;
use ledger::gen::arctan::{
    arctan_file, atan_eighth, eighth_q64, emit, half_pi_q62, pi_q62, series_q32, series_terms,
    sum_bits, ARCTAN_PATH,
};
use validation::negative_control;

/// The checked-in file.
fn on_disk() -> String {
    std::fs::read_to_string(format!(
        "{}/../../{ARCTAN_PATH}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("read the generated arctan file")
}

/// Checks that `file` is the emitter's output, and that the driver's emitters write the one path.
fn check_emitted(file: &str) {
    let emitted = emit(&[], &[]);
    assert_eq!(emitted.len(), 1, "the emitter writes one file");
    assert_eq!(
        emitted[0].path.to_str(),
        Some(ARCTAN_PATH),
        "the emitted path"
    );
    assert!(
        emitted[0].contents == file,
        "the checked-in arctan.rs is not the emitter's output"
    );
    assert_eq!(
        arctan_file(),
        emitted[0].contents,
        "the file is arctan_file()"
    );
    let all = ledger::gen::generate(&ledger::layout(), ledger::gen::EMITTERS).expect("generates");
    assert!(
        all.iter().any(|g| g.path.to_str() == Some(ARCTAN_PATH)),
        "the driver's emitters do not write {ARCTAN_PATH}"
    );
}

#[test]
fn arctan_generated_is_emitted() {
    check_emitted(&on_disk());
}

negative_control!(
    arctan_generated_is_emitted,
    "a checked-in file with one digit of π/4 changed",
    expected = "the checked-in arctan.rs is not the emitter's output",
    check_emitted(&on_disk().replace("0x2168_C235", "0x2168_C236"))
);

/// The emitted numbers, as `(atan(k/8) in Q0.64 for k = 0 … 8, π/2 and π in Q2.62, 1/(2j + 3) in Q0.32)`.
type Numbers = (Vec<u64>, u64, u64, Vec<u32>);

/// The emitter's numbers.
fn emitted() -> Numbers {
    (
        (0..=8).map(eighth_q64).collect(),
        half_pi_q62(),
        pi_q62(),
        (0..series_terms()).map(series_q32).collect(),
    )
}

/// `v` in units of `2^-bits`, as f64.
fn scaled(v: u64, bits: i32) -> f64 {
    v as f64 * 2f64.powi(-bits)
}

/// Checks `numbers` against what they name (the module's list).
fn check_numbers((eighths, half_pi, pi, series): Numbers) {
    assert_eq!(eighths.len(), 9, "nine breakpoints");
    assert_eq!(eighths[0], 0, "atan 0 is 0");
    let quarter = atan_eighth(8);
    assert_eq!(
        eighths[8],
        ((quarter + (1 << 35)) >> 36) as u64,
        "atan 1 is the generator's π/4"
    );
    assert!(sum_bits() == 100, "the sums' units are 2⁻¹⁰⁰");
    assert_eq!(
        f64::from(sum_bits()),
        ARCTAN_SUM_BITS.number(),
        "the sums' units are the register's"
    );
    for (k, &q) in eighths.iter().enumerate() {
        let want = (k as f64 / 8.0).atan();
        let got = scaled(q, 64);
        assert!(
            (got - want).abs() <= 2.0 * f64::EPSILON * want.max(f64::MIN_POSITIVE),
            "atan({k}/8) = {got}, not {want}"
        );
    }
    let (h, p) = (scaled(half_pi, 62), scaled(pi, 62));
    assert!(
        (h - core::f64::consts::FRAC_PI_2).abs() <= f64::EPSILON,
        "π/2 = {h}"
    );
    assert!(
        (p - core::f64::consts::PI).abs() <= 2.0 * f64::EPSILON,
        "π = {p}"
    );
    assert_eq!(series.len(), 5, "five series terms");
    for (j, &c) in series.iter().enumerate() {
        let d = 2 * j as u64 + 3;
        let want = ((1u64 << 32) + d / 2) / d;
        assert_eq!(u64::from(c), want, "1/{d} in Q0.32");
        assert!(
            (scaled(u64::from(c), 32) - 1.0 / d as f64).abs() <= 2f64.powi(-33),
            "1/{d} rounded to nearest"
        );
    }
}

#[test]
fn arctan_numbers() {
    check_numbers(emitted());
}

negative_control!(
    arctan_numbers,
    "atan(3/8) off by one unit of 2⁻⁴⁰",
    expected = "atan(3/8) = ",
    {
        let (mut e, h, p, s) = emitted();
        e[3] += 1 << 24;
        check_numbers((e, h, p, s))
    }
);
