//! QA tests for TASK-M2-50 (REQ-VAL-022, REQ-VAL-121, REQ-VAL-193), written from the requirements, R-551 and RQ-624's
//! decision (applied per R-369), not from the gate's code: the shape families, the gauge elements and the readings of
//! `w` and `z_β` are this file's own, on `kernel::encode` and `kernel::shape` directly.
//!
//! - REQ-VAL-022, the bit-for-bit check: every collinear state's `z_β` at its clamp image (the β link's inverse of an
//!   input inside the clamp's band, `ε_z = 10⁻⁶`), bit for bit, with its `lookup_clamped` bit set; no equal-mass
//!   equilateral state sets it; a state moved past `π·ε_z` leaves the clamp.
//! - REQ-VAL-022, the pole and the equator, read on the canonical state `C(x)` (RQ-624): `|w − 1|` (L⁺ and L⁻, `C`'s
//!   mirror taking L⁻ to `w = +1`) and `|w|` within REQ-VAL-121's proposed bounds; a collinear state moved about 10³
//!   ulps off the line keeps `z_β` clamped and is caught by `|w|`; the proposed bounds sit on the measurement.
//! - REQ-VAL-193: the Randomise draw is the stated distribution (masses on `[0.4, 1.8]` normalised, positions on `±0.9`,
//!   velocities on `±0.6`, each uniform); the arm's reported maxima, over all the draws and over the in-ball subset, are
//!   `‖E(h·x) − E(x)‖` re-measured here; and the arm never moves REQ-VAL-021's verdict at 1.6×10⁻¹⁴, either way.
//!
//! Each test has its negative control (R-176).

use std::f64::consts::PI;
use std::path::{Path, PathBuf};

use kernel::decode::{self, mass, Ic, Latent};
use kernel::encode::encode;
use kernel::generated::links::{Defaults, Link, Selection};
use kernel::shape::shape;
use validation::decode_f32_eps::SplitMix;
use validation::gate::{self, GateConfig, Verdict};
use validation::inspector::{
    randomise_ic, randomise_moves, randomise_seed, randomise_worst, HandlesInput,
    InspectorHandlesGate, Leak, RandomiseArm, RandomiseSeed, EQUATOR_BOUND, HANDLES, MOVES,
    POLE_BOUND,
};
use validation::negative_control;

/// REQ-VAL-021's bound, which the Randomise arm never loosens (R-551).
const HANDLE_BOUND: f64 = 1.6e-14;
/// The β link's clamp, `ε_z` (R-10, R-21; inverse-encode contract Part 3).
const EPS_Z: f64 = 1e-6;
/// The draws of each shape family per seed, the gate's own count.
const N_SHAPES: usize = 2000;

/// SplitMix64, this file's own generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform on `[lo, hi)`.
    fn on(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 * (-53f64).exp2())
    }
}

/// `g·x` for `g = (a, u, θ, s, Π)`: `rᵢ' = s·R(θ)·Π·rᵢ + a`, `pᵢ' = s^{−1/2}·R(θ)·Π·pᵢ + mᵢu` (dd_encode §3.3).
fn act(x: &Ic<f64>, a: [f64; 2], u: [f64; 2], theta: f64, s: f64, mirror: bool) -> Ic<f64> {
    let (sin, cos) = theta.sin_cos();
    let turn = |v: [f64; 2]| {
        let y = if mirror { -v[1] } else { v[1] };
        [cos * v[0] - sin * y, sin * v[0] + cos * y]
    };
    Ic {
        m: x.m,
        r: x.r.map(|v| {
            let t = turn(v);
            [s * t[0] + a[0], s * t[1] + a[1]]
        }),
        p: std::array::from_fn(|i| {
            let t = turn(x.p[i]);
            [
                t[0] / s.sqrt() + x.m[i] * u[0],
                t[1] / s.sqrt() + x.m[i] * u[1],
            ]
        }),
    }
}

