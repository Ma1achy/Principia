//! QA tests for TASK-M2-01 on the kernel side (`kernel::generated::links`, `kernel::generated::constants`), written from
//! the requirements, not from the implementation:
//! - REQ-CHART-034 (dd_generation_root §5 test 8; chart_decoder_contract § "Three hard requirements on any registered
//!   link"; R-368): per link, (a) constraint preservation for all inputs including saturation, (b) the inverse
//!   round trip within the ε clamp's tolerance in physical units, (c) the analytic log-det against an independent
//!   Jacobian, the log of the volume factor (`log |f'|`, or `log √det(JᵀJ)` for the 3×2 simplex Jacobian), and
//!   (d) C¹.
//! - Each generated link's forward is the formula §3.9 and dd_decoder §3 give for it, evaluated here from the docs.
//! - REQ-GEN-013 / REQ-GEN-015: the baked defaults are mass = softmax ∘ μ_max·tanh, config = sigmoid, free momentum =
//!   sigmoid, and the selection is a type, so another selection is another variant.
//! - REQ-DEC-009: the generated chart constants are μ_max = 5, q_max = 2, α_min = 0, ε_μ = ε_z = ε_q = 10⁻⁶,
//!   δ_λ = 10⁻¹², ε_w = 10⁻¹⁰ at f32 and f64, and no decode/encode source, nor the generated link bodies, writes one
//!   as a literal.
//!
//! The Jacobian of (c) and the derivative of (d) are computed by forward-mode automatic differentiation: the
//! generated links are generic over `LinkReal`, so they are instantiated here at a dual number, which carries the
//! exact derivative of the generated forward alongside its value. That Jacobian is the "numeric Jacobian" of test
//! 8 (c) without a finite-difference step. Its tolerance is REQ-GEN-025's (the proposed value, pending the M2 gate).
//! (d) is a convergence test, not a tolerance: a jump in `f'` at a point leaves `|f'(x+h) − f'(x−h)|` unchanged as
//! `h` shrinks tenfold, a C¹ link shrinks it tenfold, and the check requires it to at least halve.
//!
//! Every check runs over a link behind function pointers, so each negative control (R-176) runs the same check on a
//! deliberately broken link.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::num::FpCategory;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::path::Path;

use kernel::generated::constants::ChartConstants;
#[cfg(feature = "controls")]
use kernel::generated::links::LinkReal;
use kernel::generated::links::{
    Codomain, Defaults, Exp, Identity, Link, Literals, Select, Selection, SigmoidAlpha,
    SigmoidBeta, SigmoidQ, SoftmaxTanh, Softplus, SoftsignAlpha, SoftsignBeta, SoftsignQ,
    StickBreaking, TanhAlpha, TanhBeta, TanhQ, NAMES,
};
use kernel::Real;
use proptest::prelude::*;
use spirv_std::num_traits::{Float, Num, One, ToPrimitive, Zero};
use validation::{negative_control, prop};

// ── The chart constants, from the requirement (REQ-DEC-009) ─────────────────────────────────────────────────────

const MU_MAX: f64 = 5.0;
const Q_MAX: f64 = 2.0;
const ALPHA_MIN: f64 = 0.0;
const EPS: f64 = 1e-6;
const DELTA_LAMBDA: f64 = 1e-12;
const EPS_W: f64 = 1e-10;

// ── A dual number: the value and its two partial derivatives ────────────────────────────────────────────────────

/// `v + d₀·ε₀ + d₁·ε₁`, the value of a function and its partials with respect to the (at most two) controls.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct D {
    v: f64,
    d: [f64; 2],
}

const fn k(v: f64) -> D {
    D { v, d: [0.0, 0.0] }
}

impl D {
    /// `self` as the outer function `g` applied: value `g(v)`, derivative `g'(v)·d`.
    fn chain(self, value: f64, slope: f64) -> D {
        D {
            v: value,
            d: [slope * self.d[0], slope * self.d[1]],
        }
    }
}

impl PartialOrd for D {
    fn partial_cmp(&self, other: &D) -> Option<Ordering> {
        self.v.partial_cmp(&other.v)
    }
}

impl Add for D {
    type Output = D;
    fn add(self, o: D) -> D {
        D {
            v: self.v + o.v,
            d: [self.d[0] + o.d[0], self.d[1] + o.d[1]],
        }
    }
}

impl Sub for D {
    type Output = D;
    fn sub(self, o: D) -> D {
        D {
            v: self.v - o.v,
            d: [self.d[0] - o.d[0], self.d[1] - o.d[1]],
        }
    }
}

impl Mul for D {
    type Output = D;
    fn mul(self, o: D) -> D {
        D {
            v: self.v * o.v,
            d: [
                self.d[0] * o.v + self.v * o.d[0],
                self.d[1] * o.v + self.v * o.d[1],
            ],
        }
    }
}

impl Div for D {
    type Output = D;
    fn div(self, o: D) -> D {
        let q = self.v / o.v;
        D {
            v: q,
            d: [
                (self.d[0] - q * o.d[0]) / o.v,
                (self.d[1] - q * o.d[1]) / o.v,
            ],
        }
    }
}

impl Rem for D {
    type Output = D;
    fn rem(self, _: D) -> D {
        unused("rem")
    }
}

impl Neg for D {
    type Output = D;
    fn neg(self) -> D {
        D {
            v: -self.v,
            d: [-self.d[0], -self.d[1]],
        }
    }
}

/// An operation the links are not expected to use: reaching one fails loudly rather than differentiating wrongly.
fn unused(what: &str) -> ! {
    panic!("the dual number does not implement `{what}`; a generated link now uses it")
}

impl Zero for D {
    fn zero() -> D {
        k(0.0)
    }
    fn is_zero(&self) -> bool {
        self.v == 0.0
    }
}

impl One for D {
    fn one() -> D {
        k(1.0)
    }
}

impl Num for D {
    type FromStrRadixErr = ();
    fn from_str_radix(_: &str, _: u32) -> Result<D, ()> {
        Err(())
    }
}

impl ToPrimitive for D {
    fn to_i64(&self) -> Option<i64> {
        self.v.to_i64()
    }
    fn to_u64(&self) -> Option<u64> {
        self.v.to_u64()
    }
    fn to_f64(&self) -> Option<f64> {
        Some(self.v)
    }
}

impl spirv_std::num_traits::NumCast for D {
    fn from<T: ToPrimitive>(n: T) -> Option<D> {
        n.to_f64().map(k)
    }
}

