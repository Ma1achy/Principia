//! The kernel's own reduction of the decode's `β` onto the circle at f32 (`kernel::circle`), on the CPU.
//! - `circle_table_is_one_over_two_pi`: `INV_TWO_PI` is `⌊2²²⁴/2π⌋`, recomputed here from Machin's formula in integer
//!   arithmetic, word 0 zero.
//! - `circle_on_one_period`: every result lies in `[−π̂, π̂]`, `π̂` the f32 nearest `π`; `|β| ≤ π̂`, NaN and `±∞` pass
//!   through bit for bit; the result is odd in `β`, bit for bit.
//! - `circle_is_beta_mod_two_pi`: on every f32 exponent past `π̂`, edge and random mantissas, the result is within half
//!   an f32 step (and `2⁻³⁸` relative, the module's caveat at a midpoint) of `β mod 2π` as the pinned f64 libm
//!   gives it, `atan2(sin β, cos β)` at the widened `β`.
//! - `circle_is_beta_mod_two_pi_exhaustive`, ignored: the same check on every positive f32 past `π̂`, with the closest
//!   approach to a multiple of `2π` measured.
//!
//! Each pass criterion is written before the run, from `kernel::circle`'s stated accuracy, and each test has a control.

use kernel::circle::{on_circle_f32, INV_TWO_PI, PI_HAT_BITS};
use kernel::Real;
use num_bigint::BigUint;
use validation::negative_control;

// ── The table ────────────────────────────────────────────────────────────────────────────────────────────────────

/// `⌊2²²⁴/2π⌋` by Machin's `π = 16·atan(1/5) − 4·atan(1/239)`, each `atan(1/x)` summed at `2⁻⁴⁰⁰` in integers.
/// Each truncated term errs by under one unit, so `π`'s error is below a few hundred units of `2⁻⁴⁰⁰`, which cannot
/// move the quotient's floor unless `2²²⁴/2π` lay within `2⁻¹⁵⁰` of an integer.
fn inv_two_pi_224() -> BigUint {
    let p = 400u32;
    let one = BigUint::from(1u32) << p;
    let atan_inv = |x: u32| {
        let (mut sum_pos, mut sum_neg) = (BigUint::from(0u32), BigUint::from(0u32));
        let x2 = BigUint::from(x) * x;
        let mut t = &one / x;
        let mut n = 1u32;
        while t != BigUint::from(0u32) {
            if (n / 2).is_multiple_of(2) {
                sum_pos += &t / n;
            } else {
                sum_neg += &t / n;
            }
            t /= &x2;
            n += 2;
        }
        sum_pos - sum_neg
    };
    let pi = (atan_inv(5) * 16u32) - (atan_inv(239) * 4u32);
    (BigUint::from(1u32) << (224 + p)) / (pi * 2u32)
}

/// `table`'s eight words, word 0 zero and words 1 to 7 `⌊2²²⁴/2π⌋`'s, most significant first.
fn check_table(table: &[u32; 8]) {
    assert_eq!(table[0], 0, "the table's word 0 is not 0");
    let want = inv_two_pi_224();
    let mut got = BigUint::from(0u32);
    for w in &table[1..] {
        got = (got << 32u32) + *w;
    }
    assert_eq!(
        got, want,
        "the table is not ⌊2²²⁴/2π⌋: {got:x} against {want:x}"
    );
}

#[test]
fn circle_table_is_one_over_two_pi() {
    check_table(&INV_TWO_PI);
}

negative_control!(
    circle_table_is_one_over_two_pi,
    "the table with one bit of word 4 flipped",
    expected = "the table is not",
    {
        let mut t = INV_TWO_PI;
        t[4] ^= 1 << 7;
        check_table(&t)
    }
);

// ── The sweep ────────────────────────────────────────────────────────────────────────────────────────────────────