/// A random gauge element of RQ-475's families (REQ-VAL-022's verify): scale `2^[−4, 4)`, any turn, either parity,
/// offset and boost on `[−2, 2)²`. Returns the image and whether it mirrored.
fn gauge(rng: &mut Rng, x: &Ic<f64>) -> (Ic<f64>, bool) {
    let s = rng.on(-4.0, 4.0).exp2();
    let theta = rng.on(-PI, PI);
    let mirror = rng.next() & 1 == 1;
    let a = [rng.on(-2.0, 2.0), rng.on(-2.0, 2.0)];
    let u = [rng.on(-2.0, 2.0), rng.on(-2.0, 2.0)];
    (act(x, a, u, theta, s, mirror), mirror)
}

/// `z_β`'s two clamp images: the β link's inverse of an input a quarter of the band inside each end, `β = π·ε_z/4` and
/// `β = π − π·ε_z/4`, which the clamp takes to `s = ε_z` and `s = 1 − ε_z`.
fn clamp_images() -> [f64; 2] {
    type BetaLink = <Defaults as Selection>::BetaLink;
    let band = PI * EPS_Z / 4.0;
    [band, PI - band].map(|b| BetaLink::inverse([b])[0])
}

/// What this file reads off RQ-475's two families.
struct Measured {
    /// The equilateral states' `|w − 1|` on `C(x)`.
    pole: Vec<f64>,
    /// The equilateral states' `|w − 1|` on `x` itself, before `C`.
    pole_raw: Vec<f64>,
    /// How many equilateral states the gauge mirrored, and how many of those read `w = −1` before `C` (L⁻).
    mirrored: [usize; 2],
    /// The collinear states' `|w|` on `C(x)`.
    equator: Vec<f64>,
    /// The collinear states' `|w|` on `D(E(x))`.
    equator_back: Vec<f64>,
    /// `[equilateral, collinear]`: how many set `lookup_clamped`'s β bit.
    bit: [usize; 2],
    /// `[equilateral, collinear]`: how many have `z_β` bit for bit at one of its clamp images.
    image: [usize; 2],
}

/// RQ-475's families from `seed`, `n` of each: the equal-mass unit equilateral triangle (L⁺; the gauge's parity gives
/// L⁻), and three bodies on the `x`-axis at `[−1, 1)` with masses from `z_μ` on `[−4, 4)²`, body 2 raised by `lift`
/// off the line; each at rest, then through a random gauge element. `nudge` moves each `z_β` one ulp before it is
/// compared.
fn measure(seed: u64, n: usize, lift: f64, nudge: bool) -> Measured {
    let mut rng = Rng(seed);
    let images = clamp_images();
    let equal = mass::decode::<Defaults, f64>([0.0, 0.0]).m;
    let mut out = Measured {
        pole: Vec::new(),
        pole_raw: Vec::new(),
        mirrored: [0; 2],
        equator: Vec::new(),
        equator_back: Vec::new(),
        bit: [0; 2],
        image: [0; 2],
    };
    for _ in 0..n {
        let tri = Ic {
            m: equal,
            r: [[0.0, 0.0], [1.0, 0.0], [0.5, 3f64.sqrt() / 2.0]],
            p: [[0.0; 2]; 3],
        };
        let z_mu = [rng.on(-4.0, 4.0), rng.on(-4.0, 4.0)];
        let t: [f64; 3] = std::array::from_fn(|_| rng.on(-1.0, 1.0));
        let line = Ic {
            m: mass::decode::<Defaults, f64>(z_mu).m,
            r: [[t[0], 0.0], [t[1], 0.0], [t[2], lift]],
            p: [[0.0; 2]; 3],
        };
        for (k, x0) in [tri, line].iter().enumerate() {
            let (x, mirrored) = gauge(&mut rng, x0);
            let e = encode::<Defaults, f64>(&x);
            let c = e.canonical.ic;
            let w = shape(c.r, c.m)[2];
            if k == 0 {
                out.pole.push((w - 1.0).abs());
                let raw = shape(x.r, x.m)[2];
                out.pole_raw.push((raw - 1.0).abs());
                if mirrored {
                    out.mirrored[0] += 1;
                    out.mirrored[1] += usize::from((raw + 1.0).abs() < 1e-12);
                }
            } else {
                out.equator.push(w.abs());
                let back = decode::decode::<Defaults, f64>(&Latent::new(e.z.z)).ic;
                out.equator_back.push(shape(back.r, back.m)[2].abs());
            }
            let mut z_beta = e.z.z[1];
            if nudge {
                z_beta = f64::from_bits(z_beta.to_bits() + 1);
            }
            out.bit[k] += usize::from(e.report.clamped_controls & 0b10 != 0);
            out.image[k] += usize::from(images.iter().any(|i| i.to_bits() == z_beta.to_bits()));
        }
    }
    out
}