impl Float for D {
    fn nan() -> D {
        k(f64::NAN)
    }
    fn infinity() -> D {
        k(f64::INFINITY)
    }
    fn neg_infinity() -> D {
        k(f64::NEG_INFINITY)
    }
    fn neg_zero() -> D {
        k(-0.0)
    }
    fn min_value() -> D {
        k(f64::MIN)
    }
    fn min_positive_value() -> D {
        k(f64::MIN_POSITIVE)
    }
    fn max_value() -> D {
        k(f64::MAX)
    }
    fn is_nan(self) -> bool {
        self.v.is_nan()
    }
    fn is_infinite(self) -> bool {
        self.v.is_infinite()
    }
    fn is_finite(self) -> bool {
        self.v.is_finite()
    }
    fn is_normal(self) -> bool {
        self.v.is_normal()
    }
    fn classify(self) -> FpCategory {
        self.v.classify()
    }
    fn floor(self) -> D {
        unused("floor")
    }
    fn ceil(self) -> D {
        unused("ceil")
    }
    fn round(self) -> D {
        unused("round")
    }
    fn trunc(self) -> D {
        unused("trunc")
    }
    fn fract(self) -> D {
        unused("fract")
    }
    fn abs(self) -> D {
        if self.v < 0.0 {
            -self
        } else {
            self
        }
    }
    fn signum(self) -> D {
        k(self.v.signum())
    }
    fn is_sign_positive(self) -> bool {
        self.v.is_sign_positive()
    }
    fn is_sign_negative(self) -> bool {
        self.v.is_sign_negative()
    }
    fn mul_add(self, a: D, b: D) -> D {
        self * a + b
    }
    fn recip(self) -> D {
        k(1.0) / self
    }
    fn powi(self, n: i32) -> D {
        let v = self.v.powi(n);
        self.chain(v, f64::from(n) * self.v.powi(n - 1))
    }
    fn powf(self, _: D) -> D {
        unused("powf")
    }
    fn sqrt(self) -> D {
        let v = self.v.sqrt();
        self.chain(v, 0.5 / v)
    }
    fn exp(self) -> D {
        let v = self.v.exp();
        self.chain(v, v)
    }
    fn exp2(self) -> D {
        unused("exp2")
    }
    fn ln(self) -> D {
        self.chain(self.v.ln(), 1.0 / self.v)
    }
    fn log(self, _: D) -> D {
        unused("log")
    }
    fn log2(self) -> D {
        unused("log2")
    }
    fn log10(self) -> D {
        unused("log10")
    }
    /// f64's `max`: the other operand when one is NaN.
    fn max(self, o: D) -> D {
        if self.v.is_nan() || o.v > self.v {
            o
        } else {
            self
        }
    }
    /// f64's `min`: the other operand when one is NaN.
    fn min(self, o: D) -> D {
        if self.v.is_nan() || o.v < self.v {
            o
        } else {
            self
        }
    }
    fn abs_sub(self, _: D) -> D {
        unused("abs_sub")
    }
    fn cbrt(self) -> D {
        unused("cbrt")
    }
    fn hypot(self, _: D) -> D {
        unused("hypot")
    }
    fn sin(self) -> D {
        unused("sin")
    }
    fn cos(self) -> D {
        unused("cos")
    }
    fn tan(self) -> D {
        unused("tan")
    }
    fn asin(self) -> D {
        unused("asin")
    }
    fn acos(self) -> D {
        unused("acos")
    }
    fn atan(self) -> D {
        unused("atan")
    }
    fn atan2(self, _: D) -> D {
        unused("atan2")
    }
    fn sin_cos(self) -> (D, D) {
        unused("sin_cos")
    }
    fn exp_m1(self) -> D {
        self.chain(self.v.exp_m1(), self.v.exp())
    }
    fn ln_1p(self) -> D {
        self.chain(self.v.ln_1p(), 1.0 / (1.0 + self.v))
    }
    fn sinh(self) -> D {
        self.chain(self.v.sinh(), self.v.cosh())
    }
    fn cosh(self) -> D {
        self.chain(self.v.cosh(), self.v.sinh())
    }
    fn tanh(self) -> D {
        // sech² x, not 1 − tanh² x, which cancels where tanh saturates.
        let c = self.v.cosh();
        self.chain(self.v.tanh(), 1.0 / (c * c))
    }
    fn asinh(self) -> D {
        unused("asinh")
    }
    fn acosh(self) -> D {
        unused("acosh")
    }
    fn atanh(self) -> D {
        self.chain(self.v.atanh(), 1.0 / (1.0 - self.v * self.v))
    }
    fn integer_decode(self) -> (u64, i16, i8) {
        unused("integer_decode")
    }
}

impl Real for D {
    const NAME: &'static str = "dual";
}

/// The chart constants at the dual number: f64's, with zero derivative.
impl ChartConstants for D {
    const MU_MAX: D = k(<f64 as ChartConstants>::MU_MAX);
    const Q_MAX: D = k(<f64 as ChartConstants>::Q_MAX);
    const ALPHA_MIN: D = k(<f64 as ChartConstants>::ALPHA_MIN);
    const EPS_MU: D = k(<f64 as ChartConstants>::EPS_MU);
    const EPS_Z: D = k(<f64 as ChartConstants>::EPS_Z);
    const EPS_Q: D = k(<f64 as ChartConstants>::EPS_Q);
    const DELTA_LAMBDA: D = k(<f64 as ChartConstants>::DELTA_LAMBDA);
    const EPS_W: D = k(<f64 as ChartConstants>::EPS_W);
}

/// The registry's literals at the dual number: f64's, with zero derivative.
impl Literals for D {
    const LIT_0: D = k(<f64 as Literals>::LIT_0);
    const LIT_1: D = k(<f64 as Literals>::LIT_1);
    const LIT_2: D = k(<f64 as Literals>::LIT_2);
    const LIT_3: D = k(<f64 as Literals>::LIT_3);
    const LIT_4: D = k(<f64 as Literals>::LIT_4);
    const LIT_5: D = k(<f64 as Literals>::LIT_5);
    const LIT_6: D = k(<f64 as Literals>::LIT_6);
    const LIT_7: D = k(<f64 as Literals>::LIT_7);
    const LIT_8: D = k(<f64 as Literals>::LIT_8);
}

// ── The links under test, behind function pointers ──────────────────────────────────────────────────────────────

/// A link at f64, f32 and the dual number.
#[derive(Clone, Copy)]
struct Under {
    name: &'static str,
    codomain: &'static str,
    controls: usize,
    f64_forward: fn(&[f64]) -> Vec<f64>,
    f64_inverse: fn(&[f64]) -> Vec<f64>,
    f64_log_det: fn(&[f64]) -> f64,
    f32_forward: fn(&[f32]) -> Vec<f32>,
    f32_inverse: fn(&[f32]) -> Vec<f32>,
    f32_log_det: fn(&[f32]) -> f32,
    dual_forward: fn(&[D]) -> Vec<D>,
}

fn one<L: Link<1, 1>>() -> Under {
    Under {
        name: L::NAME,
        codomain: <L::Codomain as Codomain>::NAME,
        controls: 1,
        f64_forward: |x| L::forward([x[0]]).to_vec(),
        f64_inverse: |y| L::inverse([y[0]]).to_vec(),
        f64_log_det: |x| L::log_det([x[0]]),
        f32_forward: |x| L::forward([x[0]]).to_vec(),
        f32_inverse: |y| L::inverse([y[0]]).to_vec(),
        f32_log_det: |x| L::log_det([x[0]]),
        dual_forward: |x| L::forward([x[0]]).to_vec(),
    }
}

fn two<L: Link<2, 3>>() -> Under {
    Under {
        name: L::NAME,
        codomain: <L::Codomain as Codomain>::NAME,
        controls: 2,
        f64_forward: |x| L::forward([x[0], x[1]]).to_vec(),
        f64_inverse: |y| L::inverse([y[0], y[1], y[2]]).to_vec(),
        f64_log_det: |x| L::log_det([x[0], x[1]]),
        f32_forward: |x| L::forward([x[0], x[1]]).to_vec(),
        f32_inverse: |y| L::inverse([y[0], y[1], y[2]]).to_vec(),
        f32_log_det: |x| L::log_det([x[0], x[1]]),
        dual_forward: |x| L::forward([x[0], x[1]]).to_vec(),
    }
}

/// Every generated link type.
fn generated() -> Vec<Under> {
    vec![
        two::<SoftmaxTanh>(),
        two::<StickBreaking>(),
        one::<SigmoidAlpha>(),
        one::<SigmoidBeta>(),
        one::<TanhAlpha>(),
        one::<TanhBeta>(),
        one::<SoftsignAlpha>(),
        one::<SoftsignBeta>(),
        one::<SigmoidQ>(),
        one::<TanhQ>(),
        one::<SoftsignQ>(),
        one::<Softplus>(),
        one::<Exp>(),
        one::<Identity>(),
    ]
}

// ── The specification, from the docs ────────────────────────────────────────────────────────────────────────────

/// A codomain: the simplex, a closed interval `[lo, hi]` (open in exact arithmetic, reached at float saturation),
/// the half-line or the line.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Simplex,
    Interval(f64, f64),
    Half,
    Line,
}

/// What the docs say of one link.
#[derive(Clone, Copy)]
struct Spec {
    name: &'static str,
    codomain: &'static str,
    kind: Kind,
    /// How far, in physical units, the inverse's ε clamp can move a value it clamps (inverse_encode Part 3; §3.9).
    clamp: f64,
    /// The control range the inverse reaches from the closed codomain through its ε clamp: (c) and (d) run over it.
    reach: f64,
    /// The forward, as the docs write it.
    forward: fn(&[f64]) -> Vec<f64>,
}

