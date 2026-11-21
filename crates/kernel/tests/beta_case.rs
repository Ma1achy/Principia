//! The case of the encode's `β`, decided in `C` on its input and not read off the rotated floats (dd_encode §3.2, "The
//! signed encode of `β`"; `kernel::canon::signs_of`, `kernel::encode::blocks::beta_at`). A collinear state's rotated
//! `λ̃_y` is a rounding residue; its sign is a backend's own, and the encode's two ends there, 0 and `2π̂`, are one point
//! of the circle but two values.
//! - `beta_case_exact_collinear`: collinear f32 states in a random rational gauge, at a random power-of-two scale,
//!   whose terms of `ρ × Λ` and `ρ · Λ` are exact: the decision is the exact one, `λ̃_y = 0` and `λ̃_x`'s sign by
//!   construction; `β` is exactly `+0` (body 2 ahead of the pair, and the `α = 0` pole) or `π̂` (behind it), whatever
//!   the sign of the float residue, at f32 and on the same state at f64; and the encode of `C`'s output is the same
//!   point of the circle within a stated bound. Control: the superseded encode, its case read off the float `λ̃`.
//! - `beta_case_backend_independent`: the decision is one word at f32 and at f64 on the exact states, and one word
//!   under a power-of-two rescaling on states collinear only to rounding. Control: the superseded form, the float
//!   `λ̃_y` compared with 0, evaluated on the CPU as a backend that does not fuse `fma` and flushes subnormals would,
//!   decides differently from the fused, unflushed CPU.
//! - `beta_case_decided_side`: on states collinear only to rounding, in a random gauge, `β` lies on the decided side
//!   whatever the float residue's sign, and is the same for either sign of it. Control: `β` at the float's own signs.
//! - `beta_case_operands_unchanged`: `beta` on given operands is the form before the decision, bit for bit, on finite
//!   operands, so no state off the seam moves. Control: a lower half held to `π̂`.
//! - `beta_case_nan`: a NaN `λ̃` gives a NaN `β`, and the encode of a state that is not a number sets `z_β`'s clamp bit.
//!   Control: the form that reads a NaN as `+0`.
//!
//! **Domain** (pitfalls §10): `C` and the encode on states held fixed, at f32 and f64 on the CPU; no GPU (the
//! cross-backend arm is `crates/engine/tests/canon_gpu.rs`'s seam sweep), no integration.

use kernel::canon::{canonicalise, lambda_signs, LambdaSigns};
#[cfg(feature = "controls")]
use kernel::canon::{rotation, scale_step};
use kernel::decode::descriptor::pi_image;
use kernel::decode::Ic;
#[cfg(feature = "controls")]
use kernel::encode::blocks;
use kernel::encode::blocks::{beta, beta_at, clamps};
use kernel::encode::encode;
use kernel::generated::links::{Defaults, LinkReal, PeriodicBeta};
use kernel::real::{IeeeBits, Real};
use validation::decode_f32_eps::SplitMix;
use validation::negative_control;

const EPS32: f64 = f32::EPSILON as f64;
const TAU: f64 = std::f64::consts::TAU;

fn widen(x: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: x.m.map(f64::from),
        r: x.r.map(|v| v.map(f64::from)),
        p: x.p.map(|v| v.map(f64::from)),
    }
}

fn narrow(x: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: x.m.map(|v| v as f32),
        r: x.r.map(|v| v.map(|c| c as f32)),
        p: x.p.map(|v| v.map(|c| c as f32)),
    }
}

/// `2^e` at f32, `e` in the normal range.
fn pow2(e: i32) -> f32 {
    f32::from_bits(((e + 127) as u32) << 23)
}

/// A random integer in `[lo, hi]`.
fn int(rng: &mut SplitMix, lo: i32, hi: i32) -> i32 {
    lo + (rng.next_u64() % (hi - lo + 1) as u64) as i32
}