fn max(v: &[f64]) -> f64 {
    v.iter().fold(0.0f64, |a, &b| {
        assert!(!b.is_nan(), "a NaN reading");
        a.max(b)
    })
}

/// This file's seeds for the shape families.
const SEEDS: [u64; 3] = [0x5157_0501, 0x5157_0502, 0x5157_0503];

// ── The bit-for-bit check ───────────────────────────────────────────────────────────────────────────────────────

/// The clamp images are the logit's at `ε_z` and `1 − ε_z` (the β link's inverse, `z = logit(s)`, `s` clamped to
/// `[ε_z, 1 − ε_z]`), to a few ulps: so "at its clamp image" is at the clamp's, not at some other constant.
fn check_clamp_images(eps: f64) {
    let logit = |s: f64| (s / (1.0 - s)).ln();
    let want = [logit(eps), logit(1.0 - eps)];
    let got = clamp_images();
    for (g, w) in got.iter().zip(want) {
        assert!(
            (g - w).abs() <= 1e-12 * w.abs(),
            "a clamp image {g:e} is not logit at the clamp, {w:e}"
        );
    }
    type BetaLink = <Defaults as Selection>::BetaLink;
    let edges = [0.0, PI].map(|b| BetaLink::inverse([b])[0]);
    assert_eq!(
        edges.map(f64::to_bits),
        got.map(f64::to_bits),
        "β = 0 and β = π do not clamp to the band's image, bit for bit"
    );
}

#[test]
fn qa_clamp_images_are_the_clamps() {
    check_clamp_images(EPS_Z);
}

negative_control!(
    qa_clamp_images_are_the_clamps,
    "a clamp at 10⁻⁵, not ε_z, must not match",
    expected = "is not logit at the clamp",
    check_clamp_images(1e-5)
);

/// REQ-VAL-022 (R-551): every collinear state's `z_β` at its clamp image, bit for bit, with its bit set; no
/// equilateral state's bit set, nor its `z_β` at an image.
fn check_bits(nudge: bool) {
    for seed in SEEDS {
        let m = measure(seed, N_SHAPES, 0.0, nudge);
        assert_eq!(
            m.bit[1], N_SHAPES,
            "a collinear state without lookup_clamped's β bit"
        );
        assert_eq!(
            m.image[1], N_SHAPES,
            "a collinear z_β not at its clamp image, bit for bit"
        );
        assert_eq!(m.bit[0], 0, "an equilateral state sets the β bit");
        assert_eq!(m.image[0], 0, "an equilateral z_β at a clamp image");
    }
}

#[test]
fn qa_beta_bit_for_bit() {
    check_bits(false);
}

negative_control!(
    qa_beta_bit_for_bit,
    "z_β one ulp off its clamp image must fail the bit-for-bit check",
    expected = "a collinear z_β not at its clamp image",
    check_bits(true)
);

