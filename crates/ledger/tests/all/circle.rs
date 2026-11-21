//! The circle's emitter (`ledger::gen::circle`), the numbers of the kernel's own reduction of the decode's `β`:
//! - `circle_generated_is_emitted`: the checked-in `crates/kernel/src/generated/circle.rs` is the emitter's output,
//!   byte for byte, and the driver's emitters write it.
//! - `circle_numbers`: `π` summed by Machin's formula agrees with f64's `π` to f64's own error; `PI_HAT_BITS` is the bits
//!   of `f32`'s `π` (Rust's literal, the f32 nearest); and `INV_TWO_PI`'s words are `1/2π`'s: word 0 is 0 and words 1
//!   to 7 read as a fraction lie within `2⁻⁵²` relative of f64's `1/2π` (f64's error dominates), and times `2π` at the
//!   emitter's own precision fall short of `2²²⁴` by less than `2π` (the floor).
//!
//! (`crates/kernel/tests/circle.rs` recomputes the table independently, with `num-bigint`.)

use ledger::gen::circle::{
    circle_file, emit, inv_two_pi, inv_two_pi_words, machin_pi, pi_bits, pi_hat_bits, Big,
    CIRCLE_PATH,
};
use validation::negative_control;

/// The checked-in file.
fn on_disk() -> String {
    std::fs::read_to_string(format!("{}/../../{CIRCLE_PATH}", crate::manifest_dir()))
        .expect("read the generated circle file")
}

/// Checks that `file` is the emitter's output, and that the driver's emitters write the one path.
fn check_emitted(file: &str) {
    let emitted = emit(&[], &[]);
    assert_eq!(emitted.len(), 1, "the emitter writes one file");
    assert_eq!(
        emitted[0].path.to_str(),
        Some(CIRCLE_PATH),
        "the emitted path"
    );
    assert!(
        emitted[0].contents == file,
        "the checked-in circle.rs is not the emitter's output"
    );
    assert_eq!(
        circle_file(),
        emitted[0].contents,
        "the file is circle_file()"
    );
    let all = ledger::gen::generate(&ledger::layout(), ledger::gen::EMITTERS).expect("generates");
    assert!(
        all.iter().any(|g| g.path.to_str() == Some(CIRCLE_PATH)),
        "the driver's emitters do not write {CIRCLE_PATH}"
    );
}

#[test]
fn circle_generated_is_emitted() {
    check_emitted(&on_disk());
}

negative_control!(
    circle_generated_is_emitted,
    "a checked-in file with one digit of 1/2π changed",
    expected = "the checked-in circle.rs is not the emitter's output",
    check_emitted(&on_disk().replace("0x28BE_60DB", "0x28BE_60DC"))
);

/// The top 64 bits of `b` from bit `top` down, as an f64 times `2^(top − 63)`.
fn as_f64(b: &Big, top: u32) -> f64 {
    let mut v = 0u64;
    for k in 0..64 {
        let bit = top - k;
        v = (v << 1) | u64::from((b.word((bit / 32) as usize) >> (bit % 32)) & 1);
    }
    v as f64 * 2f64.powi(top as i32 - 63)
}

/// The emitted numbers against f64's `π` and `1/2π`, and the table's floor exactly, with `words` as the table.
fn check_numbers(words: &[u32], hat: u32) {
    let pi = as_f64(&machin_pi(), pi_bits() + 1) * 2f64.powi(-(pi_bits() as i32));
    assert!(
        (pi - std::f64::consts::PI).abs() <= 4.0 * f64::EPSILON,
        "Machin's π {pi:e} is not f64's"
    );
    assert_eq!(
        hat,
        std::f32::consts::PI.to_bits(),
        "PI_HAT_BITS is not the f32 nearest π"
    );
    assert_eq!(words.len(), 8, "the table's words");
    assert_eq!(words[0], 0, "the table's word 0 is not 0");
    let frac = words[1..]
        .iter()
        .enumerate()
        .map(|(k, &w)| f64::from(w) * 2f64.powi(-32 * (k as i32 + 1)))
        .sum::<f64>();
    let want = 1.0 / (2.0 * std::f64::consts::PI);
    assert!(
        ((frac - want) / want).abs() <= 2.0 * f64::EPSILON,
        "the table {frac:e} is not 1/2π {want:e}"
    );
    let mut q = Big::zero();
    for &w in &words[1..] {
        for b in (0..32).rev() {
            q = q.double();
            if (w >> b) & 1 == 1 {
                q = q.add(&Big::pow2(0));
            }
        }
    }
    assert_eq!(q, inv_two_pi(), "the table's words are not ⌊2²²⁴/2π⌋'s");
    let two_pi = machin_pi().double();
    let mut product = Big::zero();
    for (k, &w) in words[1..].iter().rev().enumerate() {
        let mut part = two_pi.mul_small(w);
        for _ in 0..32 * k {
            part = part.double();
        }
        product = product.add(&part);
    }
    let whole = Big::pow2(224 + pi_bits());
    assert!(whole.at_least(&product), "the table times 2π exceeds 2²²⁴");
    assert!(
        !whole
            .sub(&product)
            .at_least(&two_pi.add(&Big::pow2(pi_bits()))),
        "the table times 2π falls short of 2²²⁴ by 2π or more: not the floor"
    );
}

#[test]
fn circle_numbers() {
    check_numbers(&inv_two_pi_words(), pi_hat_bits());
}

negative_control!(
    circle_numbers,
    "the table with its last word one low",
    expected = "the table's words are not",
    {
        let mut w = inv_two_pi_words();
        w[7] -= 1;
        check_numbers(&w, pi_hat_bits())
    }
);