/// The masses of the exact states: few-bit, so their products with a pair difference are exact; the last two with a
/// third mass of `2⁻⁹⁰` and `2⁻¹²⁰`, where `√μ_λ`, `2⁻⁴⁵` and `2⁻⁶⁰`, takes the residue of `λ̃_y` of a state turned
/// only slightly off `+x` ([`exact_states`]) below the least normal f32.
fn exact_masses(n: usize) -> [f32; 3] {
    match n % 5 {
        0 => [0.25, 0.25, 0.5],
        1 => [0.5, 0.25, 0.25],
        2 => [0.125, 0.375, 0.5],
        3 => [0.5, 0.5, pow2(-90)],
        _ => [0.5, 0.5, pow2(-120)],
    }
}

/// An exact collinear state and the sign of its `λ̃_x`, `−1`, `0` or `1`, by construction.
struct Exact {
    x: Ic<f32>,
    ahead: i32,
}

/// Collinear f32 states in a random rational gauge whose terms of `ρ × Λ` and `ρ · Λ` are exact: the bodies at
/// `tᵢ·(a, b) + o`, `a` and `b` nonzero integers of at most 4 bits, `tᵢ` quarters in `[−4, 4]` with `t₀ ≠ t₁`, `o` a
/// pair of integers in `[−8, 8]`, all times one power of two from `2⁻³⁰` to `2³⁰`; and on one state in four the `y`
/// components times a further `2⁻⁴⁰` to `2⁻⁵⁰`, a line turned that little off `±x`, where step 2's products are that
/// small and their residues smaller by `2²⁴`. Every coordinate and pair difference
/// is exact, `ρ` and `Λ = m₀·d02 + m₁·d12` have at most 10 and 13 significant bits, and the two products of the cross
/// product are the same number of at most 23 bits, so their `Wide` difference is 0 exactly.
fn exact_states() -> Vec<Exact> {
    let mut rng = SplitMix::new(0x0057_be7a);
    (0..4000)
        .map(|n| {
            let m = exact_masses(n);
            let nonzero = |rng: &mut SplitMix| {
                let v = int(rng, 1, 15);
                if rng.next_u64() & 1 == 0 {
                    v
                } else {
                    -v
                }
            };
            let (a, b) = (nonzero(&mut rng), nonzero(&mut rng));
            let t0 = int(&mut rng, -16, 16);
            let t1 = loop {
                let t = int(&mut rng, -16, 16);
                if t != t0 {
                    break t;
                }
            };
            // One state in eight with body 2 at the pair's centre of mass where that is a quarter: the α = 0 pole.
            let com4 = (f64::from(m[0]) * f64::from(t0) + f64::from(m[1]) * f64::from(t1))
                / (f64::from(m[0]) + f64::from(m[1]));
            let t2 = if n % 8 == 7 && com4.fract() == 0.0 {
                com4 as i32
            } else {
                int(&mut rng, -16, 16)
            };
            let o = [int(&mut rng, -8, 8), int(&mut rng, -8, 8)];
            let k = pow2(int(&mut rng, -30, 30));
            let flat = if n % 4 == 3 {
                pow2(-int(&mut rng, 40, 50))
            } else {
                1.0
            };
            let at = |t: i32| {
                [
                    (t as f32 * 0.25 * a as f32 + o[0] as f32) * k,
                    (t as f32 * 0.25 * b as f32 + o[1] as f32) * k * flat,
                ]
            };
            // λ̃_x's sign: (t₁ − t₀)·(M₀₁t₂ − m₀t₀ − m₁t₁), exact in f64.
            let lam = (f64::from(m[0]) + f64::from(m[1])) * f64::from(t2)
                - f64::from(m[0]) * f64::from(t0)
                - f64::from(m[1]) * f64::from(t1);
            let along = f64::from(t1 - t0) * lam;
            Exact {
                x: Ic {
                    m,
                    r: [at(t0), at(t1), at(t2)],
                    p: [0; 3].map(|_| [rng.uniform(-0.5, 0.5), rng.uniform(-0.5, 0.5)]),
                },
                ahead: if along > 0.0 {
                    1
                } else if along < 0.0 {
                    -1
                } else {
                    0
                },
            }
        })
        .collect()
}