/// The bit check tells collinear from not: body 2 raised `lift` off the line, enough to put `β` past `π·ε_z`, leaves
/// every state off the clamp, its bit unset (RQ-624 (2), the optional control).
fn check_leaves_clamp(lift: f64) {
    let m = measure(SEEDS[0], N_SHAPES, lift, false);
    assert!(
        m.bit[1] == 0 && m.image[1] == 0,
        "a state {lift:e} off the line still clamps: bit in {}, image in {} of {N_SHAPES}",
        m.bit[1],
        m.image[1]
    );
}

#[test]
fn qa_beta_past_band_unclamps() {
    check_leaves_clamp(1e-2);
}

negative_control!(
    qa_beta_past_band_unclamps,
    "a state on the line must still clamp",
    expected = "off the line still clamps",
    check_leaves_clamp(0.0)
);

// ── The pole and the equator, on C(x) ───────────────────────────────────────────────────────────────────────────

/// REQ-VAL-022: the equal-mass equilateral family's `max |w − 1|`, read on `C(x)` (or on `x` itself, the control),
/// within the proposed pole bound; both L⁺ and L⁻ drawn, the mirrored ones at `w = −1` before `C`.
fn check_pole(on_canonical: bool) {
    for seed in SEEDS {
        let m = measure(seed, N_SHAPES, 0.0, false);
        assert!(
            m.mirrored[0] > N_SHAPES * 2 / 5 && m.mirrored[1] == m.mirrored[0],
            "L⁻ is not drawn: {:?} of {N_SHAPES}",
            m.mirrored
        );
        let w = max(if on_canonical { &m.pole } else { &m.pole_raw });
        assert!(
            w <= POLE_BOUND,
            "the pole's |w − 1| reaches {w:e}, past {POLE_BOUND:e}"
        );
    }
}

#[test]
fn qa_pole_on_canonical_state() {
    check_pole(true);
}

negative_control!(
    qa_pole_on_canonical_state,
    "|w − 1| read before C's mirror, L⁻ at w = −1, must fail",
    expected = "the pole's |w − 1| reaches",
    check_pole(false)
);

/// REQ-VAL-022 (RQ-624 (1a)): the collinear family's `max |w|`, read on `C(x)` (or on `D(E(x))`, the control, where it
/// is the clamp image `sin2α·sin(π·ε_z)`), within the proposed equator bound.
fn check_equator(on_canonical: bool) {
    for seed in SEEDS {
        let m = measure(seed, N_SHAPES, 0.0, false);
        let back = max(&m.equator_back);
        eprintln!(
            "seed {seed:#x}: |w| on C(x) max {:.3e}; on D(E(x)), reported: max {back:.3e} (sin(π·ε_z) = {:.3e})",
            max(&m.equator),
            (PI * EPS_Z).sin()
        );
        assert!(
            back <= (PI * EPS_Z).sin() * (1.0 + 1e-9),
            "D(E(x))'s |w|, {back:e}, past the clamp image"
        );
        let w = max(if on_canonical {
            &m.equator
        } else {
            &m.equator_back
        });
        assert!(
            w <= EQUATOR_BOUND,
            "the equator's |w| reaches {w:e}, past {EQUATOR_BOUND:e}"
        );
    }
}

#[test]
fn qa_equator_on_canonical_state() {
    check_equator(true);
}

negative_control!(
    qa_equator_on_canonical_state,
    "|w| read on D(E(x)), the clamp image, must fail the equator's bound",
    expected = "the equator's |w| reaches",
    check_equator(false)
);

/// RQ-624 (2): the collinear family moved `ulps` ulps of 1 off the line (body 2's `y`, before the gauge) keeps every
/// `z_β` at its clamp image with its bit set, bit for bit, while `|w|` on `C(x)` catches it, several times the bound.
fn check_in_band(ulps: u32) {
    let m = measure(SEEDS[1], N_SHAPES, f64::from(ulps) * f64::EPSILON, false);
    assert!(
        m.bit[1] == N_SHAPES && m.image[1] == N_SHAPES,
        "the lifted states leave the clamp: bit in {}, image in {} of {N_SHAPES}",
        m.bit[1],
        m.image[1]
    );
    let w = max(&m.equator);
    assert!(
        w > 4.0 * EQUATOR_BOUND,
        "{ulps} ulps off the line is not caught by |w|: max {w:e} against {EQUATOR_BOUND:e}"
    );
}