fn sigma(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// softmax(0, μ₁, μ₂), μₖ = μ_max·tanh zₖ (dd_decoder §3.1; §3.9's simplex row).
fn softmax_tanh_doc(z: &[f64]) -> Vec<f64> {
    let (e1, e2) = ((MU_MAX * z[0].tanh()).exp(), (MU_MAX * z[1].tanh()).exp());
    let t = 1.0 + e1 + e2;
    vec![1.0 / t, e1 / t, e2 / t]
}

/// Stick-breaking (§3.9's R-72 definition): s, u = ½(1 + (1 − ε_μ) tanh zₖ), m = (1 − s, s(1 − u), su).
fn stick_doc(z: &[f64]) -> Vec<f64> {
    let s = 0.5 * (1.0 + (1.0 - EPS) * z[0].tanh());
    let u = 0.5 * (1.0 + (1.0 - EPS) * z[1].tanh());
    vec![1.0 - s, s * (1.0 - u), s * u]
}

/// The softsign `u = x/(2 + |x|)` of §3.9's heavier-tailed bounded link.
fn softsign(x: f64) -> f64 {
    x / (2.0 + x.abs())
}

const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
const PI: f64 = std::f64::consts::PI;

/// artanh(1 − ε): the control the tanh-type inverses reach at their clamp.
fn reach_tanh() -> f64 {
    (1.0 - EPS).atanh()
}

/// logit(1 − ε): the control σ's inverse reaches at its clamp.
fn reach_sigmoid() -> f64 {
    ((1.0 - EPS) / EPS).ln()
}

/// 2u/(1 − |u|) at u = 1 − ε: the control softsign's inverse reaches at its clamp.
fn reach_softsign() -> f64 {
    2.0 * (1.0 - EPS) / EPS
}

/// The f64 control range over which `exp` and σ stay normal, `|x| ≤ 700`: the inverse `log` of every normal positive
/// value lies in it, to within the normal range's ends.
const REACH_LINE: f64 = 700.0;

/// The docs' fourteen entries: §3.9's six rows, registered once per block codomain, and the two R-72 definitions.
fn specs() -> Vec<Spec> {
    let alpha = Kind::Interval(ALPHA_MIN, HALF_PI - ALPHA_MIN);
    let beta = Kind::Interval(0.0, PI);
    let q = Kind::Interval(-Q_MAX, Q_MAX);
    let width = |k: Kind| match k {
        Kind::Interval(a, b) => b - a,
        _ => unreachable!(),
    };
    vec![
        Spec {
            name: "softmax_tanh",
            codomain: "mass",
            kind: Kind::Simplex,
            // The clamp moves each logit by at most ε_μ·μ_max, and a mass by at most a quarter of that per logit.
            clamp: EPS * MU_MAX,
            reach: reach_tanh(),
            forward: softmax_tanh_doc,
        },
        Spec {
            name: "stick_breaking",
            codomain: "mass",
            kind: Kind::Simplex,
            // The clamp moves 2v − 1 from at most 1 (a face of the closed simplex) to (1 − ε_μ)², so each of s and u
            // by at most ε_μ, and a mass, at most s·u, by at most the sum of their moves.
            clamp: 2.0 * EPS,
            reach: reach_tanh(),
            forward: stick_doc,
        },
        Spec {
            name: "sigmoid_alpha",
            codomain: "alpha",
            kind: alpha,
            clamp: EPS * width(alpha),
            reach: reach_sigmoid(),
            forward: |x| vec![ALPHA_MIN + (HALF_PI - 2.0 * ALPHA_MIN) * sigma(x[0])],
        },
        Spec {
            name: "sigmoid_beta",
            codomain: "beta",
            kind: beta,
            clamp: EPS * width(beta),
            reach: reach_sigmoid(),
            forward: |x| vec![PI * sigma(x[0])],
        },
        Spec {
            name: "tanh_alpha",
            codomain: "alpha",
            kind: alpha,
            clamp: EPS * width(alpha) / 2.0,
            reach: reach_tanh(),
            forward: |x| vec![ALPHA_MIN + (HALF_PI / 2.0 - ALPHA_MIN) * (1.0 + x[0].tanh())],
        },
        Spec {
            name: "tanh_beta",
            codomain: "beta",
            kind: beta,
            clamp: EPS * width(beta) / 2.0,
            reach: reach_tanh(),
            forward: |x| vec![HALF_PI * (1.0 + x[0].tanh())],
        },
        Spec {
            name: "softsign_alpha",
            codomain: "alpha",
            kind: alpha,
            clamp: EPS * width(alpha) / 2.0,
            reach: reach_softsign(),
            forward: |x| vec![ALPHA_MIN + (HALF_PI / 2.0 - ALPHA_MIN) * (1.0 + softsign(x[0]))],
        },
        Spec {
            name: "softsign_beta",
            codomain: "beta",
            kind: beta,
            clamp: EPS * width(beta) / 2.0,
            reach: reach_softsign(),
            forward: |x| vec![HALF_PI * (1.0 + softsign(x[0]))],
        },
        Spec {
            name: "sigmoid_q",
            codomain: "momentum",
            kind: q,
            clamp: EPS * width(q),
            reach: reach_sigmoid(),
            // dd_decoder §3.4: qₖ = q_max·(2σ(z) − 1).
            forward: |x| vec![Q_MAX * (2.0 * sigma(x[0]) - 1.0)],
        },
        Spec {
            name: "tanh_q",
            codomain: "momentum",
            kind: q,
            clamp: EPS * width(q) / 2.0,
            reach: reach_tanh(),
            forward: |x| vec![Q_MAX * x[0].tanh()],
        },
        Spec {
            name: "softsign_q",
            codomain: "momentum",
            kind: q,
            clamp: EPS * width(q) / 2.0,
            reach: reach_softsign(),
            forward: |x| vec![Q_MAX * softsign(x[0])],
        },
        Spec {
            name: "softplus",
            codomain: "half_line",
            kind: Kind::Half,
            clamp: 0.0,
            reach: REACH_LINE,
            // log(1 + eˣ), as x + log(1 + e⁻ˣ) where eˣ would overflow.
            forward: |x| {
                let v = x[0];
                vec![if v > 30.0 {
                    v + (-v).exp().ln_1p()
                } else {
                    v.exp().ln_1p()
                }]
            },
        },
        Spec {
            name: "exp",
            codomain: "half_line",
            kind: Kind::Half,
            clamp: 0.0,
            reach: REACH_LINE,
            forward: |x| vec![x[0].exp()],
        },
        Spec {
            name: "identity",
            codomain: "real_line",
            kind: Kind::Line,
            clamp: 0.0,
            reach: 1e300,
            forward: |x| vec![x[0]],
        },
    ]
}

fn spec(name: &str) -> Spec {
    specs()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no entry `{name}` in §3.9"))
}

/// The scale of a link's absolute round-off: 1 on the simplex, the width on an interval, none on a line (whose
/// round-off is relative to its value).
fn scale(kind: Kind) -> f64 {
    match kind {
        Kind::Simplex => 1.0,
        Kind::Interval(a, b) => b - a,
        Kind::Half | Kind::Line => 0.0,
    }
}

// ── The inputs ──────────────────────────────────────────────────────────────────────────────────────────────────

/// Fixed controls across a float of largest finite value `max`: the centre, small, the transition into
/// saturation, the clamps' reaches, deep saturation and the float's end, each of both signs.
fn fixed(max: f64) -> Vec<f64> {
    let mags = [
        0.0,
        1e-300,
        1e-30,
        1e-8,
        0.25,
        1.0,
        3.0,
        reach_tanh(),
        10.0,
        reach_sigmoid(),
        20.0,
        40.0,
        88.0,
        100.0,
        700.0,
        709.0,
        1e4,
        reach_softsign(),
        1e10,
        1e30,
        1e300,
        max,
    ];
    mags.iter()
        .filter(|m| **m <= max)
        .flat_map(|&m| [m, -m])
        .collect()
}

/// The controls of a link with `controls` controls: each fixed control, or each pair of them.
fn points(controls: usize, max: f64) -> Vec<Vec<f64>> {
    let xs = fixed(max);
    if controls == 1 {
        xs.iter().map(|&x| vec![x]).collect()
    } else {
        xs.iter()
            .flat_map(|&a| xs.iter().map(move |&b| vec![a, b]))
            .collect()
    }
}

/// A fuzzed control: log-uniform in magnitude from 10⁻⁸ to the float's end, or uniform across the centre.
fn control() -> impl Strategy<Value = (bool, f64, f64, bool)> {
    (any::<bool>(), -8.0f64..308.0, -50.0f64..50.0, any::<bool>())
}

fn draw((neg, e, u, centre): (bool, f64, f64, bool), max: f64) -> f64 {
    if centre {
        u
    } else {
        let m = 10f64.powf(e).min(max);
        if neg {
            -m
        } else {
            m
        }
    }
}

/// `check` at every fixed point and fuzzed point of `u`'s controls, at f64's range and f32's.
fn over_inputs(u: &Under, check: impl Fn(&Under, &[f64], bool)) {
    for wide in [true, false] {
        let max = if wide { f64::MAX } else { f64::from(f32::MAX) };
        for x in points(u.controls, max) {
            check(u, &x, wide);
        }
    }
    let draws = proptest::collection::vec(control(), u.controls);
    prop::run(&draws, |d| {
        for wide in [true, false] {
            let max = if wide { f64::MAX } else { f64::from(f32::MAX) };
            let x: Vec<f64> = d.iter().map(|&c| draw(c, max)).collect();
            check(u, &x, wide);
        }
        Ok(())
    });
}

/// The largest finite input at which a half-line link's exact output is finite at the float: ln(MAX).
fn half_line_input_ok(spec: &Spec, x: &[f64], wide: bool) -> bool {
    let ln_max = if wide {
        f64::MAX.ln()
    } else {
        f64::from(f32::MAX).ln()
    };
    spec.kind != Kind::Half || x[0] <= ln_max
}

fn f32s(x: &[f64]) -> Vec<f32> {
    x.iter().map(|&v| v as f32).collect()
}

fn f64s(x: &[f32]) -> Vec<f64> {
    x.iter().map(|&v| f64::from(v)).collect()
}

// ── Coverage ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Every generated entry is one of the docs' entries, onto the docs' codomain, and every docs entry is generated.
fn check_coverage(under: &[Under], names: &[&str]) {
    let tested: BTreeSet<&str> = under.iter().map(|u| u.name).collect();
    let listed: BTreeSet<&str> = names.iter().copied().collect();
    let documented: BTreeSet<&str> = specs().iter().map(|s| s.name).collect();
    assert_eq!(
        tested, listed,
        "the links under test are not the generated NAMES (coverage)"
    );
    assert_eq!(
        listed, documented,
        "the generated entries are not §3.9's entries (coverage)"
    );
    for u in under {
        assert_eq!(
            u.codomain,
            spec(u.name).codomain,
            "{} maps onto `{}`, not §3.9's codomain (coverage)",
            u.name,
            u.codomain
        );
    }
}

#[test]
fn qa_link_properties_cover_every_entry() {
    check_coverage(&generated(), &NAMES);
}

negative_control!(
    qa_link_properties_cover_every_entry,
    "a generated entry left out of the links under test must fail coverage",
    expected = "(coverage)",
    check_coverage(&generated()[1..], &NAMES)
);

// ── The forward is the docs' formula ────────────────────────────────────────────────────────────────────────────

/// `u`'s f64 forward against the docs' formula, at `x`, to the round-off of a few operations on its scale: 16 ulps
/// of the larger of the value and its codomain's scale.
fn check_forward(u: &Under, x: &[f64]) {
    let s = spec(u.name);
    let got = (u.f64_forward)(x);
    let want = (s.forward)(x);
    for (g, w) in got.iter().zip(&want) {
        let tol = 16.0 * f64::EPSILON * (scale(s.kind) + w.abs()) + f64::MIN_POSITIVE;
        assert!(
            (g - w).abs() <= tol || (g == w),
            "{} at {x:?}: forward {got:?}, the docs' formula {want:?} (forward formula)",
            u.name
        );
    }
}

#[test]
fn qa_link_forward_is_the_docs_formula() {
    for u in generated() {
        over_inputs(&u, |u, x, wide| {
            if wide && half_line_input_ok(&spec(u.name), x, wide) {
                check_forward(u, x);
            }
        });
    }
}

negative_control!(
    qa_link_forward_is_the_docs_formula,
    "tanh_q checked against sigmoid_q's formula must fail: c·tanh x is not c·(2σ(x) − 1)",
    expected = "(forward formula)",
    check_forward(
        &Under {
            name: "sigmoid_q",
            ..one::<TanhQ>()
        },
        &[1.0]
    )
);

// ── (a) Constraint preservation ─────────────────────────────────────────────────────────────────────────────────

/// `y = f(x)` satisfies `u`'s constraint at a float of epsilon `eps` and largest finite value `max`: on the simplex,
/// finite, strictly positive, summing to 1 within the sum's round-off (each of three quotients within 3 ulps, and
/// two additions: 16 ulps); on an interval, within the closed bounds as the float rounds them; on the half-line,
/// finite and not negative; on the line, finite.
fn check_constraint(u: &Under, x: &[f64], y: &[f64], eps: f64, wide: bool) {
    let s = spec(u.name);
    let fail = |why: &str| -> ! { panic!("{} at {x:?}: {y:?} {why} (constraint)", u.name) };
    if y.iter().any(|v| !v.is_finite()) {
        fail("is not finite");
    }
    match s.kind {
        Kind::Simplex => {
            if y.iter().any(|&m| m <= 0.0) {
                fail("has a mass not strictly positive");
            }
            if (y.iter().sum::<f64>() - 1.0).abs() > 16.0 * eps {
                fail("does not sum to 1");
            }
        }
        Kind::Interval(a, b) => {
            let (lo, hi) = if wide {
                (a, b)
            } else {
                (f64::from(a as f32), f64::from(b as f32))
            };
            if y[0] < lo || y[0] > hi {
                fail(&format!("leaves [{lo}, {hi}]"));
            }
        }
        Kind::Half => {
            if y[0] < 0.0 {
                fail("is negative");
            }
        }
        Kind::Line => {}
    }
}

fn check_a(u: &Under, x: &[f64], wide: bool) {
    if !half_line_input_ok(&spec(u.name), x, wide) {
        return;
    }
    if wide {
        check_constraint(u, x, &(u.f64_forward)(x), f64::EPSILON, wide);
    } else {
        let y = f64s(&(u.f32_forward)(&f32s(x)));
        check_constraint(u, x, &y, f64::from(f32::EPSILON), wide);
    }
}

#[test]
fn qa_link_properties_a_constraint() {
    for u in generated() {
        over_inputs(&u, check_a);
    }
}

/// `v` at the float `R`.
#[cfg(feature = "controls")]
fn lit<R: LinkReal>(v: f64) -> R {
    <R as spirv_std::num_traits::NumCast>::from(v).expect("a float converts")
}

/// A link onto β that escapes it: π·(1.5σ(x) − 0.25), which leaves (0, π) at both ends.
#[cfg(feature = "controls")]
struct EscapingBeta;

#[cfg(feature = "controls")]
impl Link<1, 1> for EscapingBeta {
    const NAME: &'static str = "sigmoid_beta";
    type Codomain = <SigmoidBeta as Link<1, 1>>::Codomain;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        let one = R::one();
        let s = one / (one + (-v[0]).exp());
        let three_halves = lit::<R>(1.5);
        let quarter = lit::<R>(0.25);
        [lit::<R>(PI) * (three_halves * s - quarter)]
    }
    fn inverse<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidBeta::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 1]) -> R {
        SigmoidBeta::log_det(v)
    }
}