/// States collinear only to rounding: f64 collinear states, random masses, the bodies at random places on a line,
/// turned by a random angle, moved and scaled, then rounded to f32.
fn rounded_states() -> Vec<Ic<f32>> {
    let mut rng = SplitMix::new(0x0057_5eed);
    (0..4000)
        .map(|_| {
            let mut u = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
            let w = [u(0.2, 1.2), u(0.2, 1.2), u(0.2, 1.2)];
            let total: f64 = w.iter().sum();
            let t = [u(-1.0, 1.0), u(-1.0, 1.0), u(-1.5, 1.5)];
            let (s, c) = u(-3.2, 3.2).sin_cos();
            let (o, k) = ([u(-2.0, 2.0), u(-2.0, 2.0)], u(0.1, 10.0));
            narrow(&Ic {
                m: w.map(|x| x / total),
                r: t.map(|t| [(t * c + o[0]) * k, (t * s + o[1]) * k]),
                p: [0; 3].map(|_| [u(-0.5, 0.5), u(-0.5, 0.5)]),
            })
        })
        .collect()
}

/// An encode's `β` and the signs it took, on a state at `R`: the kernel's, or a control's.
type Enc<R> = fn(&Ic<R>) -> (R, LambdaSigns);

fn kernel_beta<R: LinkReal + IeeeBits>(x: &Ic<R>) -> (R, LambdaSigns) {
    let e = encode::<Defaults, R>(x);
    (e.blocks.beta, e.canonical.signs)
}

/// The superseded encode: `β`'s case read off the rotated float `λ̃`, each component compared with 0.
#[cfg(feature = "controls")]
fn superseded_beta<R: LinkReal + IeeeBits>(x: &Ic<R>) -> (R, LambdaSigns) {
    let lambda_t = blocks::jacobi(&canonicalise(x).ic).lambda_t;
    (beta(lambda_t), LambdaSigns::of_floats(lambda_t))
}

/// The distance of two angles on the circle.
fn circle(a: f64, b: f64) -> f64 {
    let d = (a - b).abs() % TAU;
    d.min(TAU - d)
}

/// What the exact states measured: the float residues of `λ̃_y` by sign, the subnormal ones, the states by side, and
/// the worst idempotence distance in units of its bound.
#[derive(Default, Debug)]
struct ExactSeen {
    positive: [usize; 2],
    negative: [usize; 2],
    zero: usize,
    subnormal: usize,
    poles: usize,
    worst_idempotence: f64,
}

