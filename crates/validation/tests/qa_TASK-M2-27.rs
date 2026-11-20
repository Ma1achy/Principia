//! QA tests for TASK-M2-27 (REQ-VAL-021, REQ-VAL-022), written from the requirements and
//! `ic_inspector_scratchpad.md` § "Status — as built" and § "Whole-system handles — the invariance audit", not from the
//! gate's code: the random ICs, the handles and the CoM reading are this file's own, on `kernel::decode` and
//! `kernel::encode` directly.
//!
//! - REQ-VAL-021: each whole-system handle at the tool's button sizes (Rotate 15°, Scale ×1.25) and at random sizes,
//!   `max ‖E(h·x) − E(x)‖ ≤ 1.6×10⁻¹⁴` over 2×10⁴ ICs `g₀·D(z)`, `z` in `‖z‖ < 4` (the gate's distribution; RQ-476
//!   holds the reading of "|z| < 4"). On the reference tool's own kind of IC, the Inspector's Randomise (three bodies,
//!   `m` on `[0.4, 1.8]` normalised, `r` on `±0.9`, `v` on `±0.6`, `p = m·v`), kept where `‖E(x)‖ < 4`, the worst
//!   `‖Δz‖` is measured and printed, held (RQ-477: what "random ICs" means), not judged.
//! - REQ-VAL-022 on 2×10⁴ random `z` in `‖z‖ < 4`: `‖z − E(D(z))‖ ≤ 1.2×10⁻¹³`, and the CoM of `C(D(z))` within
//!   1.7×10⁻¹⁶ of the origin, read by an exact-product, compensated sum.
//! - The `inspector-roundtrip` control's shift is added (`C`'s output moved by `+shift` on both axes), as its fixture
//!   and doc say: the read CoM is the `+shift` state's, not the `−shift` state's.
//!
//! Each has its negative control (R-176): a move that is not a gauge element, a nudged `z`, a translated canonical
//! state and the opposite shift each fail.

use kernel::canon::canonicalise;
use kernel::decode::{self, Ic, Latent};
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use validation::eps_phys::EncodeKind;
use validation::inspector::round_trip;
use validation::negative_control;

/// REQ-VAL-021's bound.
const HANDLE_BOUND: f64 = 1.6e-14;
/// REQ-VAL-022's round-trip bound.
const TRIP_BOUND: f64 = 1.2e-13;
/// REQ-VAL-022's canonical-CoM bound.
const COM_BOUND: f64 = 1.7e-16;
/// REQ-VAL-021's and REQ-VAL-022's sample counts.
const N: usize = 20_000;

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

fn z_of(x: &Ic<f64>) -> [f64; 8] {
    encode::<Defaults, f64>(x).z.z
}