/// A simplex link whose masses do not sum to 1: stick-breaking with its last mass `s` in place of `s·u`.
#[cfg(feature = "controls")]
struct LeakySimplex;

#[cfg(feature = "controls")]
impl Link<2, 3> for LeakySimplex {
    const NAME: &'static str = "stick_breaking";
    type Codomain = <StickBreaking as Link<2, 3>>::Codomain;
    fn forward<R: LinkReal>(v: [R; 2]) -> [R; 3] {
        let m = StickBreaking::forward(v);
        [m[0], m[1], m[1] + m[2]]
    }
    fn inverse<R: LinkReal>(v: [R; 3]) -> [R; 2] {
        StickBreaking::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 2]) -> R {
        StickBreaking::log_det(v)
    }
}

negative_control!(
    qa_link_properties_a_constraint,
    "a bounded link that leaves its interval at saturation must fail (a)",
    expected = "(constraint)",
    over_inputs(&one::<EscapingBeta>(), check_a)
);

negative_control!(
    qa_link_properties_a_constraint_simplex,
    "a simplex link whose masses do not sum to 1 must fail (a)",
    expected = "does not sum to 1",
    over_inputs(&two::<LeakySimplex>(), check_a)
);

#[test]
fn qa_link_properties_a_constraint_simplex() {
    for u in generated().iter().filter(|u| u.controls == 2) {
        over_inputs(u, check_a);
    }
}

