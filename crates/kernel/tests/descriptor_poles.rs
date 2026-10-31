//! The descriptor's pole values, selected, never divided (R-436; dd_decoder §3.6, "Each producer selects its pole
//! value"; `kernel::decode::descriptor`, `kernel::real::IeeeBits`), natively at f32 and f64:
//! - `ieee_bits`: `or_pos_inf` and `or_neg_inf` give `±∞`'s integer pattern where their condition holds and `self`'s
//!   bits, unchanged, where it doesn't; `non_finite_bits` is true exactly on an exponent field of all ones.
//! - `descriptor_poles`: on `(m, r, p)` built at each pole, `rho_ratio` stores `+∞`'s bits where `‖λ̃‖ = 0`, `V_0`
//!   `−∞`'s wherever a pair separation is 0, and `virial_ratio` 0 there; one step off each pole the quotient is stored,
//!   finite (REQ-TOOL-030).

use kernel::decode::descriptor::{describe, potential, ratio, virial};
use kernel::decode::Ic;
use kernel::generated::links::LinkReal;
use kernel::payload::{ICDescriptorOf, PayloadReal};
use kernel::real::IeeeBits;
use validation::negative_control;

/// The patterns at one width: `+∞`, `−∞`, and the exponent mask.
trait Bits: LinkReal + IeeeBits + PayloadReal {
    const POS: u64;
    const NEG: u64;
    fn bits(self) -> u64;
    fn of(x: f64) -> Self;
    fn of_bits(bits: u64) -> Self;
    /// The largest finite value's bits, and the smallest positive subnormal's.
    const MAX: u64;
    const TINY: u64;
}

impl Bits for f32 {
    const POS: u64 = 0x7F80_0000;
    const NEG: u64 = 0xFF80_0000;
    const MAX: u64 = 0x7F7F_FFFF;
    const TINY: u64 = 1;
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
    fn of(x: f64) -> Self {
        x as f32
    }
    fn of_bits(bits: u64) -> Self {
        f32::from_bits(bits as u32)
    }
}

impl Bits for f64 {
    const POS: u64 = 0x7FF0_0000_0000_0000;
    const NEG: u64 = 0xFFF0_0000_0000_0000;
    const MAX: u64 = 0x7FEF_FFFF_FFFF_FFFF;
    const TINY: u64 = 1;
    fn bits(self) -> u64 {
        self.to_bits()
    }
    fn of(x: f64) -> Self {
        x
    }
    fn of_bits(bits: u64) -> Self {
        f64::from_bits(bits)
    }
}

/// A function `(x, c) → x or ±∞`.
type Select<R> = fn(R, bool) -> R;

/// `pos` and `neg` select `+∞` and `−∞` where `c` and keep every other value's bits, signed zeros and a NaN payload
/// included; `non_finite` holds on all-ones exponents only.
fn check_bits<R: Bits>(pos: Select<R>, neg: Select<R>, non_finite: fn(R) -> bool) {
    let nan = R::POS | 1;
    let keep = [
        0,
        R::NEG & !R::POS,
        1u64 << 20,
        R::TINY,
        R::MAX,
        R::POS,
        R::NEG,
        nan,
    ];
    for b in keep {
        let x = R::of_bits(b);
        assert_eq!(pos(x, false).bits(), b, "or_pos_inf(false) changed {b:#x}");
        assert_eq!(neg(x, false).bits(), b, "or_neg_inf(false) changed {b:#x}");
        assert_eq!(
            pos(x, true).bits(),
            R::POS,
            "or_pos_inf(true) of {b:#x} is not +∞'s bits"
        );
        assert_eq!(
            neg(x, true).bits(),
            R::NEG,
            "or_neg_inf(true) of {b:#x} is not −∞'s bits"
        );
    }
    for (b, want) in [
        (0, false),
        (R::NEG & !R::POS, false),
        (R::of(1.0).bits(), false),
        (R::TINY, false),
        (R::MAX, false),
        (R::MAX | (R::NEG & !R::POS), false),
        (R::POS, true),
        (R::NEG, true),
        (nan, true),
        (R::NEG | 1, true),
    ] {
        assert_eq!(
            non_finite(R::of_bits(b)),
            want,
            "non_finite_bits({b:#x}) at {}",
            R::NAME
        );
    }
}

#[test]
fn ieee_bits() {
    check_bits::<f32>(f32::or_pos_inf, f32::or_neg_inf, f32::non_finite_bits);
    check_bits::<f64>(f64::or_pos_inf, f64::or_neg_inf, f64::non_finite_bits);
}

negative_control!(
    ieee_bits,
    "a non-finite test against the largest finite value, which misses NaN, must fail",
    expected = "non_finite_bits(0x7f800001) at f32",
    check_bits::<f32>(f32::or_pos_inf, f32::or_neg_inf, |x| x.abs() > f32::MAX)
);

/// `nan` holds on an all-ones exponent with a non-zero fraction: not on `±∞`, the pattern just under NaN's least, nor on
/// the largest finite value; on the least NaN, of either sign, and on the greatest.
fn check_nan_bits<R: Bits>(nan: fn(R) -> bool) {
    let sign = R::NEG & !R::POS;
    for (b, want) in [
        (0, false),
        (R::of(1.0).bits(), false),
        (R::MAX, false),
        (R::MAX | sign, false),
        (R::POS, false),
        (R::NEG, false),
        (R::POS | 1, true),
        (R::NEG | 1, true),
        (R::POS | R::MAX, true),
        (R::NEG | R::MAX, true),
    ] {
        assert_eq!(nan(R::of_bits(b)), want, "nan_bits({b:#x}) at {}", R::NAME);
    }
}