fn gap(a: [f64; 8], b: [f64; 8]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// The Inspector's Randomise: masses on `[0.4, 1.8]`, normalised to `Σmᵢ = 1` exactly (the last as `1 − m₀ − m₁`),
/// positions on `±0.9`, velocities on `±0.6`, `p = m·v`. The CoM and the total momentum are left as drawn.
fn randomise(rng: &mut Rng) -> Ic<f64> {
    let raw: [f64; 3] = std::array::from_fn(|_| rng.on(0.4, 1.8));
    let total: f64 = raw.iter().sum();
    let (m0, m1) = (raw[0] / total, raw[1] / total);
    let m = [m0, m1, 1.0 - m0 - m1];
    let r: [[f64; 2]; 3] = std::array::from_fn(|_| [rng.on(-0.9, 0.9), rng.on(-0.9, 0.9)]);
    let v: [[f64; 2]; 3] = std::array::from_fn(|_| [rng.on(-0.6, 0.6), rng.on(-0.6, 0.6)]);
    let p = std::array::from_fn(|i| [m[i] * v[i][0], m[i] * v[i][1]]);
    Ic { m, r, p }
}

fn translate(x: &Ic<f64>, a: [f64; 2]) -> Ic<f64> {
    Ic {
        r: x.r.map(|v| [v[0] + a[0], v[1] + a[1]]),
        ..*x
    }
}

fn boost(x: &Ic<f64>, u: [f64; 2]) -> Ic<f64> {
    Ic {
        p: std::array::from_fn(|i| [x.p[i][0] + x.m[i] * u[0], x.p[i][1] + x.m[i] * u[1]]),
        ..*x
    }
}

fn rotate(x: &Ic<f64>, theta: f64) -> Ic<f64> {
    let (s, c) = theta.sin_cos();
    let turn = |v: [f64; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    Ic {
        r: x.r.map(turn),
        p: x.p.map(turn),
        ..*x
    }
}

/// The similarity `r → k·r`, `p → k^{−1/2}·p`, which keeps the shape and `p/√I`'s ratios.
fn scale(x: &Ic<f64>, k: f64) -> Ic<f64> {
    Ic {
        r: x.r.map(|v| v.map(|w| w * k)),
        p: x.p.map(|v| v.map(|w| w / k.sqrt())),
        ..*x
    }
}

/// Where REQ-VAL-021's random ICs come from. What "2×10⁴ random ICs" means is RQ-477's, and the reading of "|z| < 4"
/// RQ-476's: the gate's own distribution is asserted meanwhile, the Inspector's Randomise measured and reported only.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Draw {
    /// The gate's: `g₀·D(z)`, `z` uniform in `‖z‖ < 4`, `g₀` a random gauge element (scale `s₀` on `2^[−4, 4)`, any
    /// turn, either parity, offset `s₀·[−1, 1)²`, boost `s₀^{−1/2}·[−1, 1)²`), built here, not by `validation::gauge`.
    Gauge,
    /// The Inspector's Randomise, kept where `‖E(x)‖ < 4`; about 5 % of the draws fall outside and are drawn again.
    Randomise,
}

/// `(x_k, −y_k)` on every position and momentum: the parity half of the gauge.
fn mirror(x: &Ic<f64>) -> Ic<f64> {
    Ic {
        r: x.r.map(|v| [v[0], -v[1]]),
        p: x.p.map(|v| [v[0], -v[1]]),
        ..*x
    }
}

/// One IC from `draw`, and the scale its handles' offsets and boosts are drawn in proportion to: `(s₀, s₀^{−1/2})`
/// for [`Draw::Gauge`], the canvas's `(0.9, 0.6)` for [`Draw::Randomise`].
fn draw_ic(rng: &mut Rng, draw: Draw) -> (Ic<f64>, [f64; 2]) {
    match draw {
        Draw::Gauge => {
            let x = decode_z(ball(rng));
            let s0 = rng.on(-4.0, 4.0).exp2();
            let theta = rng.on(-std::f64::consts::PI, std::f64::consts::PI);
            let flip = rng.next() & 1 == 1;
            let a = [rng.on(-1.0, 1.0) * s0, rng.on(-1.0, 1.0) * s0];
            let u = [rng.on(-1.0, 1.0) / s0.sqrt(), rng.on(-1.0, 1.0) / s0.sqrt()];
            let x = if flip { mirror(&x) } else { x };
            let x = boost(&translate(&scale(&rotate(&x, theta), s0), a), u);
            (x, [s0, 1.0 / s0.sqrt()])
        }
        Draw::Randomise => loop {
            let x = randomise(rng);
            if z_of(&x).iter().map(|v| v * v).sum::<f64>() < 16.0 {
                return (x, [0.9, 0.6]);
            }
        },
    }
}

/// Every one of the four handles, each at the tool's button size where it has one (Rotate 15°, Scale ×1.25) and at a
/// random size: translate by an offset of the IC's own size, boost by a velocity of its own size, rotate by any turn,
/// scale by `2^[−2, 2)`. `move_one` replaces translate-all with a move of body 0 alone (not a gauge element), the
/// control. Returns the worst `‖Δz‖` per handle over `n` ICs from `draw`.
fn handles_worst(n: usize, draw: Draw, move_one: bool) -> [f64; 4] {
    let mut rng = Rng(0x5157_0021);
    let mut worst = [0.0f64; 4];
    for _ in 0..n {
        let (x, size) = draw_ic(&mut rng, draw);
        let z0 = z_of(&x);
        let a = [rng.on(-size[0], size[0]), rng.on(-size[0], size[0])];
        let moved = if move_one {
            let mut y = x;
            y.r[0] = [y.r[0][0] + a[0], y.r[0][1] + a[1]];
            y
        } else {
            translate(&x, a)
        };
        let u = [rng.on(-size[1], size[1]), rng.on(-size[1], size[1])];
        let theta = rng.on(-std::f64::consts::PI, std::f64::consts::PI);
        let k = rng.on(-2.0, 2.0).exp2();
        let images: [Vec<Ic<f64>>; 4] = [
            vec![moved],
            vec![boost(&x, u)],
            vec![rotate(&x, 15f64.to_radians()), rotate(&x, theta)],
            vec![scale(&x, 1.25), scale(&x, k)],
        ];
        for (h, list) in images.iter().enumerate() {
            for y in list {
                let d = gap(z0, z_of(y));
                assert!(!d.is_nan(), "a NaN ‖Δz‖ under handle {h}");
                worst[h] = worst[h].max(d);
            }
        }
    }
    worst
}

const HANDLE_NAMES: [&str; 4] = ["translate-all", "boost-all", "rotate-all", "scale-all"];

fn check_handles(n: usize, move_one: bool) {
    let worst = handles_worst(n, Draw::Gauge, move_one);
    for (w, name) in worst.iter().zip(HANDLE_NAMES) {
        assert!(
            *w <= HANDLE_BOUND,
            "REQ-VAL-021: {name} moves z by {w:e}, past {HANDLE_BOUND:e}"
        );
    }
}

/// REQ-VAL-021: 2×10⁴ random ICs, each handle applied, `max ‖Δz‖ ≤ 1.6×10⁻¹⁴`, asserted on the gate's own
/// distribution (RQ-476 holds its reading of "|z| < 4" meanwhile). On the Inspector's Randomise the worst `‖Δz‖` per
/// handle is measured and printed, held (RQ-477), not judged: what "random ICs" means is RQ-477's.
#[test]
fn qa_handles_hold_on_randomised_ics() {
    check_handles(N, false);
    let held = handles_worst(N, Draw::Randomise, false);
    for (w, name) in held.iter().zip(HANDLE_NAMES) {
        eprintln!(
            "held (RQ-477), not judged: Randomise, {name}: max ‖Δz‖ {w:.3e} over {N} ICs (REQ-VAL-021's bound \
             {HANDLE_BOUND:e})"
        );
    }
}

negative_control!(
    qa_handles_hold_on_randomised_ics,
    "moving one body, not every body, must move z past the bound",
    expected = "REQ-VAL-021: translate-all moves z",
    check_handles(200, true)
);

/// `a·b` as an exact pair `(hi, lo)`.
fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    (p, a.mul_add(b, -p))
}

