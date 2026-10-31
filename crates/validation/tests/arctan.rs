//! The arctangent sweeps' own instruments (`validation::arctan`; RQ-628, decided by the orchestrator per R-369, option
//! (a)): the measures the kernel's and engine's sweeps report, each checked on values whose answer is known, so that a
//! sweep that passes could have failed.
//! - `arctan_measure_steps`: [`place`], [`steps`] and [`ulps`] at `±0`, across zero, at a binade edge and between f32s.
//! - `arctan_measure_accuracy`: [`Accuracy`] counts results off the nearest f32 and more than one step from it, keeps
//!   the first of equal worst errors and where it is, skips NaN operands, and merges.
//! - `arctan_measure_round_trip`: [`RoundTrip`] counts moved angles, keeps the most steps and the largest relative
//!   errors in `cos` and `sin`, skips a zero `cos` or `sin`, merges, and gives the share moved.
//! - `arctan_sweep_ranges`: [`reduced`] and [`alphas`] start at `+0`, step by their stride, and end at 1 and at the
//!   largest f32 below `π/2`.
//! - `arctan_sweep_plane`: [`plane`] is finite, mixes in its zeros, poles and diagonals at about their shares, spans
//!   every exponent, and is, by checksum, the sweep the PR's results were measured on (a regression pin).

use validation::arctan::{
    alphas, place, plane, reduced, steps, ulps, Accuracy, RoundTrip, HALF_PI_BELOW_BITS, ONE_BITS,
};
use validation::negative_control;

/// The f32 above `v`'s magnitude, with `v`'s sign.
fn up(v: f32) -> f32 {
    f32::from_bits(v.to_bits() + 1)
}

/// Checks the step measures; `least` is taken as the least positive f32.
fn check_steps(least: f32) {
    assert_eq!(place(0.0), 0, "+0's place");
    assert_eq!(place(-0.0), 0, "-0's place");
    assert_eq!(place(1.0), i64::from(ONE_BITS), "1's place");
    assert_eq!(place(-1.0), -i64::from(ONE_BITS), "-1's place");
    assert_eq!(
        steps(least, -least),
        2,
        "the least subnormals either side of 0 are two steps apart"
    );
    assert_eq!(steps(1.0, up(1.0)), 1, "1 and the f32 above it");
    assert_eq!(steps(up(1.0), 1.0), 1, "steps is symmetric");
    assert_eq!(steps(2.0, 1.0), 1 << 23, "a binade is 2²³ steps");
    assert_eq!(ulps(1.0, 1.0), 0.0, "an exact result");
    assert_eq!(ulps(up(1.0), 1.0), 1.0, "one f32 above 1");
    assert_eq!(
        ulps(1.0, 1.0 + 2f64.powi(-24)),
        0.5,
        "half way between 1 and the f32 above it"
    );
    assert_eq!(ulps(-up(1.0), -1.0), 1.0, "the ULP is the magnitude's");
    assert_eq!(ulps(least, 0.0), 1.0, "the ULP at 0 is the least subnormal");
    assert_eq!(
        ulps(2.0, 2.0 - 2f64.powi(-23)),
        1.0,
        "below 2 the ULP is 2⁻²³, the gap up from the exact value's nearest f32"
    );
    assert_eq!(
        ulps(2.0 - 2f32.powi(-23), 2.0),
        0.5,
        "at 2 the ULP is 2⁻²², the gap up from 2"
    );
}

#[test]
fn arctan_measure_steps() {
    check_steps(f32::from_bits(1));
}

negative_control!(
    arctan_measure_steps,
    "the least normal taken for the least subnormal",
    expected = "the least subnormals either side of 0 are two steps apart",
    check_steps(f32::MIN_POSITIVE)
);

/// The accuracy of the results `got` for the pairs `yx`.
fn accuracy(yx: &[[f32; 2]], got: &[f32]) -> Accuracy {
    let mut acc = Accuracy::default();
    for (&p, &g) in yx.iter().zip(got) {
        acc.add(p, g);
    }
    acc
}