#[test]
fn ieee_nan_bits() {
    check_nan_bits::<f32>(f32::nan_bits);
    check_nan_bits::<f64>(f64::nan_bits);
}

negative_control!(
    ieee_nan_bits,
    "the exponent's test alone, which reads `+∞` as NaN, must fail",
    expected = "nan_bits(0x7f800000) at f32",
    check_nan_bits::<f32>(f32::non_finite_bits)
);

/// Equal masses, `r₀ = (−a, 0)`, `r₁ = (a, 0)`, `r₂ = (0, b)`, at rest: `λ = r₂`, so `b = 0` is the `α = 0` pole.
fn ic<R: Bits>(a: f64, b: f64) -> Ic<R> {
    let t = R::of(1.0 / 3.0);
    let z = R::of(0.0);
    Ic {
        m: [t, t, t],
        r: [[R::of(-a), z], [R::of(a), z], [z, R::of(b)]],
        p: [[z, z]; 3],
    }
}

/// The descriptor of `ic` under `d`.
type Describe<R> = fn(&Ic<R>) -> ICDescriptorOf<R>;

/// At each pole the stored bits are the pole's; one step off it, the quotient, finite.
fn check_poles<R: Bits>(d: Describe<R>) {
    let at = R::NAME;
    // ‖λ̃‖ = 0: r₂ at the inner pair's centre of mass.
    let pole = d(&ic::<R>(0.5, 0.0));
    assert_eq!(
        pole.lambda_mag.bits(),
        0,
        "λ̃ is not 0 at the α = 0 pole, {at}"
    );
    assert_eq!(
        pole.rho_ratio.bits(),
        R::POS,
        "rho_ratio is not +∞'s bits where ‖λ̃‖ = 0, {at}"
    );
    let off = d(&ic::<R>(0.5, 1e-3));
    assert!(
        !off.rho_ratio.non_finite_bits()
            && off.rho_ratio.bits() == (off.rho_mag / off.lambda_mag).bits(),
        "rho_ratio is not ‖ρ̃‖/‖λ̃‖ off the pole, {at}"
    );
    // A pair separation 0: r₀ = r₁, the collision pole, for the whole sum.
    let pole = d(&ic::<R>(0.0, 1.0));
    assert_eq!(
        pole.r_min_pair_0.bits(),
        0,
        "r_min_pair_0 is not 0 at the collision pole, {at}"
    );
    assert_eq!(
        pole.V_0.bits(),
        R::NEG,
        "V_0 is not −∞'s bits where a pair separation is 0, {at}"
    );
    assert_eq!(
        pole.virial_ratio.bits(),
        0,
        "virial_ratio is not +0 where V_0 is non-finite, {at}"
    );
    let off = d(&ic::<R>(1e-3, 1.0));
    assert!(
        !off.V_0.non_finite_bits() && off.V_0 < R::of(0.0),
        "V_0 is not a finite negative sum off the pole, {at}"
    );
    assert_eq!(
        off.V_0.bits(),
        potential(&ic::<R>(1e-3, 1.0)).bits(),
        "V_0 is not `potential`, {at}"
    );
    // The selects themselves, on the divisor's sides of 0.
    let one = R::of(1.0);
    assert_eq!(ratio(one, R::of(0.0)).bits(), R::POS, "ratio(1, 0), {at}");
    assert_eq!(
        ratio(one, R::of_bits(R::TINY)).bits(),
        (one / R::of_bits(R::TINY)).bits(),
        "ratio(1, tiny), {at}"
    );
    assert_eq!(
        virial(one, R::of_bits(R::NEG)).bits(),
        0,
        "virial(1, −∞), {at}"
    );
    assert_eq!(
        virial(one, R::of_bits(R::POS | 1)).bits(),
        0,
        "virial(1, NaN), {at}"
    );
    assert_eq!(
        virial(one, R::of(-4.0)).bits(),
        R::of(0.5).bits(),
        "virial(1, −4), {at}"
    );
    assert_eq!(
        virial(one, R::of_bits(R::MAX | (R::NEG & !R::POS))).bits(),
        (R::of(2.0) / R::of_bits(R::MAX)).bits(),
        "virial(1, −MAX), {at}"
    );
}

#[test]
fn descriptor_poles() {
    check_poles::<f32>(describe::<f32>);
    check_poles::<f64>(describe::<f64>);
}

/// `describe` with `V_0` the plain sum, its pole term infinite by `1/0`, not selected.
#[cfg(feature = "controls")]
fn divided<R: Bits>(ic: &Ic<R>) -> ICDescriptorOf<R> {
    let mut d = describe(ic);
    let [r0, r1, _] = ic.r;
    let sep = (r1[0] - r0[0]).abs();
    // At the pole the selected −∞ is replaced by the arithmetic a GPU need not pin: here a finite stand-in.
    if sep == R::of(0.0) {
        d.V_0 = R::of(-1e30);
        d.virial_ratio = (d.K_0 + d.K_0) / d.V_0.abs();
    }
    d
}

negative_control!(
    descriptor_poles,
    "a V_0 that is not −∞'s bits at the collision pole must fail",
    expected = "V_0 is not −∞'s bits where a pair separation is 0, f32",
    check_poles::<f32>(divided::<f32>)
);