/// `Σ` of `terms` with every rounding error carried (Neumaier), the terms themselves exact.
fn exact_sum(terms: &[f64]) -> f64 {
    let mut s = 0.0f64;
    let mut c = 0.0f64;
    for &t in terms {
        let u = s + t;
        c += if s.abs() >= t.abs() {
            (s - u) + t
        } else {
            (t - u) + s
        };
        s = u;
    }
    s + c
}

/// `(Σmᵢrᵢ_x, Σmᵢrᵢ_y)` read without the reading's own rounding.
fn com_vec(x: &Ic<f64>) -> [f64; 2] {
    [0, 1].map(|k| {
        let mut terms = Vec::with_capacity(6);
        for i in 0..3 {
            let (h, l) = two_prod(x.m[i], x.r[i][k]);
            terms.push(h);
            terms.push(l);
        }
        exact_sum(&terms)
    })
}

/// A `z` uniform in the ball `‖z‖ < 4`.
fn ball(rng: &mut Rng) -> [f64; 8] {
    loop {
        let z: [f64; 8] = std::array::from_fn(|_| rng.on(-4.0, 4.0));
        if z.iter().map(|v| v * v).sum::<f64>() < 16.0 {
            return z;
        }
    }
}

fn decode_z(z: [f64; 8]) -> Ic<f64> {
    decode::decode::<Defaults, f64>(&Latent::new(z)).ic
}

