//! A forward-mode dual number over f64, for the host: `v + d·ε` with `ε² = 0`, so a decode run at [`Dual`] returns its
//! slope along the seeded direction with the value it would have had at f64. It implements the traits the decoder
//! is generic over (`Real` with `num_traits::Float`, `IeeeBits`, `ExactSign` and `WarpReal`), so the one decode source
//! gives the exact `J_D` column and nothing is written twice.
//!
//! Every value goes through the same function the f64 path calls (`Real::sine` and the rest, so the pinned libm), never
//! `f64`'s inherent method: the value part is then bit-for-bit the f64 decode's, and a test holds it to that.
//!
//! **The bit-level steps** (`IeeeBits`, `ExactSign`, `WarpReal`) act on the value: a select or a bit test has slope 0
//! where it picks a constant, and the slope of the picked operand where it passes one on; `limbs` splits the value and
//! gives the tangent to the last limb, so the limbs still sum to `v + d·ε`. The exact feasibility test reads only signs
//! and orderings out of them, so what their tangents are never reaches a slope; the kink sites ([`crate::slope`]) are
//! where such a step changes branch.
//!
//! Host-only: the SPIR-V build does not take this module.

use core::cmp::Ordering;
use core::num::FpCategory;
use core::ops::{Add, Div, Mul, Neg, Rem, Sub};

use spirv_std::num_traits::{Float, Num, NumCast, One, ToPrimitive, Zero};

use crate::chart::invariant::WarpReal;
use crate::decode::invariant::{Binade, ExactSign};
use crate::generated::constants::ChartConstants;
use crate::generated::links::Literals;
use crate::payload::PayloadReal;
use crate::real::{IeeeBits, Real};

/// `v + d·ε`: a value and its slope along the seeded direction.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dual {
    /// The value.
    pub v: f64,
    /// The slope.
    pub d: f64,
}

impl Dual {
    /// A constant: slope 0.
    pub const fn constant(v: f64) -> Dual {
        Dual { v, d: 0.0 }
    }

    /// The variable the slope is taken along: slope 1.
    pub const fn variable(v: f64) -> Dual {
        Dual { v, d: 1.0 }
    }

    /// `2^{-e}`, the slope of `|x|/2^e` in `x`, and 0 where `x` is zero, subnormal or not finite: there the bits give no
    /// binade a nearby float shares (`pow2` takes a normal exponent only, and `2^{-e}` for `e = −1023` is not one).
    fn inverse_pow2(e: i32, zero: bool) -> f64 {
        if zero || !(-1022..=1023).contains(&e) {
            0.0
        } else {
            <f64 as IeeeBits>::pow2(-e)
        }
    }

    /// `atan2(y, x)` and its slope, the value the pinned libm's.
    fn atan2_of(y: Dual, x: Dual) -> Dual {
        let n = y.v * y.v + x.v * x.v;
        Dual {
            v: Real::arctan2(y.v, x.v),
            d: (x.v * y.d - y.v * x.d) / n,
        }
    }

    /// `f(self)` with value `v` and slope `df·self.d`, the chain rule's one step.
    #[inline]
    fn chain(self, v: f64, df: f64) -> Dual {
        Dual { v, d: df * self.d }
    }
}

impl PartialEq for Dual {
    fn eq(&self, o: &Dual) -> bool {
        self.v == o.v
    }
}

impl PartialOrd for Dual {
    fn partial_cmp(&self, o: &Dual) -> Option<Ordering> {
        self.v.partial_cmp(&o.v)
    }
}

impl Add for Dual {
    type Output = Dual;
    #[inline]
    fn add(self, o: Dual) -> Dual {
        Dual {
            v: self.v + o.v,
            d: self.d + o.d,
        }
    }
}

impl Sub for Dual {
    type Output = Dual;
    #[inline]
    fn sub(self, o: Dual) -> Dual {
        Dual {
            v: self.v - o.v,
            d: self.d - o.d,
        }
    }
}

impl Mul for Dual {
    type Output = Dual;
    #[inline]
    fn mul(self, o: Dual) -> Dual {
        Dual {
            v: self.v * o.v,
            d: self.d * o.v + self.v * o.d,
        }
    }
}

impl Div for Dual {
    type Output = Dual;
    #[inline]
    fn div(self, o: Dual) -> Dual {
        let q = self.v / o.v;
        Dual {
            v: q,
            d: (self.d - q * o.d) / o.v,
        }
    }
}

impl Rem for Dual {
    type Output = Dual;
    #[inline]
    fn rem(self, o: Dual) -> Dual {
        Dual {
            v: self.v % o.v,
            d: self.d - o.d * Float::trunc(self.v / o.v),
        }
    }
}