/// Checks [`Accuracy`]; `one` is taken as `atan2(1, 1)`'s nearest f32.
fn check_accuracy(one: f32) {
    let a = accuracy(&[[1.0, 1.0]], &[one]);
    assert_eq!((a.n, a.off, a.far), (1, 0, 0), "the nearest f32 is not off");
    assert_eq!(a.worst_at, [1.0, 1.0], "where the worst is");
    assert!(a.worst > 0.0 && a.worst < 0.5, "π/4's error, {}", a.worst);
    let a = accuracy(&[[1.0, 1.0]], &[up(one)]);
    assert_eq!((a.off, a.far), (1, 0), "one step off");
    let a = accuracy(&[[1.0, 1.0]], &[up(up(one))]);
    assert_eq!((a.off, a.far), (1, 1), "two steps off");
    let nan = accuracy(&[[f32::NAN, 1.0], [1.0, f32::NAN]], &[0.0, 0.0]);
    assert_eq!(nan, Accuracy::default(), "a NaN operand is skipped");
    let pairs = [[0.0, 1.0], [0.0, 2.0], [0.0, 3.0]];
    let tie = accuracy(&pairs, &[0.0, f32::from_bits(1), f32::from_bits(1)]);
    assert_eq!(tie.worst, 1.0, "the worst error");
    assert_eq!(tie.worst_at, [0.0, 2.0], "the first of equal worst errors");
    let low = accuracy(&pairs[..1], &[f32::from_bits(2)]);
    let both = tie.merge(low);
    assert_eq!((both.n, both.off, both.far), (4, 3, 1), "merged counts");
    assert_eq!(
        (both.worst, both.worst_at),
        (2.0, [0.0, 1.0]),
        "merged worst from the larger"
    );
    let back = low.merge(tie);
    assert_eq!(
        (back.worst, back.worst_at),
        (2.0, [0.0, 1.0]),
        "merged worst, either order"
    );
    let later = accuracy(&pairs[2..], &[f32::from_bits(1)]);
    assert_eq!(later.worst_at, [0.0, 3.0], "a later equal worst");
    assert_eq!(
        tie.merge(later).worst_at,
        [0.0, 2.0],
        "merging equal worsts keeps the first"
    );
}

#[test]
fn arctan_measure_accuracy() {
    check_accuracy(core::f32::consts::FRAC_PI_4);
}

negative_control!(
    arctan_measure_accuracy,
    "the f32 below π/4 taken for its nearest",
    expected = "the nearest f32 is not off",
    check_accuracy(f32::from_bits(core::f32::consts::FRAC_PI_4.to_bits() - 1))
);

/// Checks [`RoundTrip`]; `moved` is the angle `α'` the round trip of `α = 1` is taken to give.
fn check_round_trip(moved: f32) {
    let mut r = RoundTrip::default();
    r.add(1.0, [0.5, 0.25], 1.0, [0.5, 0.25]);
    assert_eq!(
        (r.n, r.moved, r.worst_steps),
        (1, 0, 0),
        "an angle that comes back"
    );
    let mut m = RoundTrip::default();
    m.add(1.0, [0.5, 0.25], moved, [0.75, 0.375]);
    assert_eq!(
        (m.n, m.moved, m.worst_steps),
        (1, 1, 2),
        "an angle moved two steps"
    );
    assert_eq!(
        (m.worst_cos, m.worst_sin),
        (0.5, 0.5),
        "the relative errors"
    );
    let mut z = RoundTrip::default();
    z.add(0.0, [0.0, 1.0], 0.0, [1.0, 2.0]);
    assert_eq!(z.worst_sin, 0.0, "a zero sin is skipped");
    assert_eq!(z.worst_cos, 1.0, "the cos error");
    let mut c = RoundTrip::default();
    c.add(1.0, [1.0, 0.0], 1.0, [3.0, 1.0]);
    assert_eq!(
        (c.worst_cos, c.worst_sin),
        (0.0, 2.0),
        "a zero cos is skipped"
    );
    let all = r.merge(m).merge(z).merge(c);
    assert_eq!(
        (all.n, all.moved, all.worst_steps),
        (4, 1, 2),
        "merged counts"
    );
    assert_eq!(
        (all.worst_cos, all.worst_sin),
        (1.0, 2.0),
        "merged largest errors"
    );
    assert_eq!(all.share(), 0.25, "the share moved");
    let back = c.merge(z).merge(m).merge(r);
    assert_eq!(back, all, "merging in either order");
    let mut lower = m;
    lower.add(1.0, [1.0, 1.0], 1.0, [1.25, 1.25]);
    assert_eq!(
        (lower.worst_steps, lower.worst_cos, lower.worst_sin),
        (2, 0.5, 0.5),
        "a smaller error does not replace the largest"
    );
}

#[test]
fn arctan_measure_round_trip() {
    check_round_trip(up(up(1.0)));
}

negative_control!(
    arctan_measure_round_trip,
    "the round trip taken to move one step, not two",
    expected = "an angle moved two steps",
    check_round_trip(up(1.0))
);

/// Checks [`reduced`] and [`alphas`] at `stride`, taking `half_pi` as the bits of the largest f32 below `π/2`.
fn check_ranges(stride: u32, half_pi: u32) {
    let below = f32::from_bits(half_pi);
    assert!(
        f64::from(below) < core::f64::consts::FRAC_PI_2
            && f64::from(up(below)) > core::f64::consts::FRAC_PI_2,
        "{half_pi:#010x} is the largest f32 below π/2"
    );
    let t: Vec<f32> = reduced(stride).collect();
    assert_eq!(
        t.len() as u32,
        ONE_BITS / stride + 2,
        "the reduced sweep's count"
    );
    assert_eq!(t[0].to_bits(), 0, "the reduced sweep starts at +0");
    assert_eq!(
        t[1].to_bits(),
        stride,
        "the reduced sweep steps by its stride"
    );
    assert_eq!(*t.last().unwrap(), 1.0, "the reduced sweep ends at 1");
    assert!(t.iter().all(|&v| (0.0..=1.0).contains(&v)), "t ∈ [0, 1]");
    let a: Vec<f32> = alphas(stride).collect();
    assert_eq!(
        a.len() as u32,
        half_pi / stride + 1,
        "the round trip's count"
    );
    assert_eq!(a[0].to_bits(), 0, "the round trip starts at +0");
    assert_eq!(a[1].to_bits(), stride, "the round trip steps by its stride");
    let last = a.last().unwrap().to_bits();
    assert!(
        last <= half_pi && half_pi - last < stride,
        "the round trip ends below π/2"
    );
}

