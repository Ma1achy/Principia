//! QA tests for TASK-M2-50 (REQ-VAL-022, REQ-VAL-121, REQ-VAL-193), written from the requirements, R-551 and RQ-624's
//! decision (applied per R-369), not from the gate's code: the shape families, the gauge elements and the readings of
//! `w` and `z_β` are this file's own, on `kernel::encode` and `kernel::shape` directly.
//!
//! - REQ-VAL-022, the bit-for-bit check, on the decoder without its mirror step and with `β`'s one periodic link
//!   (TASK-M2-57): `β` has no clamp, so no collinear and no equilateral state sets `lookup_clamped`'s `β` bit; every
//!   collinear state whose `C(x)` has `λ̃_y` exactly `±0` has its `z_β` bit for bit at the link's inverse of `β = 0` or
//!   `β = π̂`, both reached; a state moved off the line leaves those images. *Was:* every collinear state's `z_β` at
//!   its clamp image (the bounded link's inverse of an input inside the clamp's band, `ε_z = 10⁻⁶`) with its bit set,
//!   a state moved past `π·ε_z` leaving the clamp.
//! - REQ-VAL-022, the pole and the equator, read on the canonical state `C(x)` (RQ-624): `|w − w_in|`, `w_in` the
//!   input's own orientation (+1 for L⁺, −1 for L⁻: `C` keeps the sign), and `|w|` within REQ-VAL-121's proposed
//!   bounds; a collinear state moved about 10³ ulps off the line is caught by `|w|`; the proposed bounds sit on the
//!   measurement; the collinear states whose `λ̃_y` is not `±0` are within the gate's collinear tolerance on `|λ̃_y|`.
//!   *Was:* `|w − 1|` for L⁺ and L⁻ alike, `C`'s mirror taking L⁻ to `w = +1`; the lifted state's `z_β` kept clamped.
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
    InspectorHandlesGate, Leak, RandomiseArm, RandomiseSeed, COLLINEAR_TOL, EQUATOR_BOUND, HANDLES,
    MOVES, POLE_BOUND,
};
use validation::negative_control;

/// REQ-VAL-021's bound, which the Randomise arm never loosens (R-551).
const HANDLE_BOUND: f64 = 1.6e-14;
/// The retired bounded β links' clamp, `ε_z`: the periodic link has none, which the image test's control reads.
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

/// A gauge element that keeps a state on the `x`-axis there exactly: no turn, an offset along `x` only; scale, parity
/// and boost as [`gauge`]'s. `C` of a collinear state through it has `λ̃_y` exactly `±0`.
fn axis_gauge(rng: &mut Rng, x: &Ic<f64>) -> Ic<f64> {
    let s = rng.on(-4.0, 4.0).exp2();
    let mirror = rng.next() & 1 == 1;
    let a = [rng.on(-2.0, 2.0), 0.0];
    let u = [rng.on(-2.0, 2.0), rng.on(-2.0, 2.0)];
    act(x, a, u, 0.0, s, mirror)
}

/// `π̂`, π's nearest f64 (dd_decoder §3.2's image of π at f64).
const PI_HAT: f64 = PI;

/// `z_β`'s two collinear images: the β link's inverse of `β = 0` (`λ̃` along `+x`) and of `β = π̂` (along `−x`).
fn beta_images() -> [f64; 2] {
    type BetaLink = <Defaults as Selection>::BetaLink;
    [0.0, PI_HAT].map(|b| BetaLink::inverse([b])[0])
}