impl Neg for Dual {
    type Output = Dual;
    #[inline]
    fn neg(self) -> Dual {
        Dual {
            v: -self.v,
            d: -self.d,
        }
    }
}

impl Zero for Dual {
    fn zero() -> Dual {
        Dual::constant(0.0)
    }
    fn is_zero(&self) -> bool {
        self.v == 0.0
    }
}

impl One for Dual {
    fn one() -> Dual {
        Dual::constant(1.0)
    }
}

impl Num for Dual {
    type FromStrRadixErr = <f64 as Num>::FromStrRadixErr;
    fn from_str_radix(s: &str, radix: u32) -> Result<Dual, Self::FromStrRadixErr> {
        <f64 as Num>::from_str_radix(s, radix).map(Dual::constant)
    }
}

impl ToPrimitive for Dual {
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

impl NumCast for Dual {
    fn from<T: ToPrimitive>(n: T) -> Option<Dual> {
        n.to_f64().map(Dual::constant)
    }
}

impl From<f64> for Dual {
    fn from(v: f64) -> Dual {
        Dual::constant(v)
    }
}

impl From<f32> for Dual {
    fn from(v: f32) -> Dual {
        Dual::constant(v as f64)
    }
}

impl Float for Dual {
    fn nan() -> Dual {
        Dual::constant(<f64 as Float>::nan())
    }
    fn infinity() -> Dual {
        Dual::constant(<f64 as Float>::infinity())
    }
    fn neg_infinity() -> Dual {
        Dual::constant(<f64 as Float>::neg_infinity())
    }
    fn neg_zero() -> Dual {
        Dual::constant(-0.0)
    }
    fn min_value() -> Dual {
        Dual::constant(<f64 as Float>::min_value())
    }
    fn min_positive_value() -> Dual {
        Dual::constant(<f64 as Float>::min_positive_value())
    }
    fn epsilon() -> Dual {
        Dual::constant(<f64 as Float>::epsilon())
    }
    fn max_value() -> Dual {
        Dual::constant(<f64 as Float>::max_value())
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
    // The steps below are constant between jumps: slope 0, the jump a kink site.
    fn floor(self) -> Dual {
        Dual::constant(Float::floor(self.v))
    }
    fn ceil(self) -> Dual {
        Dual::constant(Float::ceil(self.v))
    }
    fn round(self) -> Dual {
        Dual::constant(Float::round(self.v))
    }
    fn trunc(self) -> Dual {
        Dual::constant(Float::trunc(self.v))
    }
    fn fract(self) -> Dual {
        self.chain(Float::fract(self.v), 1.0)
    }
    fn abs(self) -> Dual {
        self.chain(
            Float::abs(self.v),
            if self.v.is_sign_negative() { -1.0 } else { 1.0 },
        )
    }
    fn signum(self) -> Dual {
        Dual::constant(Float::signum(self.v))
    }
    fn is_sign_positive(self) -> bool {
        self.v.is_sign_positive()
    }
    fn is_sign_negative(self) -> bool {
        self.v.is_sign_negative()
    }
    fn mul_add(self, a: Dual, b: Dual) -> Dual {
        self * a + b
    }
    fn recip(self) -> Dual {
        let r = 1.0 / self.v;
        self.chain(r, -r * r)
    }
    fn powi(self, n: i32) -> Dual {
        let v = Float::powi(self.v, n);
        if n == 0 {
            Dual::constant(v)
        } else {
            self.chain(v, (n as f64) * Float::powi(self.v, n - 1))
        }
    }
    fn powf(self, n: Dual) -> Dual {
        let v = Real::power(self.v, n.v);
        let dv = if self.d == 0.0 {
            0.0
        } else {
            n.v * Real::power(self.v, n.v - 1.0) * self.d
        };
        let dn = if n.d == 0.0 {
            0.0
        } else {
            Real::logarithm(self.v) * v * n.d
        };
        Dual { v, d: dv + dn }
    }
    fn sqrt(self) -> Dual {
        let s = Float::sqrt(self.v);
        self.chain(s, 0.5 / s)
    }
    fn exp(self) -> Dual {
        let e = Real::exponential(self.v);
        self.chain(e, e)
    }
    fn exp2(self) -> Dual {
        let e = Real::power(2.0, self.v);
        self.chain(e, e * core::f64::consts::LN_2)
    }
    fn ln(self) -> Dual {
        self.chain(Real::logarithm(self.v), 1.0 / self.v)
    }
    fn log(self, base: Dual) -> Dual {
        Real::logarithm(self) / Real::logarithm(base)
    }
    fn log2(self) -> Dual {
        self.chain(
            Real::logarithm(self.v) / core::f64::consts::LN_2,
            1.0 / (self.v * core::f64::consts::LN_2),
        )
    }
    fn log10(self) -> Dual {
        self.chain(
            Real::logarithm(self.v) / core::f64::consts::LN_10,
            1.0 / (self.v * core::f64::consts::LN_10),
        )
    }
    fn max(self, o: Dual) -> Dual {
        if o.v > self.v || self.v.is_nan() {
            o
        } else {
            self
        }
    }
    fn min(self, o: Dual) -> Dual {
        if o.v < self.v || self.v.is_nan() {
            o
        } else {
            self
        }
    }
    fn abs_sub(self, o: Dual) -> Dual {
        if self.v > o.v {
            self - o
        } else {
            Dual::zero()
        }
    }
    fn cbrt(self) -> Dual {
        let c = Real::cube_root(self.v);
        self.chain(c, 1.0 / (3.0 * c * c))
    }
    fn hypot(self, o: Dual) -> Dual {
        let h = Real::hypotenuse(self.v, o.v);
        Dual {
            v: h,
            d: (self.v * self.d + o.v * o.d) / h,
        }
    }
    fn sin(self) -> Dual {
        self.chain(Real::sine(self.v), Real::cosine(self.v))
    }
    fn cos(self) -> Dual {
        self.chain(Real::cosine(self.v), -Real::sine(self.v))
    }
    fn tan(self) -> Dual {
        let t = Real::tangent(self.v);
        self.chain(t, 1.0 + t * t)
    }
    fn asin(self) -> Dual {
        self.chain(
            Real::arcsine(self.v),
            1.0 / Float::sqrt(1.0 - self.v * self.v),
        )
    }
    fn acos(self) -> Dual {
        self.chain(
            Real::arccosine(self.v),
            -1.0 / Float::sqrt(1.0 - self.v * self.v),
        )
    }
    fn atan(self) -> Dual {
        self.chain(Real::arctan2(self.v, 1.0), 1.0 / (1.0 + self.v * self.v))
    }
    fn atan2(self, x: Dual) -> Dual {
        Dual::atan2_of(self, x)
    }
    fn sin_cos(self) -> (Dual, Dual) {
        (Real::sine(self), Real::cosine(self))
    }
    fn exp_m1(self) -> Dual {
        self.chain(Real::exp_minus_one(self.v), Real::exponential(self.v))
    }
    fn ln_1p(self) -> Dual {
        self.chain(Real::ln_one_plus(self.v), 1.0 / (1.0 + self.v))
    }
    fn sinh(self) -> Dual {
        self.chain(
            Real::hyperbolic_sine(self.v),
            Real::hyperbolic_cosine(self.v),
        )
    }
    fn cosh(self) -> Dual {
        self.chain(
            Real::hyperbolic_cosine(self.v),
            Real::hyperbolic_sine(self.v),
        )
    }
    fn tanh(self) -> Dual {
        let t = Real::hyperbolic_tangent(self.v);
        self.chain(t, 1.0 - t * t)
    }
    fn asinh(self) -> Dual {
        let root = Float::sqrt(self.v * self.v + 1.0);
        self.chain(Real::logarithm(self.v + root), 1.0 / root)
    }
    fn acosh(self) -> Dual {
        let root = Float::sqrt(self.v * self.v - 1.0);
        self.chain(Real::logarithm(self.v + root), 1.0 / root)
    }
    fn atanh(self) -> Dual {
        self.chain(Real::arctanh(self.v), 1.0 / (1.0 - self.v * self.v))
    }
    fn integer_decode(self) -> (u64, i16, i8) {
        Float::integer_decode(self.v)
    }
    fn to_degrees(self) -> Dual {
        self.chain(Float::to_degrees(self.v), 180.0 / core::f64::consts::PI)
    }
    fn to_radians(self) -> Dual {
        self.chain(Float::to_radians(self.v), core::f64::consts::PI / 180.0)
    }
}

impl Real for Dual {
    const NAME: &'static str = "dual";