/// The exact states' check on an encode at f32 and one at f64 (the module's note). The criteria, written before the
/// first run:
/// - every state's decision is `λ̃_y = 0`, and `λ̃_x`'s sign is the construction's;
/// - `β` is `+0`'s bits ahead of the pair and at the pole, `π̂`'s bits behind it, at f32 and at f64;
/// - the encode of `C`'s output is within `16·ε·R·(1/‖ρ‖ + 1/‖λ‖)` of it on the circle, `R` the largest coordinate of
///   `C`'s output: `C`'s rotation leaves each coordinate within an ulp of `R` of the line, twice over on a second pass,
///   and `β` is the angle of `λ` from `ρ`, so about `6·ε·R·(1/‖ρ‖ + 1/‖λ‖)`; 16 leaves a factor of under 3;
/// - the float residue of `λ̃_y` is positive on some states and negative on others, on each side of the pair, and
///   subnormal and nonzero on some: without them the sign of the residue is never put to the test.
fn check_exact(enc32: Enc<f32>, enc64: Enc<f64>) -> ExactSeen {
    let mut seen = ExactSeen::default();
    let (pi32, pi64) = (pi_image::<f32>(), pi_image::<f64>());
    for (n, s) in exact_states().iter().enumerate() {
        let c = canonicalise(&s.x);
        let y = c.lambda_t_y;
        let side = usize::from(s.ahead < 0);
        if y > 0.0 {
            seen.positive[side] += 1;
        } else if y < 0.0 {
            seen.negative[side] += 1;
        } else {
            seen.zero += 1;
        }
        seen.subnormal += usize::from(y != 0.0 && y.abs() < f32::MIN_POSITIVE);
        seen.poles += usize::from(s.ahead == 0);
        let (b32, signs32) = enc32(&s.x);
        let (b64, signs64) = enc64(&widen(&s.x));
        let want = LambdaSigns {
            y_negative: false,
            y_zero: true,
            x_negative: s.ahead < 0,
            x_zero: s.ahead == 0,
        };
        assert_eq!(
            (signs32, signs64),
            (want, want),
            "the decided signs of λ̃ are not the exact ones at state {n} (residue λ̃_y = {y:e}): {:?}",
            s.x
        );
        let (want32, want64) = if s.ahead < 0 {
            (pi32, pi64)
        } else {
            (0.0, 0.0)
        };
        assert!(
            b32.to_bits() == want32.to_bits() && b64.to_bits() == want64.to_bits(),
            "β is not the image at state {n}: {b32:e} at f32 and {b64:e} at f64, not {want32:e} (residue λ̃_y = \
             {y:e}): {:?}",
            s.x
        );
        if s.ahead == 0 {
            continue;
        }
        // Idempotence, on the circle.
        let out = widen(&c.ic);
        let rho = [out.r[1][0] - out.r[0][0], out.r[1][1] - out.r[0][1]];
        let m01 = out.m[0] + out.m[1];
        let lam =
            [0, 1].map(|k| out.r[2][k] - (out.m[0] * out.r[0][k] + out.m[1] * out.r[1][k]) / m01);
        let size = out
            .r
            .as_flattened()
            .iter()
            .fold(0.0f64, |a, v| a.max(v.abs()));
        let bound = 16.0 * EPS32 * size * (1.0 / rho[0].hypot(rho[1]) + 1.0 / lam[0].hypot(lam[1]));
        let (again, _) = enc32(&c.ic);
        let d = circle(f64::from(again), f64::from(b32));
        seen.worst_idempotence = seen.worst_idempotence.max(d / bound);
        assert!(
            d <= bound,
            "the encode of C's output is {d:e} from the state's β on the circle, past {bound:e}, at state {n}: \
             β = {b32:e}, then {again:e}"
        );
    }
    for side in 0..2 {
        assert!(
            seen.positive[side] > 0 && seen.negative[side] > 0,
            "the residue of λ̃_y has one sign only on side {side}: {seen:?}"
        );
    }
    assert!(seen.subnormal > 0, "no subnormal residue of λ̃_y: {seen:?}");
    assert!(seen.poles > 0, "no state at the α = 0 pole: {seen:?}");
    seen
}

#[test]
fn beta_case_exact_collinear() {
    let seen = check_exact(kernel_beta::<f32>, kernel_beta::<f64>);
    eprintln!(
        "beta_case_exact_collinear: 4000 exact collinear f32 states; the float residue of λ̃_y positive on {:?} \
         (ahead, behind), negative on {:?}, zero on {}, subnormal and nonzero on {}; {} at the pole; β exactly +0 or \
         π̂ on every one at f32 and f64; the encode of C's output at most {:.3} of its bound away on the circle",
        seen.positive, seen.negative, seen.zero, seen.subnormal, seen.poles, seen.worst_idempotence
    );
}

negative_control!(
    beta_case_exact_collinear,
    "the superseded encode, β's case read off the rotated float λ̃, follows the residue's sign and must fail",
    expected = "the decided signs of λ̃ are not the exact ones",
    {
        check_exact(superseded_beta::<f32>, superseded_beta::<f64>);
    }
);

/// A subnormal result flushed to zero, as Metal does.
#[cfg(feature = "controls")]
fn flush(x: f32) -> f32 {
    if x != 0.0 && x.abs() < f32::MIN_POSITIVE {
        0.0
    } else {
        x
    }
}