/// What this file reads off RQ-475's two families.
struct Measured {
    /// The equilateral states' `|w − w_in|` on `C(x)`, `w_in` the input's own orientation.
    pole: Vec<f64>,
    /// The equilateral states' `|w − 1|` on `C(x)`: L⁻ read against `+1` (the control's reading).
    pole_plus: Vec<f64>,
    /// How many equilateral states the gauge mirrored; how many of those read `w = −1` before `C` (L⁻); and how many
    /// of those still read `w = −1` on `C(x)`.
    mirrored: [usize; 3],
    /// The collinear states' `|w|` on `C(x)`.
    equator: Vec<f64>,
    /// The collinear states' `|w|` on `D(E(x))`.
    equator_back: Vec<f64>,
    /// `[equilateral, collinear, collinear on the axis]`: how many set `lookup_clamped`'s β bit.
    bit: [usize; 3],
    /// `[equilateral, collinear, collinear on the axis]`: how many have `z_β` bit for bit at one of its images.
    image: [usize; 3],
    /// `[equilateral, collinear, collinear on the axis]`: how many have `λ̃_y` exactly `±0` on `C(x)`, and how many of
    /// those have `z_β` at an image.
    exact: [[usize; 2]; 3],
    /// The axis family's `z_β` at the image of `β = 0`, and of `β = π̂`.
    sides: [usize; 2],
    /// The collinear states' `|λ̃_y|` on `C(x)` where it is not `±0`.
    line: Vec<f64>,
}