// ── (b) The inverse round trip, in physical units ───────────────────────────────────────────────────────────────

/// `y` against `again`, both in physical units, within the ε clamp's move and the round-off: 16 ulps of the scale
/// and of `|y|` (forward and inverse each a few operations) and, on the half-line, where `exp` and `softplus` turn one
/// ulp of the recovered control into `|x|` ulps of the value, 16·(1 + |x|) ulps of `|y|`; the float's smallest normal
/// is a floor for a subnormal `y`.
fn check_round_trip(u: &Under, x: &[f64], y: &[f64], again: &[f64], eps: f64, tiny: f64) {
    let s = spec(u.name);
    let span = if s.kind == Kind::Half {
        1.0 + x[0].abs()
    } else {
        1.0
    };
    for (a, b) in y.iter().zip(again) {
        let tol = s.clamp + 16.0 * eps * (scale(s.kind) + span * a.abs()) + tiny;
        assert!(
            b.is_finite() && (a - b).abs() <= tol,
            "{} at {x:?}: f(x) = {y:?}, f(f⁻¹(f(x))) = {again:?}, off by more than {tol} (round trip)",
            u.name
        );
    }
}

fn check_b(u: &Under, x: &[f64], wide: bool) {
    if !half_line_input_ok(&spec(u.name), x, wide) {
        return;
    }
    if wide {
        let y = (u.f64_forward)(x);
        let again = (u.f64_forward)(&(u.f64_inverse)(&y));
        check_round_trip(u, x, &y, &again, f64::EPSILON, f64::MIN_POSITIVE);
    } else {
        let y = (u.f32_forward)(&f32s(x));
        let again = (u.f32_forward)(&(u.f32_inverse)(&y));
        let (eps, tiny) = (f64::from(f32::EPSILON), f64::from(f32::MIN_POSITIVE));
        check_round_trip(u, x, &f64s(&y), &f64s(&again), eps, tiny);
    }
}

#[test]
fn qa_link_properties_b_round_trip() {
    for u in generated() {
        over_inputs(&u, check_b);
    }
}

/// σ onto β with a clamp a thousand times ε_z: its round trip moves a saturated β by π·10⁻³, past the ε clamp's
/// tolerance.
#[cfg(feature = "controls")]
struct LooseClampBeta;

#[cfg(feature = "controls")]
impl Link<1, 1> for LooseClampBeta {
    const NAME: &'static str = "sigmoid_beta";
    type Codomain = <SigmoidBeta as Link<1, 1>>::Codomain;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidBeta::forward(v)
    }
    fn inverse<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        let one = R::one();
        let eps = lit::<R>(1e-3);
        let s = (v[0] / lit::<R>(PI)).max(eps).min(one - eps);
        [(s / (one - s)).ln()]
    }
    fn log_det<R: LinkReal>(v: [R; 1]) -> R {
        SigmoidBeta::log_det(v)
    }
}

negative_control!(
    qa_link_properties_b_round_trip,
    "an inverse clamped at 10⁻³ instead of ε_z must fail (b)",
    expected = "(round trip)",
    over_inputs(&one::<LooseClampBeta>(), check_b)
);

/// From the physical side: on an interval, `f(f⁻¹(y))` is `y` clamped into the interval, within the ε clamp's move,
/// for `y` across the closed interval and beyond it (inverse_encode Part 3: `|q_k| > q_max` → clamp); the inverse is
/// finite there (chart_decoder_contract: "a conditioned inverse").
fn check_interval_encode(u: &Under) {
    let s = spec(u.name);
    let Kind::Interval(a, b) = s.kind else {
        return;
    };
    let w = b - a;
    let mut ys = vec![
        a - w,
        a - 1e-9,
        a,
        a + 1e-12,
        a + EPS * w,
        b - EPS * w,
        b - 1e-12,
    ];
    ys.extend([b, b + 1e-9, b + w]);
    ys.extend((0..=200).map(|i| a + w * f64::from(i) / 200.0));
    for y in ys {
        let z = (u.f64_inverse)(&[y]);
        assert!(
            z[0].is_finite(),
            "{} at y = {y}: f⁻¹(y) = {z:?} (encode)",
            u.name
        );
        let again = (u.f64_forward)(&z)[0];
        let want = y.clamp(a, b);
        let tol = s.clamp + 16.0 * f64::EPSILON * w;
        assert!(
            (again - want).abs() <= tol,
            "{} at y = {y}: f(f⁻¹(y)) = {again}, want {want} within {tol} (encode)",
            u.name
        );
    }
}

#[test]
fn qa_link_properties_b_encode_interval() {
    for u in generated() {
        check_interval_encode(&u);
    }
}

negative_control!(
    qa_link_properties_b_encode_interval,
    "an inverse clamped at 10⁻³ instead of ε_z must fail the encode round trip",
    expected = "(encode)",
    check_interval_encode(&one::<LooseClampBeta>())
);

/// Points of the closed simplex: its corners, its edges' midpoints and near-corner points, and a barycentric grid.
fn simplex_points() -> Vec<[f64; 3]> {
    let mut ms = vec![
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.5, 0.5, 0.0],
        [0.5, 0.0, 0.5],
        [0.0, 0.5, 0.5],
        [1.0 - 2e-300, 1e-300, 1e-300],
        [1e-300, 1.0 - 2e-300, 1e-300],
        [1e-300, 1e-300, 1.0 - 2e-300],
    ];
    let n = 40;
    for i in 0..=n {
        for j in 0..=n - i {
            let (a, b) = (f64::from(i) / f64::from(n), f64::from(j) / f64::from(n));
            ms.push([a, b, 1.0 - a - b]);
        }
    }
    ms
}

/// On the simplex: the inverse is finite at every point of the closed simplex, at f64 and f32, and the decoded masses
/// satisfy the constraint; where `exact` (a link that reaches within ε_μ/2 of every edge and corner, §3.9),
/// `f(f⁻¹(m))` is `m` within the clamp's move.
fn check_simplex_encode(u: &Under, exact: bool) {
    let s = spec(u.name);
    for m in simplex_points() {
        let z = (u.f64_inverse)(&m);
        assert!(
            z.iter().all(|v| v.is_finite()),
            "{} at m = {m:?}: f⁻¹(m) = {z:?} (simplex encode)",
            u.name
        );
        let z32 = (u.f32_inverse)(&f32s(&m));
        assert!(
            z32.iter().all(|v| v.is_finite()),
            "{} at m = {m:?}: f32 f⁻¹(m) = {z32:?} (simplex encode)",
            u.name
        );
        let again = (u.f64_forward)(&z);
        check_constraint(u, &z, &again, f64::EPSILON, true);
        if exact {
            for (a, b) in m.iter().zip(&again) {
                let tol = s.clamp + 16.0 * f64::EPSILON;
                assert!(
                    (a - b).abs() <= tol,
                    "{} at m = {m:?}: f(f⁻¹(m)) = {again:?}, off by more than {tol} (simplex encode)",
                    u.name
                );
            }
        }
    }
}