/// The superseded form's `λ̃_y` (`Canonical::lambda_t_y`), evaluated on the CPU as a backend would: step 2's `R(−φ)`
/// on step 0's output and `kernel::canon::lambda_t_axis`'s arithmetic, each `fma` fused or, as CI's lavapipe evaluates
/// it, a rounded product and an add (`fused`), every result flushed to zero where subnormal or not (`flush_subnormals`).
/// Step 0's output and the turn are the kernel's own, the same on both arms. Fused and unflushed it is `C`'s recorded
/// `λ̃_y`, bit for bit, which the control checks first.
#[cfg(feature = "controls")]
fn plain_lambda_t_y(x: &Ic<f32>, fused: bool, flush_subnormals: bool) -> f32 {
    let f = |q: f32| if flush_subnormals { flush(q) } else { q };
    let mad = |a: f32, b: f32, c: f32| {
        if fused {
            f(a.mul_add(b, c))
        } else {
            f(f(a * b) + c)
        }
    };
    let r = scale_step(x).r;
    let (c, s) = rotation([x.r[1][0] - x.r[0][0], x.r[1][1] - x.r[0][1]]);
    let y = r.map(|v| mad(c, v[1], -f(s * v[0])));
    let m = x.m;
    let m01 = mad(m[1], 1.0, m[0]);
    let inner = f(mad(m[1], y[1], f(m[0] * y[0])) / m01);
    f(f(f(m[2] * m01).sqrt()) * f(y[2] - inner))
}

/// The case a float `λ̃_y` reads: below 0, 0, or above.
#[cfg(feature = "controls")]
fn float_case(y: f32) -> u32 {
    LambdaSigns::of_floats([1.0, y]).word() & 3
}

/// Two evaluations decide one case on every state. Criterion, written before the first run: the words are equal on
/// every state, no exception.
fn check_cases(a: &[u32], b: &[u32], what: &str) {
    assert_eq!(a.len(), b.len());
    for (k, (x, y)) in a.iter().zip(b).enumerate() {
        assert_eq!(
            x, y,
            "the case of β is decided differently at state {k} ({what})"
        );
    }
}

#[test]
fn beta_case_backend_independent() {
    // f32 against f64 on the exact states, the f64 state the f32 one widened.
    let exact = exact_states();
    let at32: Vec<u32> = exact
        .iter()
        .map(|s| lambda_signs(s.x.m, s.x.r).word())
        .collect();
    let at64: Vec<u32> = exact
        .iter()
        .map(|s| {
            let w = widen(&s.x);
            lambda_signs(w.m, w.r).word()
        })
        .collect();
    check_cases(&at32, &at64, "f32 against f64, exact states");
    // A power-of-two rescaling, which moves every exponent and no mantissa, on states collinear only to rounding:
    // the decision carries its exponents as integers, so it is the same word from 2⁻⁶⁰ to 2⁶⁰.
    let rounded = rounded_states();
    let base: Vec<u32> = rounded
        .iter()
        .map(|x| lambda_signs(x.m, x.r).word())
        .collect();
    for e in [-60, -27, 27, 60] {
        let k = pow2(e);
        let scaled: Vec<u32> = rounded
            .iter()
            .map(|x| lambda_signs(x.m, x.r.map(|v| [v[0] * k, v[1] * k])).word())
            .collect();
        check_cases(&scaled, &base, &format!("scaled by 2^{e}"));
    }
    // Both halves and the axis are reached, or the equalities above say nothing.
    let lower = base.iter().filter(|w| *w & 1 != 0).count();
    let axis = at32.iter().filter(|w| *w & 2 != 0).count();
    assert!(
        lower > 0 && lower < base.len() && axis == at32.len(),
        "{lower} of {} rounded states decided below, {axis} of {} exact states on the axis",
        base.len(),
        at32.len()
    );
    // Measured, not asserted: the two widths on the states collinear only to rounding, where each rounds its own
    // cross product.
    let widths = rounded
        .iter()
        .zip(&base)
        .filter(|(x, w)| {
            let x = widen(x);
            lambda_signs(x.m, x.r).word() != **w
        })
        .count();
    eprintln!(
        "beta_case_backend_independent: {} exact states, one case at f32 and f64; {} states collinear to rounding \
         ({lower} decided below), one case at 2^-60, 2^-27, 2^27 and 2^60; f32 and f64 decide differently on {widths} \
         of them (measured: each width rounds its own cross product)",
        at32.len(),
        base.len()
    );
}