    // A dual stays on the host's f64 maths, so these route to the f64 path's own functions.
    fn arctan2(self, x: Dual) -> Dual {
        Dual::atan2_of(self, x)
    }
    fn arctangent2(self, x: Dual) -> Dual {
        Dual::atan2_of(self, x)
    }
}

impl ChartConstants for Dual {
    const MU_MAX: Dual = Dual::constant(<f64 as ChartConstants>::MU_MAX);
    const Q_MAX: Dual = Dual::constant(<f64 as ChartConstants>::Q_MAX);
    const ALPHA_MIN: Dual = Dual::constant(<f64 as ChartConstants>::ALPHA_MIN);
    const EPS_MU: Dual = Dual::constant(<f64 as ChartConstants>::EPS_MU);
    const EPS_Z: Dual = Dual::constant(<f64 as ChartConstants>::EPS_Z);
    const EPS_Q: Dual = Dual::constant(<f64 as ChartConstants>::EPS_Q);
    const DELTA_LAMBDA: Dual = Dual::constant(<f64 as ChartConstants>::DELTA_LAMBDA);
    const EPS_W: Dual = Dual::constant(<f64 as ChartConstants>::EPS_W);
    const EPS_M01: Dual = Dual::constant(<f64 as ChartConstants>::EPS_M01);
}

impl Literals for Dual {
    const LIT_0: Dual = Dual::constant(<f64 as Literals>::LIT_0);
    const LIT_1: Dual = Dual::constant(<f64 as Literals>::LIT_1);
    const LIT_2: Dual = Dual::constant(<f64 as Literals>::LIT_2);
    const LIT_3: Dual = Dual::constant(<f64 as Literals>::LIT_3);
    const LIT_4: Dual = Dual::constant(<f64 as Literals>::LIT_4);
    const LIT_5: Dual = Dual::constant(<f64 as Literals>::LIT_5);
    const LIT_6: Dual = Dual::constant(<f64 as Literals>::LIT_6);
    const LIT_7: Dual = Dual::constant(<f64 as Literals>::LIT_7);
    const LIT_8: Dual = Dual::constant(<f64 as Literals>::LIT_8);
}

impl PayloadReal for Dual {
    type SimStateFTLETail = [u32; 0];
    type SimStateBaseTail = [u32; 0];
    type ICDescriptorTail = [u32; 0];
}

impl ExactSign for Dual {
    fn binade(self) -> Binade<Dual> {
        let b = ExactSign::binade(self.v);
        // `|self| = s·2^e` with `e` a constant of the binade, so `s` moves with `self` at slope `±2^{-e}`.
        let sign = if b.negative { -1.0 } else { 1.0 };
        let scale = sign * Dual::inverse_pow2(b.e, b.zero);
        Binade {
            negative: b.negative,
            zero: b.zero,
            s: Dual {
                v: b.s,
                d: scale * self.d,
            },
            e: b.e,
        }
    }
    fn limbs(self) -> [Dual; 3] {
        let [a, b, c] = ExactSign::limbs(self.v);
        [
            Dual::constant(a),
            Dual::constant(b),
            Dual { v: c, d: self.d },
        ]
    }
    fn two_to(e: i32) -> Dual {
        Dual::constant(<f64 as ExactSign>::two_to(e))
    }
    fn doubling_limit() -> i32 {
        <f64 as ExactSign>::doubling_limit()
    }
}

impl WarpReal for Dual {
    fn below(self) -> Dual {
        Dual {
            v: WarpReal::below(self.v),
            d: self.d,
        }
    }
    fn above(self) -> Dual {
        Dual {
            v: WarpReal::above(self.v),
            d: self.d,
        }
    }
}

impl IeeeBits for Dual {
    fn or_pos_inf(self, c: bool) -> Dual {
        Dual {
            v: self.v.or_pos_inf(c),
            d: if c { 0.0 } else { self.d },
        }
    }
    fn or_neg_inf(self, c: bool) -> Dual {
        Dual {
            v: self.v.or_neg_inf(c),
            d: if c { 0.0 } else { self.d },
        }
    }
    fn non_finite_bits(self) -> bool {
        self.v.non_finite_bits()
    }
    fn nan_bits(self) -> bool {
        self.v.nan_bits()
    }
    // Don't pass the bits' exponent of a zero or subnormal on (`−1023`): the decode scales by `pow2(−exponent)` and back,
    // and `2^{−1023}` is not a normal float, so the slope of a scaled zero is multiplied by `2^1023` and not given back.
    fn exponent(self) -> i32 {
        let e = IeeeBits::exponent(self.v);
        if (-1022..=1023).contains(&e) {
            e
        } else {
            0
        }
    }
    fn pow2(e: i32) -> Dual {
        Dual::constant(<f64 as IeeeBits>::pow2(e))
    }
    fn significand(self) -> Dual {
        let e = IeeeBits::exponent(self.v);
        let sign = if self.v.is_sign_negative() { -1.0 } else { 1.0 };
        Dual {
            v: IeeeBits::significand(self.v),
            d: sign * Dual::inverse_pow2(e, self.v == 0.0) * self.d,
        }
    }
    fn product(self, b: Dual) -> Dual {
        Dual {
            v: IeeeBits::product(self.v, b.v),
            d: self.d * b.v + self.v * b.d,
        }
    }
    const CARRIES_SLOPE: bool = true;
    fn slope_from(self, smooth: Dual) -> Dual {
        Dual {
            v: self.v,
            d: smooth.d,
        }
    }
    const INV_PHI: Dual = Dual::constant(<f64 as IeeeBits>::INV_PHI);
    const TAU_I: Dual = Dual::constant(<f64 as IeeeBits>::TAU_I);
}