#[test]
fn arctan_sweep_ranges() {
    check_ranges(1021, HALF_PI_BELOW_BITS);
    check_ranges(1 << 20, HALF_PI_BELOW_BITS);
    let all: Vec<f32> = alphas(HALF_PI_BELOW_BITS).collect();
    assert_eq!(
        all.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        [0, HALF_PI_BELOW_BITS],
        "stride HALF_PI_BELOW_BITS: +0 and the largest f32 below π/2"
    );
    let ends: Vec<f32> = reduced(ONE_BITS).collect();
    assert_eq!(ends, [0.0, 1.0, 1.0], "stride ONE_BITS: +0, 1 and 1");
}

negative_control!(
    arctan_sweep_ranges,
    "f32's nearest π/2, which lies above it, taken as the largest f32 below",
    expected = "is the largest f32 below π/2",
    check_ranges(1021, HALF_PI_BELOW_BITS + 1)
);

/// FNV-1a over the pairs' bits.
fn checksum(yx: &[[f32; 2]]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for v in yx.as_flattened() {
        for b in v.to_bits().to_le_bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3);
        }
    }
    h
}

/// The plane sweep's checksum at 2¹⁶ pairs from seed `0x0628_0002`, the engine's sweep's seed; recorded from the sweep
/// the PR's results were measured on.
const PLANE_CHECKSUM: u64 = 0x53D0_F980_1D61_E64D;

/// Checks [`plane`]'s `n` pairs from `seed`, and its checksum against `want` at the engine's seed.
fn check_plane(n: usize, seed: u64, want: u64) {
    let p = plane(n, seed);
    assert_eq!(p.len(), n, "n pairs");
    assert_eq!(p, plane(n, seed), "the same seed, the same sweep");
    assert_ne!(p, plane(n, seed + 1), "another seed, another sweep");
    assert!(
        p.as_flattened().iter().all(|v| v.is_finite()),
        "every operand is finite"
    );
    let share = |f: &dyn Fn(&[f32; 2]) -> bool| p.iter().filter(|q| f(q)).count() as f64 / n as f64;
    let near = |got: f64, want: f64, what: &str| {
        assert!(
            (got - want).abs() < 0.2 * want,
            "{what}: share {got}, not about {want}"
        );
    };
    near(share(&|q| q[0] == 0.0 && q[1] != 0.0), 1.0 / 16.0, "y = ±0");
    near(share(&|q| q[1] == 0.0 && q[0] != 0.0), 1.0 / 16.0, "x = ±0");
    near(
        share(&|q| q[0].abs() == q[1].abs() && q[0] != 0.0),
        1.0 / 16.0,
        "|y| = |x|",
    );
    near(
        share(&|q| q[0] == 0.0 && q[1] == 0.0),
        1.0 / 256.0,
        "both ±0",
    );
    near(share(&|q| q[0].is_sign_negative()), 0.5, "negative y");
    near(share(&|q| q[1].is_sign_negative()), 0.5, "negative x");
    near(
        share(&|q| q[0].is_sign_negative() == q[1].is_sign_negative()),
        0.5,
        "signs independent",
    );
    near(
        share(&|q| q[0] != 0.0 && q[0].is_subnormal()),
        1.0 / 255.0 * 15.0 / 16.0,
        "subnormal y",
    );
    let fields: std::collections::BTreeSet<u32> = p
        .iter()
        .filter(|q| q[1] != 0.0)
        .map(|q| q[1].to_bits() >> 23 & 0xFF)
        .collect();
    assert_eq!(fields.len(), 255, "x spans every finite exponent field");
    let fractions = p
        .iter()
        .filter(|q| q[0].to_bits() & 0x007F_FFFF != 0)
        .count();
    assert!(fractions as f64 > 0.8 * n as f64, "the fractions are drawn");
    if seed == 0x0628_0002 && n == 1 << 16 {
        assert_eq!(checksum(&p), want, "the plane sweep's checksum");
    }
}

#[test]
fn arctan_sweep_plane() {
    check_plane(1 << 16, 0x0628_0002, PLANE_CHECKSUM);
}

negative_control!(
    arctan_sweep_plane,
    "the checksum of the sweep one seed on",
    expected = "the plane sweep's checksum",
    check_plane(1 << 16, 0x0628_0002, checksum(&plane(1 << 16, 0x0628_0003)))
);