negative_control!(
    beta_case_backend_independent,
    "the superseded form, the float λ̃_y compared with 0, evaluated unfused and flushing subnormals, decides the \
     case differently from the fused, unflushed CPU and must fail",
    expected = "the case of β is decided differently",
    {
        let states: Vec<Ic<f32>> = exact_states().into_iter().map(|s| s.x).collect();
        for x in &states {
            assert_eq!(
                plain_lambda_t_y(x, true, false).to_bits(),
                canonicalise(x).lambda_t_y.to_bits(),
                "the emulation, fused and unflushed, is not C's recorded λ̃_y"
            );
        }
        let cpu: Vec<u32> = states
            .iter()
            .map(|x| float_case(plain_lambda_t_y(x, true, false)))
            .collect();
        let emulated: Vec<u32> = states
            .iter()
            .map(|x| float_case(plain_lambda_t_y(x, false, true)))
            .collect();
        check_cases(&emulated, &cpu, "the float λ̃_y, unfused and flushed against fused and unflushed");
    }
);

/// What the rounded states measured: the float residue against the decision.
#[derive(Default, Debug)]
struct SideSeen {
    below_with_positive: usize,
    below_with_zero: usize,
    above_with_negative: usize,
    above_with_zero: usize,
    agree: usize,
    axis: usize,
}

/// `β` on the decided side, on the states collinear only to rounding, `beta_of` giving `β` from the float `λ̃` and the
/// decided signs. Criteria, written before the first run:
/// - decided below: `β` in `[π̂ − 1 ulp, 2π̂]` (dd_encode §3.2: within an ulp of the `−x` half-axis the lower half's sum
///   can round just below `π̂`), and above `3π/2` where body 2 is decided ahead of the pair;
/// - decided above: `β` in `[+0, π̂]`, and below `π/2` where body 2 is decided ahead;
/// - decided on the axis: `+0` or `π̂` exactly;
/// - `β` is the same bits with the float residue's sign turned;
/// - some state is decided below with a float residue above 0, and some decided above with one below 0.
fn check_sides(beta_of: fn([f32; 2], LambdaSigns) -> f32) -> SideSeen {
    let mut seen = SideSeen::default();
    let pi = pi_image::<f32>();
    let tau = pi + pi;
    for (n, x) in rounded_states().iter().enumerate() {
        let e = encode::<Defaults, f32>(x);
        let signs = e.canonical.signs;
        let lambda_t = e.blocks.jacobi.lambda_t;
        let y = lambda_t[1];
        let b = beta_of(lambda_t, signs);
        let ahead = !(signs.x_negative | signs.x_zero);
        if signs.y_zero {
            seen.axis += 1;
            let want = if signs.x_negative { pi } else { 0.0 };
            assert!(
                b.to_bits() == want.to_bits(),
                "β = {b:e} is on the other side of a state decided on the axis, not {want:e} (float λ̃ = \
                 {lambda_t:?}) at state {n}"
            );
        } else if signs.y_negative {
            seen.below_with_positive += usize::from(y > 0.0);
            seen.below_with_zero += usize::from(y == 0.0);
            seen.agree += usize::from(y < 0.0);
            assert!(
                b >= pi.next_down() && b <= tau && (!ahead || f64::from(b) > 0.75 * TAU),
                "β = {b:e} is on the other side of a state decided below (float λ̃ = {lambda_t:?}) at state {n}"
            );
        } else {
            seen.above_with_negative += usize::from(y < 0.0);
            seen.above_with_zero += usize::from(y == 0.0);
            seen.agree += usize::from(y > 0.0);
            assert!(
                b.to_bits() != (-0.0f32).to_bits()
                    && b >= 0.0
                    && b <= pi
                    && (!ahead || f64::from(b) < 0.25 * TAU),
                "β = {b:e} is on the other side of a state decided above (float λ̃ = {lambda_t:?}) at state {n}"
            );
        }
        let turned = beta_of([lambda_t[0], -y], signs);
        assert_eq!(
            turned.to_bits(),
            b.to_bits(),
            "β follows the sign of the float residue at state {n}"
        );
    }
    assert!(
        seen.below_with_positive > 0 && seen.above_with_negative > 0,
        "no state whose float residue disagrees with the decision in each direction: {seen:?}"
    );
    seen
}