#[test]
fn qa_off_line_caught_by_w() {
    check_in_band(1000);
}

negative_control!(
    qa_off_line_caught_by_w,
    "a state on the line must not be caught",
    expected = "off the line is not caught by |w|",
    check_in_band(0)
);

/// REQ-VAL-121 (R-71): each proposed shape bound passes its measurement over this file's own draws, and sits on it, at
/// most 4× its worst, so the bound can fire; a bound one ulp below the measurement fails.
fn check_bounds(pole_bound: f64, equator_bound: f64) {
    let (mut pole, mut equator) = (0.0f64, 0.0f64);
    for seed in SEEDS {
        let m = measure(seed, N_SHAPES, 0.0, false);
        pole = pole.max(max(&m.pole));
        equator = equator.max(max(&m.equator));
    }
    eprintln!("this file's worst: |w − 1| {pole:e}, |w| {equator:e}");
    assert!(
        pole > 0.0 && pole <= pole_bound && equator <= equator_bound,
        "a shape bound below its measurement: |w − 1| {pole:e} against {pole_bound:e}, |w| {equator:e} against \
         {equator_bound:e}"
    );
    assert!(
        pole_bound <= 4.0 * pole && equator_bound <= 4.0 * equator,
        "a shape bound not from its measurement: |w − 1| {pole:e} against {pole_bound:e}, |w| {equator:e} against \
         {equator_bound:e}"
    );
}

#[test]
fn qa_shape_bounds_from_measurement() {
    check_bounds(POLE_BOUND, EQUATOR_BOUND);
}

negative_control!(
    qa_shape_bounds_from_measurement,
    "the pole's bound one ulp below 2⁻⁵², the measured worst, must fail",
    expected = "a shape bound below its measurement",
    check_bounds(f64::from_bits(f64::EPSILON.to_bits() - 1), EQUATOR_BOUND)
);

// ── REQ-VAL-193: the Randomise arm ──────────────────────────────────────────────────────────────────────────────

/// A wrong Randomise, the control's: positions on `±1.0`, not `±0.9`.
#[cfg(feature = "controls")]
fn wide_randomise(rng: &mut SplitMix) -> Ic<f64> {
    let mut x = randomise_ic(rng);
    x.r = x.r.map(|v| v.map(|c| c / 0.9));
    x
}