#[test]
fn qa_link_properties_b_encode_simplex() {
    check_simplex_encode(&two::<SoftmaxTanh>(), false);
    check_simplex_encode(&two::<StickBreaking>(), true);
}

negative_control!(
    qa_link_properties_b_encode_simplex,
    "softmax_tanh, which stops at mass ratios e^(±2μ_max), held to the edge-reaching round trip must fail",
    expected = "(simplex encode)",
    check_simplex_encode(
        &Under {
            name: "stick_breaking",
            ..two::<SoftmaxTanh>()
        },
        true
    )
);

// ── (c) The analytic log-det against the Jacobian ───────────────────────────────────────────────────────────────

/// REQ-GEN-025: the tolerance of test 8 (c), the proposed value, unconfirmed until the M2 gate (R-71). The Jacobian
/// here is exact to round-off, so the discrepancy measured against it is the log-det's own.
const C_TOL: f64 = 1e-5;

/// The log of the forward's volume factor at `x`, from the dual Jacobian: `log |f'|` for one control, and for the
/// simplex's 3×2 `J`, `log √det(JᵀJ)`, the area element, which is `log ‖J₁ × J₂‖` (R-368).
fn log_volume(u: &Under, x: &[f64]) -> f64 {
    let seeded: Vec<D> = x
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let mut d = [0.0; 2];
            d[i] = 1.0;
            D { v, d }
        })
        .collect();
    let y = (u.dual_forward)(&seeded);
    if u.controls == 1 {
        y[0].d[0].abs().ln()
    } else {
        let a: Vec<f64> = y.iter().map(|m| m.d[0]).collect();
        let b: Vec<f64> = y.iter().map(|m| m.d[1]).collect();
        let cross = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        cross.iter().map(|c| c * c).sum::<f64>().sqrt().ln()
    }
}

/// The controls (c) and (d) run over for `u`: a grid across the clamp's reach, a finer grid across the centre, and
/// the reach's ends, per control.
fn c_axis(reach: f64) -> Vec<f64> {
    let mut xs: Vec<f64> = (-20..=20).map(|i| reach * f64::from(i) / 20.0).collect();
    xs.extend(
        (-40..=40)
            .map(|i| f64::from(i) / 4.0)
            .filter(|x| x.abs() <= reach),
    );
    xs
}

fn c_points(u: &Under) -> Vec<Vec<f64>> {
    let xs = c_axis(spec(u.name).reach);
    if u.controls == 1 {
        xs.iter().map(|&x| vec![x]).collect()
    } else {
        let coarse: Vec<f64> = xs.iter().copied().step_by(3).collect();
        coarse
            .iter()
            .flat_map(|&a| coarse.iter().map(move |&b| vec![a, b]))
            .collect()
    }
}

/// The largest |analytic − Jacobian| over `u`'s domain; fails above [`C_TOL`].
fn check_c(u: &Under) -> f64 {
    let mut worst = 0.0f64;
    for x in c_points(u) {
        let analytic = (u.f64_log_det)(&x);
        let jacobian = log_volume(u, &x);
        let gap = (analytic - jacobian).abs();
        assert!(
            analytic.is_finite() && gap <= C_TOL,
            "{} at {x:?}: log-det {analytic}, the Jacobian's log volume factor {jacobian} (log-det)",
            u.name
        );
        worst = worst.max(gap);
    }
    worst
}

#[test]
fn qa_link_properties_c_log_det() {
    for u in generated() {
        let worst = check_c(&u);
        println!(
            "{}: max |log-det − log volume| = {worst:e} over |x| ≤ {:e}",
            u.name,
            spec(u.name).reach
        );
    }
}

/// softmax_tanh with `log det(JᵀJ)` for its log-det: the volume factor's square, which R-368 rules out.
#[cfg(feature = "controls")]
struct NoRootSimplex;

#[cfg(feature = "controls")]
impl Link<2, 3> for NoRootSimplex {
    const NAME: &'static str = "softmax_tanh";
    type Codomain = <SoftmaxTanh as Link<2, 3>>::Codomain;
    fn forward<R: LinkReal>(v: [R; 2]) -> [R; 3] {
        SoftmaxTanh::forward(v)
    }
    fn inverse<R: LinkReal>(v: [R; 3]) -> [R; 2] {
        SoftmaxTanh::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 2]) -> R {
        let two = R::one() + R::one();
        two * SoftmaxTanh::log_det(v)
    }
}

negative_control!(
    qa_link_properties_c_log_det,
    "a simplex log-det of log det(JᵀJ), without R-368's square root, must fail (c)",
    expected = "(log-det)",
    {
        check_c(&two::<NoRootSimplex>());
    }
);

// ── The log-det's value across the whole domain, at f64 and f32 ─────────────────────────────────────────────────

/// `softplus t = log(1 + eᵗ)`, written so that no step overflows.
fn softplus(t: f64) -> f64 {
    t.max(0.0) + (-t.abs()).exp().ln_1p()
}

/// `log σ(x) = −softplus(−x)`.
fn log_sigma(x: f64) -> f64 {
    -softplus(-x)
}

/// `log sech² x`: `sech² x = 4/(eˣ + e⁻ˣ)² = 4e^(−2|x|)/(1 + e^(−2|x|))²`, so
/// `log sech² x = log 4 − 2|x| − 2·log(1 + e^(−2|x|))`; it is below the float's range only where the exact value is.
fn log_sech2(x: f64) -> f64 {
    4f64.ln() - 2.0 * x.abs() - 2.0 * (-2.0 * x.abs()).exp().ln_1p()
}

/// `(v, 1 − v)` for §3.9's stick fraction `v = ½(1 + (1 − ε_μ)·tanh z)`, each a ratio of sums of positive terms, so
/// that neither cancels against 1 (with `w = e^(−2|z|)`, `v = ((1 − ε/2) + (ε/2)·w)/(1 + w)` for `z ≥ 0`, and the two
/// numerators swapped for `z < 0`).
fn stick_half(z: f64) -> (f64, f64) {
    let w = (-2.0 * z.abs()).exp();
    let big = (1.0 - EPS / 2.0) + EPS / 2.0 * w;
    let small = EPS / 2.0 + (1.0 - EPS / 2.0) * w;
    let (v, rest) = if z >= 0.0 { (big, small) } else { (small, big) };
    (v / (1.0 + w), rest / (1.0 + w))
}

/// §3.9's log-det column for the entry `name` at `x`, evaluated at f64 in forms that neither overflow nor cancel
/// before the exact value does: σ's `log((b − a)·σ(x)·σ(−x))`, tanh's `log(c·sech² x)`, softsign's
/// `log(2c) − 2 log(2 + |x|)`, softplus's `log σ(x)`, exp's `x`, identity's 0, softmax_tanh's
/// `½ log 3 + Σᵢ log mᵢ + 2 log μ_max + log sech² z₁ + log sech² z₂` and stick-breaking's
/// `log(√3/4·(1 − ε_μ)²·s·sech² z₁·sech² z₂)` (R-368).
fn log_det_doc(name: &str, x: &[f64]) -> f64 {
    let s = spec(name);
    let width = match s.kind {
        Kind::Interval(a, b) => b - a,
        _ => f64::NAN,
    };
    match name {
        "softmax_tanh" => {
            let m = softmax_tanh_doc(x);
            0.5 * 3f64.ln()
                + m.iter().map(|v| v.ln()).sum::<f64>()
                + 2.0 * MU_MAX.ln()
                + log_sech2(x[0])
                + log_sech2(x[1])
        }
        "stick_breaking" => {
            (3f64.sqrt() / 4.0).ln()
                + 2.0 * (1.0 - EPS).ln()
                + stick_half(x[0]).0.ln()
                + log_sech2(x[0])
                + log_sech2(x[1])
        }
        "sigmoid_alpha" | "sigmoid_beta" | "sigmoid_q" => {
            width.ln() + log_sigma(x[0]) + log_sigma(-x[0])
        }
        "tanh_alpha" | "tanh_beta" | "tanh_q" => (width / 2.0).ln() + log_sech2(x[0]),
        "softsign_alpha" | "softsign_beta" | "softsign_q" => {
            width.ln() - 2.0 * (2.0 + x[0].abs()).ln()
        }
        "softplus" => log_sigma(x[0]),
        "exp" => x[0],
        "identity" => 0.0,
        other => panic!("`{other}` has no log-det in §3.9 (log-det value)"),
    }
}