/// A deterministic generator for the mantissas (SplitMix64).
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// The magnitudes past `π̂`: on every exponent field from 128 (`[2, 4)`) to 254, the mantissas 0, 1, the largest,
/// and 2000 drawn; the decode stage's saturated controls (43.671875, 44, 44.5, 10³, 10⁴, 10³⁰ and the largest finite, each
/// plus `π/2` as `periodic_beta` adds it); and the f32 values just past `π̂`.
fn sweep() -> Vec<f32> {
    let mut rng = Mix(2026);
    let mut out = Vec::new();
    for field in 128u32..=254 {
        let fractions = [0, 1, 0x007F_FFFF]
            .into_iter()
            .chain((0..2000).map(|_| (rng.next() as u32) & 0x007F_FFFF));
        out.extend(fractions.map(|f| f32::from_bits((field << 23) | f)));
    }
    for s in [43.671_875f32, 44.0, 44.5, 1e3, 1e4, 1e30, f32::MAX] {
        out.push(s + core::f32::consts::FRAC_PI_2);
        out.push(s);
    }
    out.extend((1..=64).map(|k| f32::from_bits(PI_HAT_BITS + k)));
    out
}

/// `reduce` on the sweep, each sign: the result in `[−π̂, π̂]` and odd in `β`, bit for bit.
fn check_period(reduce: fn(f32) -> f32) {
    for b in sweep() {
        for beta in [b, -b] {
            let r = reduce(beta);
            assert!(
                r.to_bits() & 0x7FFF_FFFF <= PI_HAT_BITS,
                "the angle {r:e} for β = {beta:e} is outside [−π̂, π̂]"
            );
        }
        assert_eq!(
            reduce(-b).to_bits(),
            (-reduce(b)).to_bits(),
            "the reduction is not odd at β = {b:e}"
        );
    }
}

/// The values that pass through, each bit for bit: `|β| ≤ π̂` (the zeros, the least subnormal, `π/2`, `π̂` and the
/// f32 below it, 1, a third), NaN and `±∞`.
fn check_pass_through(reduce: fn(f32) -> f32) {
    let small = [
        0.0f32,
        -0.0,
        f32::from_bits(1),
        core::f32::consts::FRAC_PI_2,
        f32::from_bits(PI_HAT_BITS),
        f32::from_bits(PI_HAT_BITS - 1),
        1.0,
        1.0 / 3.0,
    ];
    for x in
        small
            .into_iter()
            .flat_map(|x| [x, -x])
            .chain([f32::NAN, f32::INFINITY, f32::NEG_INFINITY])
    {
        assert_eq!(
            reduce(x).to_bits(),
            x.to_bits(),
            "{x:e} does not pass through unchanged"
        );
    }
}

#[test]
fn circle_on_one_period() {
    check_period(on_circle_f32);
    check_pass_through(on_circle_f32);
    check_pass_through(Real::on_circle);
    for b in sweep() {
        assert_eq!(
            Real::on_circle(b).to_bits(),
            on_circle_f32(b).to_bits(),
            "f32's on_circle is not the kernel's reduction at {b:e}"
        );
        assert_eq!(
            Real::on_circle(f64::from(b)).to_bits(),
            f64::from(b).to_bits(),
            "f64's on_circle is not the identity at {b:e}"
        );
    }
}

negative_control!(
    circle_on_one_period,
    "the decode before the fix, which handed β to cos and sin unreduced",
    expected = "is outside [−π̂, π̂]",
    check_period(|b| b)
);

/// `β mod 2π` in `(−π, π]` at f64, from the pinned libm's exact reductions: `atan2(sin β, cos β)` at the widened `β`.
fn reference(beta: f32) -> f64 {
    let b = f64::from(beta);
    Real::arctangent2(Real::sine(b), Real::cosine(b))
}

/// `reduce` at `beta` against the reference, in f32 steps of the result, read on the circle so that `π̂` and `−π̂` are
/// one point: within half a step, plus `2⁻³⁸` of the reference (the module's caveat: within `2⁻⁴⁰` of a midpoint the
/// rounding may go either way) and `10⁻¹⁵` (the reference's own error). Returns the error in steps and the result.
fn check_one(reduce: fn(f32) -> f32, beta: f32) -> (f64, f64) {
    let r = f64::from(reduce(beta));
    let want = reference(beta);
    let two_pi = 2.0 * core::f64::consts::PI;
    let d = r - want;
    let d = (d - two_pi * (d / two_pi).round()).abs();
    let step = f64::from(f32::from_bits((r.abs() as f32).to_bits().max(1) + 1)) - r.abs();
    let bound = 0.5 * step + want.abs() * 2f64.powi(-38) + 1e-15;
    assert!(
        d <= bound,
        "the angle {r:e} for β = {beta:e} is {d:e} from β mod 2π = {want:e}, past {bound:e}"
    );
    (d / step, r)
}