/// The Randomise draw is REQ-VAL-193's distribution: `Σmᵢ = 1`, each ratio `mᵢ/mⱼ` inside `[0.4/1.8, 1.8/0.4]` and
/// reaching near its ends; positions uniform on `±0.9` and velocities `pᵢ/mᵢ` on `±0.6`, each inside its range,
/// reaching its ends, with the uniform's mean 0 and variance `a²/3`, to five standard errors.
fn check_distribution(draw: fn(&mut SplitMix) -> Ic<f64>) {
    const N: usize = 20_000;
    let mut rng = SplitMix::new(0x5157_0193);
    let (mut pos, mut vel, mut ratio) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..N {
        let x = draw(&mut rng);
        assert!(
            (x.m.iter().sum::<f64>() - 1.0).abs() <= 2.0 * f64::EPSILON,
            "the masses are not normalised: {:?}",
            x.m
        );
        for i in 0..3 {
            ratio.push(x.m[i] / x.m[(i + 1) % 3]);
            for k in 0..2 {
                pos.push(x.r[i][k]);
                vel.push(x.p[i][k] / x.m[i]);
            }
        }
    }
    let (lo, hi) = (0.4 / 1.8, 1.8 / 0.4);
    let (rmin, rmax) = ratio
        .iter()
        .fold((f64::INFINITY, 0.0f64), |(a, b), &r| (a.min(r), b.max(r)));
    assert!(
        rmin >= lo * (1.0 - 1e-12) && rmax <= hi * (1.0 + 1e-12),
        "a mass ratio outside [0.4/1.8, 1.8/0.4]: {rmin} to {rmax}"
    );
    assert!(
        rmin < 1.1 * lo && rmax > 0.9 * hi,
        "the mass ratios do not reach their ends: {rmin} to {rmax}"
    );
    for (v, a, word) in [(&pos, 0.9, "positions"), (&vel, 0.6, "velocities")] {
        let n = v.len() as f64;
        let (vmin, vmax) = v
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(p, q), &c| {
                (p.min(c), q.max(c))
            });
        assert!(
            vmin >= -a * (1.0 + 1e-12) && vmax <= a * (1.0 + 1e-12),
            "{word} outside ±{a}: {vmin} to {vmax}"
        );
        assert!(
            vmin < -0.99 * a && vmax > 0.99 * a,
            "{word} do not reach ±{a}: {vmin} to {vmax}"
        );
        let mean = v.iter().sum::<f64>() / n;
        let var = v.iter().map(|c| c * c).sum::<f64>() / n;
        let (want, se_mean, se_var) = (
            a * a / 3.0,
            (a * a / 3.0 / n).sqrt(),
            (4.0 * a.powi(4) / 45.0 / n).sqrt(),
        );
        assert!(
            mean.abs() <= 5.0 * se_mean && (var - want).abs() <= 5.0 * se_var,
            "{word} not uniform on ±{a}: mean {mean:e}, variance {var} against {want}"
        );
    }
}

#[test]
fn qa_randomise_distribution() {
    check_distribution(randomise_ic);
}

negative_control!(
    qa_randomise_distribution,
    "positions drawn on ±1.0 must fail",
    expected = "positions outside ±0.9",
    check_distribution(wide_randomise)
);

/// `‖E(y) − E(x)‖`.
fn gap(x: &Ic<f64>, y: &Ic<f64>) -> f64 {
    let (a, b) = (
        encode::<Defaults, f64>(x).z.z,
        encode::<Defaults, f64>(y).z.z,
    );
    a.iter()
        .zip(b)
        .map(|(p, q)| (p - q) * (p - q))
        .sum::<f64>()
        .sqrt()
}