/// The reference [`log_det_doc`] against the plain formulae where those do not overflow or cancel (`|x| ≤ 15`): a
/// check of the check.
fn check_reference() {
    for x in (-60..=60).map(|i| f64::from(i) / 4.0) {
        let plain_sech2 = (1.0 / (x.cosh() * x.cosh())).ln();
        let plain_sigma = (sigma(x) * sigma(-x)).ln();
        assert!(
            (log_sech2(x) - plain_sech2).abs() <= 1e-12 * (1.0 + plain_sech2.abs())
                && (log_sigma(x) + log_sigma(-x) - plain_sigma).abs()
                    <= 1e-12 * (1.0 + plain_sigma.abs()),
            "the reference log sech² or log σ at {x} is not the plain formula (log-det value)"
        );
        if x.abs() <= 3.0 {
            let (v, rest) = stick_half(x);
            let plain = 0.5 * (1.0 + (1.0 - EPS) * x.tanh());
            assert!(
                (v - plain).abs() <= 1e-15 && (rest - (1.0 - plain)).abs() <= 1e-15,
                "the reference stick fraction at {x} is not §3.9's (log-det value)"
            );
        }
    }
}

/// `u`'s log-det at `x` (f64 when `wide`, else at f32 on `x` rounded to f32) against §3.9's value there, evaluated at
/// f64 by [`log_det_doc`] (walls W7 and W9, measure honesty and totality, as §3.9's "overflow-free forms" paragraph
/// reads them). Where the exact value is well inside the float's range (`|value| ≤ MAX/4`), the log-det is finite
/// and within 64 ulps of the float on the size of the terms it sums, `1 + |value| + Σ|xₖ|`; where it is well beyond
/// (`≥ 4·MAX`), it is the infinity of its sign, the float's correct rounding; between, either; never NaN.
fn check_log_det_value(u: &Under, x: &[f64], wide: bool) {
    let (got, x, eps, max) = if wide {
        ((u.f64_log_det)(x), x.to_vec(), f64::EPSILON, f64::MAX)
    } else {
        let x32 = f32s(x);
        (
            f64::from((u.f32_log_det)(&x32)),
            f64s(&x32),
            f64::from(f32::EPSILON),
            f64::from(f32::MAX),
        )
    };
    let want = log_det_doc(u.name, &x);
    let size = (1.0 + want.abs() + x.iter().map(|v| v.abs()).sum::<f64>()).min(f64::MAX);
    let tol = 64.0 * eps * size;
    let close = got.is_finite() && (got - want).abs() <= tol;
    let overflowed = got.is_infinite() && got.signum() == want.signum();
    let ok = if want.abs() <= max / 4.0 {
        close
    } else if want.abs() >= 4.0 * max {
        overflowed
    } else {
        close || overflowed
    };
    assert!(
        ok,
        "{} at {x:?} ({}): log-det {got}, §3.9's {want}, allowed {tol} (log-det value)",
        u.name,
        if wide { "f64" } else { "f32" }
    );
}

#[test]
fn qa_link_log_det_value_whole_domain() {
    check_reference();
    for u in generated() {
        over_inputs(&u, check_log_det_value);
    }
}

/// tanh onto α whose log-det is floored at −80: finite at both floats, and the two agree, but it is not the volume
/// factor's log past `|x| ≈ 40`, where `log sech² x ≈ log 4 − 2|x|`.
#[cfg(feature = "controls")]
struct FlooredLogDet;

#[cfg(feature = "controls")]
impl Link<1, 1> for FlooredLogDet {
    const NAME: &'static str = "tanh_alpha";
    type Codomain = <TanhAlpha as Link<1, 1>>::Codomain;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        TanhAlpha::forward(v)
    }
    fn inverse<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        TanhAlpha::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 1]) -> R {
        TanhAlpha::log_det(v).max(lit::<R>(-80.0))
    }
}

negative_control!(
    qa_link_log_det_value_whole_domain,
    "a tanh log-det floored at −80, finite and equal at both floats, must fail past |x| ≈ 40",
    expected = "(log-det value)",
    over_inputs(&one::<FlooredLogDet>(), check_log_det_value)
);

// ── (d) C¹ ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// The partials of `u`'s forward along control `k` at `x`, one per physical component.
fn partials(u: &Under, x: &[f64], k: usize) -> Vec<f64> {
    let seeded: Vec<D> = x
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let mut d = [0.0; 2];
            d[i] = f64::from(u8::from(i == k));
            D { v, d }
        })
        .collect();
    (u.dual_forward)(&seeded)
        .iter()
        .map(|m| m.d[0] + m.d[1])
        .collect()
}

/// `max |∂f(x + h·e_k) − ∂f(x − h·e_k)|` and the largest partial seen.
fn spread(u: &Under, x: &[f64], k: usize, h: f64) -> (f64, f64) {
    let at = |s: f64| {
        let mut p = x.to_vec();
        p[k] += s * h;
        partials(u, &p, k)
    };
    let (l, r) = (at(-1.0), at(1.0));
    let gap = l
        .iter()
        .zip(&r)
        .fold(0.0f64, |m, (a, b)| m.max((a - b).abs()));
    let size = l.iter().chain(&r).fold(0.0f64, |m, a| m.max(a.abs()));
    (gap, size)
}

/// (d) at each point of `u`'s domain and along each control: the derivative's spread across the point at h shrinks
/// at least by half at h/10 (a C¹ link's shrinks tenfold, a jump's not at all), or is round-off (64 ulps of the
/// derivative).
fn check_d(u: &Under) {
    for x in c_points(u) {
        for k in 0..u.controls {
            let h = 1e-3 * x[k].abs().max(1.0);
            let (wide, size) = spread(u, &x, k, h);
            let (narrow, _) = spread(u, &x, k, h / 10.0);
            let floor = 64.0 * f64::EPSILON * size + f64::MIN_POSITIVE;
            assert!(
                narrow <= wide / 2.0 || narrow <= floor,
                "{} at {x:?} along control {k}: the derivative's jump across the point is {wide:e} at h = {h:e} \
                 and {narrow:e} at h/10 (C¹)",
                u.name
            );
        }
    }
}

#[test]
fn qa_link_properties_d_c1() {
    for u in generated() {
        check_d(&u);
    }
}

/// A link onto β with a kink at 0: π·σ(max(x, 0)), whose derivative jumps from 0 to π/4.
#[cfg(feature = "controls")]
struct KinkedBeta;

#[cfg(feature = "controls")]
impl Link<1, 1> for KinkedBeta {
    const NAME: &'static str = "sigmoid_beta";
    type Codomain = <SigmoidBeta as Link<1, 1>>::Codomain;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidBeta::forward([v[0].max(R::zero())])
    }
    fn inverse<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        SigmoidBeta::inverse(v)
    }
    fn log_det<R: LinkReal>(v: [R; 1]) -> R {
        SigmoidBeta::log_det(v)
    }
}

negative_control!(
    qa_link_properties_d_c1,
    "a link with a kink at 0 must fail (d)",
    expected = "(C¹)",
    check_d(&one::<KinkedBeta>())
);

// ── Selection (REQ-GEN-013, REQ-GEN-015) ────────────────────────────────────────────────────────────────────────