#[test]
fn beta_case_decided_side() {
    let seen = check_sides(beta_at);
    eprintln!(
        "beta_case_decided_side: 4000 states collinear to rounding; the float λ̃_y agrees with the decision on {}, is \
         above 0 on {} decided below and 0 on {}, below 0 on {} decided above and 0 on {}; {} decided on the axis; β \
         on the decided side on every one",
        seen.agree,
        seen.below_with_positive,
        seen.below_with_zero,
        seen.above_with_negative,
        seen.above_with_zero,
        seen.axis
    );
}

negative_control!(
    beta_case_decided_side,
    "β at the float λ̃'s own signs lands on the other side wherever the residue disagrees and must fail",
    expected = "is on the other side of a state decided",
    {
        check_sides(|lambda_t, _| beta(lambda_t));
    }
);

/// `β` as it was before the decision, on operands: every case chosen by comparing the float `λ̃_x`, `λ̃_y` with 0.
fn before<R: LinkReal>(lambda_t: [R; 2]) -> R {
    let zero = R::zero();
    let pi = pi_image::<R>();
    let [x, y] = lambda_t;
    let x0 = if x == zero { zero } else { x };
    let raw = Real::arctan2(y, x0);
    let upper = if raw > zero {
        if raw > pi {
            pi
        } else {
            raw
        }
    } else {
        zero
    };
    if (y == zero) & (x < zero) {
        pi
    } else if (x == zero) & (y == zero) {
        zero
    } else if y < zero {
        raw + (pi + pi)
    } else {
        upper
    }
}

/// Finite operand pairs: each sign of each zero, the least subnormal and the least normal, values beside the `−x`
/// half-axis where `atan2` rounds to the nearest π, and random pairs over 60 binades of each sign.
fn operand_pairs() -> Vec<[f32; 2]> {
    let edge = [
        0.0f32,
        -0.0,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-30,
        -1e-30,
        1e-10,
        -1e-10,
        0.6,
        -0.6,
        1.0,
        -1.0,
        3e20,
        -3e20,
    ];
    let mut pairs: Vec<[f32; 2]> = edge.iter().flat_map(|x| edge.map(|y| [*x, y])).collect();
    let mut rng = SplitMix::new(0x0be7_a0ff);
    for _ in 0..20_000 {
        let mut v = || {
            let m = rng.uniform(1.0, 2.0) * pow2(int(&mut rng, -30, 30));
            if rng.next_u64() & 1 == 0 {
                m
            } else {
                -m
            }
        };
        pairs.push([v(), v()]);
    }
    pairs
}

/// `beta` on operands against the form before the decision, at f32 and f64. Criterion, written before the first run:
/// the same bits on every finite pair, so no state off the seam moves.
fn check_operands(f32_form: fn([f32; 2]) -> f32, f64_form: fn([f64; 2]) -> f64) -> usize {
    let pairs = operand_pairs();
    let mut near_pi = 0;
    for p in &pairs {
        let (got, want) = (f32_form(*p), before(*p));
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "β moved at the operands {p:?} at f32: {got:e}, was {want:e}"
        );
        near_pi += usize::from(p[1] < 0.0 && want < pi_image::<f32>());
        let w = p.map(f64::from);
        let (got, want) = (f64_form(w), before(w));
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "β moved at the operands {w:?} at f64: {got:e}, was {want:e}"
        );
    }
    assert!(
        near_pi > 0,
        "no lower-half pair whose β rounds just below π̂"
    );
    pairs.len()
}