/// REQ-VAL-022 over `n` random `z`: the worst `‖z − E(D(z + nudge·e₀))‖` and the worst canonical CoM, the canonical
/// state moved by `offset` on both axes before its CoM is read (0 in the test, the control's offset otherwise).
fn trips_worst(n: usize, nudge: f64, offset: f64) -> (f64, f64) {
    let mut rng = Rng(0x5157_0022);
    let (mut trip, mut com) = (0.0f64, 0.0f64);
    for _ in 0..n {
        let z = ball(&mut rng);
        let mut zn = z;
        zn[0] += nudge;
        let x = decode_z(zn);
        let d = gap(z, z_of(&x));
        let c = translate(&canonicalise(&x).ic, [offset, offset]);
        let r = com_vec(&c);
        let norm = r[0].hypot(r[1]);
        assert!(!d.is_nan() && !norm.is_nan(), "a NaN at {z:?}");
        trip = trip.max(d);
        com = com.max(norm);
    }
    (trip, com)
}

fn check_trips(n: usize, nudge: f64, offset: f64) {
    let (trip, com) = trips_worst(n, nudge, offset);
    assert!(
        trip <= TRIP_BOUND,
        "REQ-VAL-022: a round trip of {trip:e}, past {TRIP_BOUND:e}"
    );
    assert!(
        com <= COM_BOUND,
        "REQ-VAL-022: a canonical CoM {com:e} from the origin, past {COM_BOUND:e}"
    );
}

/// REQ-VAL-022: 2×10⁴ random `z`, `‖z‖ < 4`: `max ‖z − E(D(z))‖ ≤ 1.2×10⁻¹³`, canonical CoM ≤ 1.7×10⁻¹⁶.
#[test]
fn qa_round_trip_and_canonical_com() {
    check_trips(N, 0.0, 0.0);
}

negative_control!(
    qa_round_trip_and_canonical_com,
    "z nudged by 10⁻¹² before the decode must fail the round trip",
    expected = "REQ-VAL-022: a round trip of",
    check_trips(200, 1e-12, 0.0)
);

negative_control!(
    qa_round_trip_com_control,
    "the canonical state moved by 2⁻⁵⁰ must fail the CoM bound",
    expected = "REQ-VAL-022: a canonical CoM",
    check_trips(200, 0.0, (-50f64).exp2())
);

/// The `inspector-roundtrip` control's CoM, `‖Σmᵢ(rᵢ + shift)‖` of `C(D(z))` with `shift` added to every coordinate:
/// the gate's reading of it equals this file's reading of the `+shift` state (each coordinate's own rounding
/// included), relative `10⁻¹²`, on every `z`, and on most `z` is told apart from the `−shift` state's by more than
/// that.
fn check_shift(sign: f64) {
    let shift = (-50f64).exp2();
    let mut rng = Rng(0x5157_5f17);
    let mut apart = 0;
    for _ in 0..200 {
        let z = ball(&mut rng);
        let c = canonicalise(&decode_z(z)).ic;
        let read = |s: f64| {
            let y = Ic {
                r: c.r.map(|v| v.map(|w| w + s)),
                ..c
            };
            let r = com_vec(&y);
            r[0].hypot(r[1])
        };
        let (want, other) = (read(sign * shift), read(-sign * shift));
        let got = round_trip(z, EncodeKind::Defaults, shift).1;
        assert!(
            (got - want).abs() <= 1e-12 * want,
            "the shifted CoM reads {got:e}, not the +shift state's {want:e}"
        );
        if (want - other).abs() > 1e-12 * want {
            apart += 1;
        }
    }
    assert!(
        apart >= 100,
        "the two signs are told apart on {apart} of 200"
    );
}

#[test]
fn qa_shift_is_added() {
    check_shift(1.0);
}

negative_control!(
    qa_shift_is_added,
    "the opposite shift's CoM must not match",
    expected = "not the +shift state's",
    check_shift(-1.0)
);