/// A selection's link ids are dd_decoder §3's defaults: mass = softmax ∘ μ_max·tanh, config = sigmoid, free
/// momentum = sigmoid.
fn check_defaults(ids: [&str; 7]) {
    let want = [
        "softmax_tanh",
        "sigmoid_alpha",
        "sigmoid_beta",
        "sigmoid_q",
        "sigmoid_q",
        "sigmoid_q",
        "sigmoid_q",
    ];
    assert_eq!(
        ids, want,
        "the selection is not dd_decoder §3's defaults (defaults)"
    );
}

/// The default selection's links decode as dd_decoder §3 writes them, through the selection's associated types.
fn defaults_decode<S: Selection>() {
    for x in [-3.0, -0.5, 0.0, 0.7, 4.0] {
        let m = <S::MassLink as Link<2, 3>>::forward([x, -x / 2.0]);
        for (got, want) in m.iter().zip(softmax_tanh_doc(&[x, -x / 2.0])) {
            assert!(
                (got - want).abs() <= 8.0 * f64::EPSILON,
                "mass at {x} (defaults)"
            );
        }
        let a = <S::AlphaLink as Link<1, 1>>::forward([x])[0];
        assert!(
            (a - HALF_PI * sigma(x)).abs() <= 4.0 * f64::EPSILON,
            "α at {x} (defaults)"
        );
        let b = <S::BetaLink as Link<1, 1>>::forward([x])[0];
        assert!(
            (b - PI * sigma(x)).abs() <= 8.0 * f64::EPSILON,
            "β at {x} (defaults)"
        );
        for q in [
            <S::Q0Link as Link<1, 1>>::forward([x])[0],
            <S::Q1Link as Link<1, 1>>::forward([x])[0],
            <S::Q2Link as Link<1, 1>>::forward([x])[0],
            <S::Q3Link as Link<1, 1>>::forward([x])[0],
        ] {
            let want = Q_MAX * (2.0 * sigma(x) - 1.0);
            assert!(
                (q - want).abs() <= 8.0 * f64::EPSILON,
                "q at {x}: {q} (defaults)"
            );
        }
    }
}

#[test]
fn qa_link_selection_defaults() {
    check_defaults(<Defaults as Selection>::LINK_IDS);
    defaults_decode::<Defaults>();
    // Another selection is another type, with its own ids: the selection is baked per block.
    type Swapped =
        Select<StickBreaking, TanhAlpha, SoftsignBeta, TanhQ, SigmoidQ, SoftsignQ, TanhQ>;
    assert_eq!(
        <Swapped as Selection>::LINK_IDS,
        [
            "stick_breaking",
            "tanh_alpha",
            "softsign_beta",
            "tanh_q",
            "sigmoid_q",
            "softsign_q",
            "tanh_q"
        ]
    );
}

negative_control!(
    qa_link_selection_defaults,
    "a selection with tanh for a free momentum is not the defaults",
    expected = "(defaults)",
    check_defaults(
        <Select<SoftmaxTanh, SigmoidAlpha, SigmoidBeta, TanhQ, SigmoidQ, SigmoidQ, SigmoidQ> as Selection>::LINK_IDS
    )
);

// ── The chart constants (REQ-DEC-009) ───────────────────────────────────────────────────────────────────────────

/// The generated constants, in the order μ_max, q_max, α_min, ε_μ, ε_z, ε_q, δ_λ, ε_w, against REQ-DEC-009's.
fn check_constants(got: [f64; 8], at: fn(f64) -> f64) {
    let want = [MU_MAX, Q_MAX, ALPHA_MIN, EPS, EPS, EPS, DELTA_LAMBDA, EPS_W];
    for (g, w) in got.iter().zip(want) {
        assert!(
            g.to_bits() == at(w).to_bits(),
            "the generated chart constants {got:?} are not REQ-DEC-009's {want:?} (constants)"
        );
    }
}

fn constants<R: ChartConstants + Into<f64>>() -> [f64; 8] {
    [
        R::MU_MAX.into(),
        R::Q_MAX.into(),
        R::ALPHA_MIN.into(),
        R::EPS_MU.into(),
        R::EPS_Z.into(),
        R::EPS_Q.into(),
        R::DELTA_LAMBDA.into(),
        R::EPS_W.into(),
    ]
}

#[test]
fn qa_chart_constants_values() {
    check_constants(constants::<f64>(), |v| v);
    check_constants(constants::<f32>(), |v| f64::from(v as f32));
}

negative_control!(
    qa_chart_constants_values,
    "ε_q = 10⁻⁵ is not REQ-DEC-009's value",
    expected = "(constants)",
    {
        let mut c = constants::<f64>();
        c[5] = 1e-5;
        check_constants(c, |v| v)
    }
);

/// The numbers of the float literals in Rust `source`, comments and strings dropped.
fn literals(source: &str) -> Vec<(usize, f64)> {
    let mut found = Vec::new();
    for (n, line) in source.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        let mut cleaned = String::new();
        let mut in_str = false;
        for c in code.chars() {
            if c == '"' {
                in_str = !in_str;
            } else if !in_str {
                cleaned.push(c);
            }
        }
        let b: Vec<char> = cleaned.chars().collect();
        let mut i = 0;
        while i < b.len() {
            let starts = b[i].is_ascii_digit()
                && (i == 0 || !(b[i - 1].is_alphanumeric() || b[i - 1] == '_' || b[i - 1] == '.'));
            if !starts {
                i += 1;
                continue;
            }
            let mut j = i;
            while j < b.len()
                && (b[j].is_ascii_alphanumeric()
                    || b[j] == '_'
                    || (b[j] == '.' && b.get(j + 1).is_some_and(|c| c.is_ascii_digit()))
                    || ((b[j] == '-' || b[j] == '+') && matches!(b[j - 1], 'e' | 'E')))
            {
                j += 1;
            }
            let token: String = b[i..j].iter().filter(|c| **c != '_').collect();
            let digits = token.trim_end_matches("f32").trim_end_matches("f64");
            let is_float = token.contains('.')
                || token.contains(['e', 'E'])
                || token.ends_with("f32")
                || token.ends_with("f64");
            if is_float {
                if let Ok(v) = digits.parse::<f64>() {
                    found.push((n + 1, v));
                }
            }
            i = j;
        }
    }
    found
}

/// No file of `files` writes μ_max, ε_μ = ε_z = ε_q, δ_λ or ε_w as a float literal (q_max's 2 and α_min's 0 are also
/// every formula's own numbers, so a scan cannot tell them apart; their names are checked through the registry's
/// trees in ledger's QA tests).
fn check_no_literals(files: &[(String, String)]) {
    for (path, source) in files {
        for (line, v) in literals(source) {
            assert!(
                ![MU_MAX, EPS, DELTA_LAMBDA, EPS_W].contains(&v),
                "{path}:{line}: the literal {v:e} is a chart constant's value; the formulae name it (literal)"
            );
        }
    }
}

/// Every `.rs` file under `dir` whose path, below `dir`, names decode or encode, as a directory or a file.
fn decode_encode_sources(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            decode_encode_sources(&path, root, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            if rel.contains("decode") || rel.contains("encode") {
                let source = std::fs::read_to_string(&path).expect("a kernel source reads");
                out.push((rel, source));
            }
        }
    }
}

#[test]
fn qa_chart_constants_named_not_written() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    decode_encode_sources(&src, &src, &mut files);
    // The generated link functions are the decode's and encode's link formulae (§3.9): they too read each constant by
    // name.
    let links = src.join("generated").join("links.rs");
    files.push((
        "generated/links.rs".to_owned(),
        std::fs::read_to_string(links).expect("the generated links read"),
    ));
    check_no_literals(&files);
}

negative_control!(
    qa_chart_constants_named_not_written,
    "an encode file writing ε_μ as 1e-6 must fail the scan",
    expected = "(literal)",
    check_no_literals(&[(
        "encode.rs".to_owned(),
        "let cap = (1.0 - 1e-6) * R::MU_MAX; // clamp\n".to_owned()
    )])
);