/// The arm's measurement is REQ-VAL-193's: re-drawn here from the same stream, each of the six moves is the handle it
/// names (only its own component off the identity, the buttons at 15° and ×1.25), each `‖Δz‖` is `‖E(h·x) − E(x)‖`
/// read by this file's own gauge action (to `10⁻¹³`, the actions' roundings apart), the in-ball flag is
/// `‖E(x)‖ < 4`, and the reported worst, over all the draws and over the in-ball subset, is this file's maximum.
fn check_arm_measures(leak: Leak) {
    const ICS: u32 = 400;
    for seed in [1u64, 7] {
        let arm: RandomiseSeed = randomise_seed(seed, ICS, 4.0, 4.0, leak);
        assert_eq!(arm.gaps.len(), ICS as usize, "the draws' count");
        let mut rng = SplitMix::new(seed);
        let mut mine = vec![[0.0f64; 6]; ICS as usize];
        for (j, row) in mine.iter_mut().enumerate() {
            let x = randomise_ic(&mut rng);
            let g = randomise_moves(&mut rng, 4.0);
            let id = |k: usize| {
                let mut f = [false; 5];
                f[0] = g[k].a != [0.0; 2];
                f[1] = g[k].u != [0.0; 2];
                f[2] = g[k].theta != 0.0;
                f[3] = g[k].s != 1.0;
                f[4] = g[k].mirror;
                f
            };
            let own = [0, 1, 2, 2, 3, 3];
            for k in 0..6 {
                let f = id(k);
                assert!(
                    (0..5).all(|c| f[c] == (c == own[k])),
                    "move {k}, {}, is not its own handle: {:?}",
                    MOVES[k],
                    g[k]
                );
            }
            assert!(
                (g[3].theta - PI / 12.0).abs() <= f64::EPSILON && g[5].s == 1.25,
                "the buttons are not 15° and ×1.25"
            );
            let images = [
                act(&x, g[0].a, [0.0; 2], 0.0, 1.0, false),
                act(&x, [0.0; 2], g[1].u, 0.0, 1.0, false),
                act(&x, [0.0; 2], [0.0; 2], g[2].theta, 1.0, false),
                act(&x, [0.0; 2], [0.0; 2], g[3].theta, 1.0, false),
                act(&x, [0.0; 2], [0.0; 2], 0.0, g[4].s, false),
                act(&x, [0.0; 2], [0.0; 2], 0.0, g[5].s, false),
            ];
            for (k, y) in images.iter().enumerate() {
                row[k] = gap(&x, y);
            }
            let z = encode::<Defaults, f64>(&x).z.z;
            let inside = z.iter().map(|c| c * c).sum::<f64>().sqrt() < 4.0;
            assert_eq!(arm.in_ball[j], inside, "draw {j}'s in-ball flag");
            for k in 0..6 {
                assert!(
                    (arm.gaps[j][k] - row[k]).abs() <= 1e-13,
                    "draw {j}, {}: the arm's ‖Δz‖ {:e} is not ‖E(h·x) − E(x)‖, {:e}",
                    MOVES[k],
                    arm.gaps[j][k],
                    row[k]
                );
            }
        }
        for k in 0..6 {
            for ball_only in [false, true] {
                let want = arm
                    .gaps
                    .iter()
                    .zip(&arm.in_ball)
                    .filter(|(_, &b)| b || !ball_only)
                    .fold(0.0f64, |a, (g, _)| a.max(g[k]));
                let count = arm.in_ball.iter().filter(|&&b| b || !ball_only).count();
                assert_eq!(
                    randomise_worst(&arm, k, ball_only),
                    (want, count),
                    "{}'s reported worst (in the ball only: {ball_only})",
                    MOVES[k]
                );
            }
        }
    }
}

#[test]
fn qa_randomise_arm_measures() {
    check_arm_measures(Leak::None);
}

negative_control!(
    qa_randomise_arm_measures,
    "an arm measured with C's rotation dropped is not ‖E(h·x) − E(x)‖",
    expected = "is not ‖E(h·x) − E(x)‖",
    check_arm_measures(Leak::Rotate)
);

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn handles(leak: Leak, arm: Option<RandomiseArm>) -> HandlesInput {
    HandlesInput {
        description: "qa".to_owned(),
        seed: 9,
        ics: 1000,
        radius: 4.0,
        span: 1.0,
        octaves: 4.0,
        leak,
        randomise: arm,
    }
}