#[test]
fn beta_case_operands_unchanged() {
    let n = check_operands(beta::<f32>, beta::<f64>);
    eprintln!(
        "beta_case_operands_unchanged: {n} finite operand pairs, β the bits it was at f32 and f64"
    );
}

negative_control!(
    beta_case_operands_unchanged,
    "a lower half read as 2π̂ less an angle held to π̂ moves the states an ulp below the −x half-axis and must fail",
    expected = "β moved at the operands",
    {
        fn held<R: LinkReal + IeeeBits>(lambda_t: [R; 2]) -> R {
            let pi = pi_image::<R>();
            let b = beta(lambda_t);
            if lambda_t[1] < R::zero() && b < pi {
                pi
            } else {
                b
            }
        }
        check_operands(held::<f32>, held::<f64>);
    }
);

/// A NaN `λ̃` on a form of `β` at `R`. Criteria, written before the first run: with a NaN in `λ̃_x`, in `λ̃_y` or in
/// both, at every one of the sixteen sign words, `β` is NaN and `z_β`'s clamp test reads it as clamped; and the encode
/// of a state with a NaN coordinate has a NaN `z_β` and bit 1 of its clamped controls set.
fn check_nan<R: LinkReal + IeeeBits + std::fmt::Debug>(form: fn([R; 2], LambdaSigns) -> R) {
    let (nan, half) = (R::nan(), R::from(0.5).unwrap());
    for lambda_t in [
        [nan, half],
        [half, nan],
        [nan, nan],
        [nan, -half],
        [-half, nan],
    ] {
        for word in 0..16 {
            let b = form(lambda_t, LambdaSigns::of_word(word));
            assert!(
                b.is_nan(),
                "β = {b:?}, not NaN, at λ̃ = {lambda_t:?} and the signs' word {word} at {}",
                R::NAME
            );
            assert!(
                clamps::<PeriodicBeta, R>(b),
                "a NaN β is not read as clamped at {}",
                R::NAME
            );
        }
    }
}

/// The encode of states with a NaN position coordinate, at `R`: `z_β` NaN and its clamp bit set.
fn check_nan_state<R: LinkReal + IeeeBits + std::fmt::Debug>() {
    let c = |v: f64| R::from(v).unwrap();
    for (i, k) in [(0, 0), (1, 1), (2, 0), (2, 1)] {
        let mut x = Ic {
            m: [c(0.2), c(0.3), c(0.5)],
            r: [[c(0.1), c(0.2)], [c(0.7), c(-0.3)], [c(-0.4), c(0.9)]],
            p: [[c(0.1), c(-0.2)], [c(0.3), c(0.05)], [c(-0.4), c(0.15)]],
        };
        x.r[i][k] = R::nan();
        let e = encode::<Defaults, R>(&x);
        assert!(
            e.blocks.beta.is_nan() && e.z.z[1].is_nan(),
            "β = {:?} and z_β = {:?}, not NaN, with r[{i}][{k}] NaN at {}",
            e.blocks.beta,
            e.z.z[1],
            R::NAME
        );
        assert!(
            e.report.clamped_controls & 2 != 0 && e.report.lookup_clamped(),
            "z_β's clamp bit is not set with r[{i}][{k}] NaN at {}: {:#b}",
            R::NAME,
            e.report.clamped_controls
        );
    }
}

#[test]
fn beta_case_nan() {
    check_nan::<f32>(beta_at);
    check_nan::<f64>(beta_at);
    check_nan::<f32>(|l, _| beta(l));
    check_nan::<f64>(|l, _| beta(l));
    check_nan_state::<f32>();
    check_nan_state::<f64>();
}

negative_control!(
    beta_case_nan,
    "the form that reads a NaN angle as not above 0, so +0, gives a clean β and must fail",
    expected = "not NaN, at λ̃",
    {
        check_nan::<f64>(|l, _| before(l));
    }
);