/// RQ-475's families from `seed`, `n` of each: the equal-mass unit equilateral triangle (L⁺; the gauge's parity gives
/// L⁻), and three bodies on the `x`-axis at `[−1, 1)` with masses from `z_μ` on `[−4, 4)²`, body 2 raised by `lift`
/// off the line; each at rest, then through a random gauge element; and the collinear state again through a gauge
/// element that keeps it on the axis ([`axis_gauge`]). `nudge` moves each `z_β` one ulp before it is compared.
fn measure(seed: u64, n: usize, lift: f64, nudge: bool) -> Measured {
    let mut rng = Rng(seed);
    // The axis family draws from its own stream, so the two random-gauge families are the draws they were.
    let mut axis_rng = Rng(seed ^ 0xA715_0000_0000);
    let images = beta_images();
    let equal = mass::decode::<Defaults, f64>([0.0, 0.0]).m;
    let mut out = Measured {
        pole: Vec::new(),
        pole_plus: Vec::new(),
        mirrored: [0; 3],
        equator: Vec::new(),
        equator_back: Vec::new(),
        bit: [0; 3],
        image: [0; 3],
        exact: [[0; 2]; 3],
        sides: [0; 2],
        line: Vec::new(),
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
        for (k, x0) in [tri, line, line].iter().enumerate() {
            let (x, mirrored) = if k == 2 {
                (axis_gauge(&mut axis_rng, x0), false)
            } else {
                gauge(&mut rng, x0)
            };
            let e = encode::<Defaults, f64>(&x);
            let c = e.canonical.ic;
            let w = shape(c.r, c.m)[2];
            if k == 0 {
                let raw = shape(x.r, x.m)[2];
                out.pole.push((w - raw.signum()).abs());
                out.pole_plus.push((w - 1.0).abs());
                if mirrored {
                    out.mirrored[0] += 1;
                    let lower = (raw + 1.0).abs() < 1e-12;
                    out.mirrored[1] += usize::from(lower);
                    out.mirrored[2] += usize::from(lower && (w + 1.0).abs() < 1e-12);
                }
            } else if k == 1 {
                out.equator.push(w.abs());
                let back = decode::decode::<Defaults, f64>(&Latent::new(e.z.z)).ic;
                out.equator_back.push(shape(back.r, back.m)[2].abs());
            }
            let mut z_beta = e.z.z[1];
            if nudge {
                z_beta = f64::from_bits(z_beta.to_bits() + 1);
            }
            let at = images.iter().position(|i| i.to_bits() == z_beta.to_bits());
            out.bit[k] += usize::from(e.report.clamped_controls & 0b10 != 0);
            out.image[k] += usize::from(at.is_some());
            let lambda_y = e.canonical.lambda_t_y;
            if lambda_y == 0.0 {
                out.exact[k][0] += 1;
                out.exact[k][1] += usize::from(at.is_some());
            } else if k == 1 {
                out.line.push(lambda_y.abs());
            }
            if let (2, Some(side)) = (k, at) {
                out.sides[side] += 1;
            }
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

/// The images are the periodic link's inverse of the two collinear angles, `β − π/2` at `β = 0` and `β = π̂` (`image`
/// is that map), to a few ulps; and the link clamps nothing: an angle inside the retired links' band, `π·ε_z/4` from
/// either end, has its own `z_β`, `π·ε_z/4` from the image, not the image's (the retired clamp gave the whole band one
/// value).
fn check_beta_images(image: fn(f64) -> f64) {
    type BetaLink = <Defaults as Selection>::BetaLink;
    let got = beta_images();
    for (g, b) in got.iter().zip([0.0, PI_HAT]) {
        let w = image(b);
        assert!(
            (g - w).abs() <= 1e-15 * w.abs().max(1.0),
            "an image {g:e} is not the link's inverse at β = {b}, {w:e}"
        );
    }
    let band = PI * EPS_Z / 4.0;
    for (inside, edge) in [(band, got[0]), (PI_HAT - band, got[1])] {
        let z = BetaLink::inverse([inside])[0];
        let apart = (z - edge).abs();
        assert!(
            (apart - band).abs() <= 1e-9 * band,
            "β = {inside:e}, inside the retired band, is {apart:e} from the image, not π·ε_z/4"
        );
    }
}

#[test]
fn qa_beta_images_are_the_links() {
    check_beta_images(|b| b - PI / 2.0);
}

negative_control!(
    qa_beta_images_are_the_links,
    "the retired sigmoid link's clamp image, logit at ε_z and 1 − ε_z, must not match",
    expected = "is not the link's inverse at β",
    check_beta_images(|b| {
        let s = (b / PI).clamp(EPS_Z, 1.0 - EPS_Z);
        (s / (1.0 - s)).ln()
    })
);

/// REQ-VAL-022 on the decoder without `β`'s clamp: no state of either family sets `lookup_clamped`'s `β` bit; every
/// collinear state whose `C(x)` has `λ̃_y` exactly `±0` has its `z_β` at an image, bit for bit, the axis family's
/// every state among them, with both images reached; no equilateral state's `z_β` is at an image.
fn check_bits(nudge: bool) {
    for seed in SEEDS {
        let m = measure(seed, N_SHAPES, 0.0, nudge);
        eprintln!(
            "seed {seed:#x}: λ̃_y exactly ±0 on C(x) in {} of {N_SHAPES} collinear states in a random gauge, {} of \
             {N_SHAPES} on the axis (at β = 0: {}, at π̂: {})",
            m.exact[1][0], m.exact[2][0], m.sides[0], m.sides[1]
        );
        assert_eq!(m.bit, [0; 3], "a state sets lookup_clamped's β bit");
        assert_eq!(
            m.exact[2][0], N_SHAPES,
            "an axis state's λ̃_y is not exactly ±0, so it is not judged bit for bit"
        );
        for k in [1, 2] {
            assert_eq!(
                m.exact[k][1], m.exact[k][0],
                "a collinear z_β not at its image, bit for bit"
            );
        }
        assert!(
            m.sides[0] > 0 && m.sides[1] > 0,
            "the axis family does not reach both images: {:?}",
            m.sides
        );
        assert_eq!(m.image[0], 0, "an equilateral z_β at a collinear image");
    }
}

#[test]
fn qa_beta_bit_for_bit() {
    check_bits(false);
}

negative_control!(
    qa_beta_bit_for_bit,
    "z_β one ulp off its image must fail the bit-for-bit check",
    expected = "a collinear z_β not at its image",
    check_bits(true)
);

/// The bit check tells collinear from not: body 2 raised `lift` off the line leaves every axis state off the images,
/// its `λ̃_y` not `±0` (RQ-624 (2), the optional control). *Was:* a lift putting `β` past `π·ε_z` left the clamp.
fn check_leaves_images(lift: f64) {
    let m = measure(SEEDS[0], N_SHAPES, lift, false);
    assert!(
        m.exact[2][0] == 0 && m.image[2] == 0 && m.image[1] == 0,
        "a state {lift:e} off the line is still at an image: λ̃_y = ±0 in {}, image in {} of {N_SHAPES}",
        m.exact[2][0],
        m.image[2]
    );
}

#[test]
fn qa_beta_off_line_leaves_images() {
    check_leaves_images(1e-2);
}

negative_control!(
    qa_beta_off_line_leaves_images,
    "a state on the line must still be at its image",
    expected = "off the line is still at an image",
    check_leaves_images(0.0)
);

// ── The pole and the equator, on C(x) ───────────────────────────────────────────────────────────────────────────

/// REQ-VAL-022: the equal-mass equilateral family's `max |w − w_in|`, read on `C(x)`, `w_in` the input's own
/// orientation, within the proposed pole bound; both L⁺ and L⁻ drawn, the mirrored ones at `w = −1` before `C` and on
/// `C(x)`, which keeps the sign. `own` false is the control's reading, every input against `+1`.
fn check_pole(own: bool) {
    for seed in SEEDS {
        let m = measure(seed, N_SHAPES, 0.0, false);
        assert!(
            m.mirrored[0] > N_SHAPES * 2 / 5
                && m.mirrored[0] < N_SHAPES * 3 / 5
                && m.mirrored[1] == m.mirrored[0],
            "L⁺ and L⁻ are not both drawn: {:?} of {N_SHAPES}",
            m.mirrored
        );
        assert_eq!(
            m.mirrored[2], m.mirrored[1],
            "C does not keep an L⁻ input at w = −1"
        );
        let w = max(if own { &m.pole } else { &m.pole_plus });
        assert!(
            w <= POLE_BOUND,
            "the pole's |w − w_in| reaches {w:e}, past {POLE_BOUND:e}"
        );
    }
}

#[test]
fn qa_pole_on_canonical_state() {
    check_pole(true);
}

negative_control!(
    qa_pole_on_canonical_state,
    "L⁻ read against w = +1, where the retired mirror step put it, must fail",
    expected = "the pole's |w − w_in| reaches",
    check_pole(false)
);

/// REQ-VAL-022 (RQ-624 (1a)): the collinear family's `max |w|`, read on `C(x)`, within the proposed equator bound,
/// the states `ulps` ulps of 1 off the line (0 in the test). `D(E(x))`'s `|w|` is reported, not judged. *Was:*
/// `D(E(x))`'s `|w|` held to the retired clamp's image, `sin(π·ε_z)`, and read against the bound as the control; with
/// no clamp on `β` it is rounding, like `C(x)`'s, and controls nothing.
fn check_equator(ulps: u32) {
    for seed in SEEDS {
        let m = measure(seed, N_SHAPES, f64::from(ulps) * f64::EPSILON, false);
        eprintln!(
            "seed {seed:#x}: |w| on C(x) max {:.3e}; on D(E(x)), reported: max {:.3e}",
            max(&m.equator),
            max(&m.equator_back)
        );
        let w = max(&m.equator);
        assert!(
            w <= EQUATOR_BOUND,
            "the equator's |w| reaches {w:e}, past {EQUATOR_BOUND:e}"
        );
    }
}

#[test]
fn qa_equator_on_canonical_state() {
    check_equator(0);
}

negative_control!(
    qa_equator_on_canonical_state,
    "collinear states 1000 ulps of 1 off the line must fail the equator's bound",
    expected = "the equator's |w| reaches",
    check_equator(1000)
);

/// RQ-624 (2): the collinear family moved `ulps` ulps of 1 off the line (body 2's `y`, before the gauge) sets no `β`
/// bit, `β` having no clamp, while `|w|` on `C(x)` catches it, several times the bound. *Was:* the lifted states kept
/// `z_β` at its clamp image with its bit set.
fn check_in_band(ulps: u32) {
    let m = measure(SEEDS[1], N_SHAPES, f64::from(ulps) * f64::EPSILON, false);
    assert_eq!(m.bit, [0; 3], "a lifted state sets lookup_clamped's β bit");
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
/// most 4× its worst, so the bound can fire; a bound one ulp below the measurement fails. The pole's measure is
/// `|w − w_in|` (was `|w − 1|`).
fn check_bounds(pole_bound: f64, equator_bound: f64) {
    let (mut pole, mut equator) = (0.0f64, 0.0f64);
    for seed in SEEDS {
        let m = measure(seed, N_SHAPES, 0.0, false);
        pole = pole.max(max(&m.pole));
        equator = equator.max(max(&m.equator));
    }
    eprintln!("this file's worst: |w − w_in| {pole:e}, |w| {equator:e}");
    assert!(
        pole > 0.0 && pole <= pole_bound && equator <= equator_bound,
        "a shape bound below its measurement: |w − w_in| {pole:e} against {pole_bound:e}, |w| {equator:e} against \
         {equator_bound:e}"
    );
    assert!(
        pole_bound <= 4.0 * pole && equator_bound <= 4.0 * equator,
        "a shape bound not from its measurement: |w − w_in| {pole:e} against {pole_bound:e}, |w| {equator:e} against \
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

/// REQ-VAL-022, the collinear tolerance: every collinear state in a random gauge whose `C(x)` has `λ̃_y` not `±0`,
/// the members not judged bit for bit, has `|λ̃_y|` within `tol` (the gate's, proposed from its own measurement).
fn check_collinear_tolerance(tol: fn(f64) -> f64) {
    let mut worst = 0.0f64;
    let mut members = 0;
    for seed in SEEDS {
        let m = measure(seed, N_SHAPES, 0.0, false);
        worst = worst.max(max(&m.line));
        members += m.line.len();
    }
    eprintln!(
        "this file's worst |λ̃_y| on C(x) over {members} collinear states not at ±0: {worst:e}"
    );
    assert!(members > 0, "no collinear state is judged by the tolerance");
    let tol = tol(worst);
    assert!(
        worst <= tol,
        "a collinear |λ̃_y| on C(x) reaches {worst:e}, past the collinear tolerance {tol:e}"
    );
}

#[test]
fn qa_collinear_tolerance() {
    check_collinear_tolerance(|_| COLLINEAR_TOL);
}

negative_control!(
    qa_collinear_tolerance,
    "a tolerance one ulp below the measured worst must fail",
    expected = "past the collinear tolerance",
    check_collinear_tolerance(|worst| f64::from_bits(worst.to_bits() - 1))
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

/// `‖a − b‖` on the latent space, this file's own: Euclidean, with `z_β`'s difference (component 1) taken on the
/// circle, the nearest of the differences `2πk` apart, `z_β` being a periodic angle since TASK-M2-57. *Was:* the
/// Euclidean norm on ℝ⁸.
fn circle_norm(a: [f64; 8], b: [f64; 8]) -> f64 {
    let mut sum = 0.0;
    for k in 0..8 {
        let mut d = a[k] - b[k];
        if k == 1 {
            d = d.rem_euclid(2.0 * PI);
            d = d.min(2.0 * PI - d);
        }
        sum += d * d;
    }
    sum.sqrt()
}

/// `‖E(y) − E(x)‖`.
fn gap(x: &Ic<f64>, y: &Ic<f64>) -> f64 {
    circle_norm(
        encode::<Defaults, f64>(x).z.z,
        encode::<Defaults, f64>(y).z.z,
    )
}

/// The arm's measurement is REQ-VAL-193's: re-drawn here from the same stream, each of the six moves is the handle it
/// names (only its own component off the identity, the buttons at 15° and ×1.25), each `‖Δz‖` is `‖E(h·x) − E(x)‖`
/// read by this file's own gauge action (to `10⁻¹³`, the actions' roundings apart), the in-ball flag is
/// `‖E(x)‖ < 4` (the norm with `z_β` on the circle), and the reported worst, over all the draws and over the in-ball subset, is this file's maximum.
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
            let inside = circle_norm(z, [0.0; 8]) < 4.0;
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