/// R-551, REQ-VAL-193: the gate's threshold is 1.6×10⁻¹⁴ from `gate.json`; the shipped input runs the arm unfed; an arm
/// whose worst is past 1.6×10⁻¹⁴ leaves REQ-VAL-021's verdict and its judged lines as they are without it, a pass; and
/// it does not rescue a REQ-VAL-021 failure (a rotation leak). `fed` is the control's.
fn check_never_loosens(fed: bool) {
    let config: GateConfig = gate::config(&root(), HANDLES).expect("the handles gate's gate.json");
    assert_eq!(
        config.threshold.value, HANDLE_BOUND,
        "REQ-VAL-021's threshold is not 1.6×10⁻¹⁴"
    );
    let shipped: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("fixtures/gates/inspector-handles/random.json"))
            .expect("random.json"),
    )
    .expect("random.json parses");
    assert!(
        shipped["randomise"]["fed"] == false && shipped["expected"] == "pass",
        "the shipped input feeds the arm into REQ-VAL-021's verdict"
    );
    let arm = RandomiseArm {
        seeds: vec![4, 6],
        ics: 2000,
        fed,
    };
    let arm_worst = arm
        .seeds
        .iter()
        .map(|&s| randomise_seed(s, arm.ics, 4.0, 4.0, Leak::None))
        .flat_map(|s| (0..6).map(move |k| randomise_worst(&s, k, false).0))
        .fold(0.0f64, f64::max);
    assert!(
        arm_worst > HANDLE_BOUND,
        "the arm's worst, {arm_worst:e}, cannot test the separation"
    );
    let judge = |i: &HandlesInput| InspectorHandlesGate::judge(&config, "qa", i).expect("judged");
    let without = judge(&handles(Leak::None, None));
    let with = judge(&handles(Leak::None, Some(arm.clone())));
    assert_eq!(without.verdict, Verdict::Pass, "REQ-VAL-021's arm fails");
    assert_eq!(
        with.verdict,
        Verdict::Pass,
        "REQ-VAL-021's verdict reads the Randomise arm"
    );
    assert_eq!(
        with.negative_results[..4],
        without.negative_results[..4],
        "the Randomise arm moves REQ-VAL-021's judged lines"
    );
    assert_eq!(
        with.threshold.value, HANDLE_BOUND,
        "the judged threshold moved"
    );
    let leak = judge(&handles(Leak::Rotate, Some(arm)));
    assert_eq!(
        leak.verdict,
        Verdict::Fail,
        "the Randomise arm rescues a failing REQ-VAL-021 arm"
    );
}

#[test]
fn qa_randomise_never_loosens() {
    check_never_loosens(false);
}

negative_control!(
    qa_randomise_never_loosens,
    "the arm's maximum fed into REQ-VAL-021's verdict must fail it",
    expected = "REQ-VAL-021's verdict reads the Randomise arm",
    check_never_loosens(true)
);

/// REQ-VAL-193's report: for each of the six moves, the gate prints the arm's maximum over all the draws and, beside it,
/// over the in-ball subset, each equal to this file's own maximum over the arm's draws (a seed reported alone, and
/// both seeds together).
fn check_report(swap: bool) {
    let config: GateConfig = gate::config(&root(), HANDLES).expect("the handles gate's gate.json");
    let arm = RandomiseArm {
        seeds: vec![2, 3],
        ics: 500,
        fed: false,
    };
    let seeds: Vec<RandomiseSeed> = arm
        .seeds
        .iter()
        .map(|&s| randomise_seed(s, arm.ics, 4.0, 4.0, Leak::None))
        .collect();
    let report = InspectorHandlesGate::judge(&config, "qa", &handles(Leak::None, Some(arm)))
        .expect("judged");
    for (k, word) in MOVES.iter().enumerate() {
        let worst = |ball: bool| {
            seeds
                .iter()
                .map(|s| randomise_worst(s, k, ball).0)
                .fold(0.0f64, f64::max)
        };
        let (all, inside) = if swap {
            (worst(true), worst(false))
        } else {
            (worst(false), worst(true))
        };
        let line = report
            .scatter
            .iter()
            .find(|l| l.contains(&format!("reported: {word}:")))
            .unwrap_or_else(|| panic!("no Randomise line for {word}"));
        let at = line.find("in the ball").expect("no in-ball part");
        let (head, tail) = line.split_at(at);
        assert!(
            head.contains(&format!("max ‖Δz‖ {all:.3e} over 1000 draws"))
                && tail.contains(&format!("max {inside:.3e} (by seed")),
            "{word}: the reported maxima are not the arm's, all {all:.3e}, in the ball {inside:.3e}: {line}"
        );
    }
}

#[test]
fn qa_randomise_report() {
    check_report(false);
}

negative_control!(
    qa_randomise_report,
    "the two populations swapped must not match the report",
    expected = "the reported maxima are not the arm's",
    check_report(true)
);