/// `reduce` on the sweep, each sign, by [`check_one`]. Prints the worst, in f32 steps.
fn check_accuracy(reduce: fn(f32) -> f32) {
    let mut worst = (0.0f64, 0.0f32);
    for b in sweep() {
        for beta in [b, -b] {
            let (steps, _) = check_one(reduce, beta);
            if steps > worst.0 {
                worst = (steps, beta);
            }
        }
    }
    println!(
        "on_circle_f32: worst {:.6} f32 steps from β mod 2π, at β = {:e}",
        worst.0, worst.1
    );
}

#[test]
fn circle_is_beta_mod_two_pi() {
    check_accuracy(on_circle_f32);
}

negative_control!(
    circle_is_beta_mod_two_pi,
    "the float reduction β − 2π·round(β/2π) at f32, which cannot reduce a large β",
    expected = "from β mod 2π",
    check_accuracy(|b| {
        let two_pi = 2.0 * core::f32::consts::PI;
        (b - two_pi * (b / two_pi).round())
            .clamp(-f32::from_bits(PI_HAT_BITS), f32::from_bits(PI_HAT_BITS))
    })
);

/// Every positive f32 past `π̂` (the reduction is odd, `circle_on_one_period`), by [`check_one`] and in `[−π̂, π̂]`, on
/// all threads. Prints the worst error in f32 steps, how many results are not the f32 nearest `β mod 2π` (past half a
/// step), and the closest approach to a multiple of `2π`, the least `|β mod 2π|`, in turns: the measured form of the
/// module's note. Minutes in a release build (too slow for CI; run with `cargo test --release -p kernel --test circle
/// -- --ignored`).
fn check_exhaustive(reduce: fn(f32) -> f32, threads: u32) {
    let (lo, hi) = (PI_HAT_BITS + 1, 0x7F80_0000u32);
    let chunk = (hi - lo).div_ceil(threads);
    let parts: Vec<(f64, u32, u64, f64, u32)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                s.spawn(move || {
                    let (a, b) = (lo + t * chunk, (lo + (t + 1) * chunk).min(hi));
                    let (mut worst, mut worst_at, mut off, mut closest, mut closest_at) =
                        (0.0f64, 0u32, 0u64, f64::MAX, 0u32);
                    for bits in a..b {
                        let beta = f32::from_bits(bits);
                        let (steps, r) = check_one(reduce, beta);
                        assert!(
                            reduce(beta).to_bits() & 0x7FFF_FFFF <= PI_HAT_BITS,
                            "the angle {r:e} for β = {beta:e} is outside [−π̂, π̂]"
                        );
                        if steps > worst {
                            (worst, worst_at) = (steps, bits);
                        }
                        if steps > 0.5 {
                            off += 1;
                        }
                        if r.abs() < closest {
                            (closest, closest_at) = (r.abs(), bits);
                        }
                    }
                    (worst, worst_at, off, closest, closest_at)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    });
    let worst = parts
        .iter()
        .fold((0.0, 0), |w, p| if p.0 > w.0 { (p.0, p.1) } else { w });
    let off: u64 = parts.iter().map(|p| p.2).sum();
    let closest = parts
        .iter()
        .fold((f64::MAX, 0), |c, p| if p.3 < c.0 { (p.3, p.4) } else { c });
    let turns = closest.0 / (2.0 * core::f64::consts::PI);
    println!(
        "on_circle_f32, every f32 in (π̂, ∞): {} values; worst {:.6} f32 steps at β = {:e}; {off} not the nearest f32; \
         closest approach |β mod 2π| = {:e} ({:e} turns, 2^{:.2}) at β = {:e}",
        hi - lo,
        worst.0,
        f32::from_bits(worst.1),
        closest.0,
        turns,
        turns.log2(),
        f32::from_bits(closest.1)
    );
}

#[test]
#[ignore]
fn circle_is_beta_mod_two_pi_exhaustive() {
    let threads = std::thread::available_parallelism().map_or(8, |n| n.get()) as u32;
    check_exhaustive(on_circle_f32, threads);
}

negative_control!(
    circle_is_beta_mod_two_pi_exhaustive,
    "the decode before the fix, β handed on unreduced, on one thread so it stops at the first β past π̂",
    expected = "is outside [−π̂, π̂]",
    check_exhaustive(|b| b, 1)
);
