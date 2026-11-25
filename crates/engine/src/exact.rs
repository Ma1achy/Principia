//! Correctly rounded f64 maths for the chart path: each function returns the f64 nearest the exact value (ties to even),
//! so its bits are the same on every platform.
//!
//! **Why.** std's `exp`, `hypot`, `sin_cos`, … are the platform's own maths library, whose last bit differs between
//! platforms: at the `ν` curve's centre macOS's `hypot` and `exp` are each an ulp off the exact value where Linux's
//! results are exact, and a curve tilt's central difference amplifies that, so the Mac's chart path had other bits than
//! Linux's. The pinned libm is an ulp off there too, so it agrees with neither everywhere; the exact value is the one
//! answer that does not depend on a library.
//!
//! **How.** Unsigned fixed point, 64 integer and 320 fraction bits ([`Fix`]), integer arithmetic alone: Taylor series
//! after an argument reduction, about 2⁻³⁰⁰ from the exact value, rounded once. `hypot` compares exact squares. No
//! crate: the convergence reference's arbitrary-precision crate stays out of the engine it judges
//! (`validation::reference::brutus_gate`).
//!
//! **Range.** Outside the ranges each function names (a huge angle, an overflow, a subnormal result, a non-finite
//! input) it is the pinned libm's value: the same on every platform, but not promised correctly rounded. `cbrt` is the
//! pinned libm's, CORE-MATH's correctly rounded one.
//!
//! A call costs some microseconds: it is for view edits, not for bulk work. `lookup`, which calls its maths many times a
//! search, uses the pinned libm, the same on every platform.

/// A non-negative fixed-point number, least limb first, the last limb its integer part.
type Fix = [u64; 6];

/// [`Fix`]'s fraction bits: every limb but the integer one.
const FRAC: u32 = ((size_of::<Fix>() - size_of::<u64>()) / size_of::<u64>()) as u32 * u64::BITS;

/// `ε²`: an `x` below it leaves `eˣ` within `ε²` of 1, which rounds to 1.
const TINY: f64 = f64::EPSILON * f64::EPSILON;

/// 1.
fn one() -> Fix {
    fix(1, 0)
}

/// What the reductions read, each from its own series, computed once.
struct Constants {
    ln2: Fix,
    pi: Fix,
    /// `atan` of 1/4, 1/2 and 3/4.
    atan_quarters: [Fix; 3],
}

fn constants() -> &'static Constants {
    static HELD: std::sync::OnceLock<Constants> = std::sync::OnceLock::new();
    HELD.get_or_init(|| {
        let atan_over = |n: u64| odd_series(&over(&one(), n), true);
        // `ln 2 = 2·artanh(1/3)`; Machin's `π = 16·atan(1/5) − 4·atan(1/239)`; `atan(3/4) = 2·atan(1/3)`.
        Constants {
            ln2: times(&odd_series(&over(&one(), 3), false), 2),
            pi: sub(&times(&atan_over(5), 16), &times(&atan_over(239), 4)),
            atan_quarters: [atan_over(4), atan_over(2), times(&atan_over(3), 2)],
        }
    })
}

/// `Σ ±u^(2n+1)/(2n+1)`: `atan u` with the signs alternating, `artanh u` without.
fn odd_series(u: &Fix, alternating: bool) -> Fix {
    let uu = mul(u, u);
    let (mut sums, mut power) = ([*u, [0; 6]], *u);
    for n in 1usize.. {
        power = mul(&power, &uu);
        if is_zero(&power) {
            break;
        }
        sums[n % 2] = add(&sums[n % 2], &over(&power, 2 * n as u64 + 1));
    }
    if alternating {
        sub(&sums[0], &sums[1])
    } else {
        add(&sums[0], &sums[1])
    }
}

fn add(a: &Fix, b: &Fix) -> Fix {
    let mut carry = 0u128;
    std::array::from_fn(|i| {
        let s = u128::from(a[i]) + u128::from(b[i]) + carry;
        carry = s >> 64;
        s as u64
    })
}

/// `a − b`, for `a ≥ b`.
fn sub(a: &Fix, b: &Fix) -> Fix {
    let mut borrow = false;
    std::array::from_fn(|i| {
        let (d, b1) = a[i].overflowing_sub(b[i]);
        let (d, b2) = d.overflowing_sub(u64::from(borrow));
        borrow = b1 || b2;
        d
    })
}

fn less(a: &Fix, b: &Fix) -> bool {
    a.iter().rev().lt(b.iter().rev())
}

fn is_zero(a: &Fix) -> bool {
    a.iter().all(|w| *w == 0)
}

/// `a·b`, cut.
fn mul(a: &Fix, b: &Fix) -> Fix {
    let mut wide = [0u64; 12];
    for (i, x) in a.iter().enumerate() {
        let mut carry = 0u128;
        for (j, y) in b.iter().enumerate() {
            let s = u128::from(*x) * u128::from(*y) + u128::from(wide[i + j]) + carry;
            wide[i + j] = s as u64;
            carry = s >> 64;
        }
        wide[i + 6] = carry as u64;
    }
    std::array::from_fn(|i| wide[i + 5])
}

/// `a·k`.
fn times(a: &Fix, k: u64) -> Fix {
    let mut carry = 0u128;
    std::array::from_fn(|i| {
        let s = u128::from(a[i]) * u128::from(k) + carry;
        carry = s >> 64;
        s as u64
    })
}

/// `a/k`, cut.
fn over(a: &Fix, k: u64) -> Fix {
    let mut out = [0; 6];
    let mut rem = 0u128;
    for i in (0..6).rev() {
        let n = (rem << 64) | u128::from(a[i]);
        out[i] = (n / u128::from(k)) as u64;
        rem = n % u128::from(k);
    }
    out
}

/// `a/b`, cut, for `a < 2b`.
fn div(a: &Fix, b: &Fix) -> Fix {
    let (mut rem, mut out) = (*a, [0u64; 6]);
    for bit in (0..=FRAC as usize).rev() {
        if !less(&rem, b) {
            rem = sub(&rem, b);
            out[bit / 64] |= 1 << (bit % 64);
        }
        rem = add(&rem, &rem);
    }
    out
}

/// The signed sum of `(negative, magnitude)` pairs.
fn plus(a: (bool, Fix), b: (bool, Fix)) -> (bool, Fix) {
    if a.0 == b.0 {
        (a.0, add(&a.1, &b.1))
    } else if less(&a.1, &b.1) {
        (b.0, sub(&b.1, &a.1))
    } else {
        (a.0, sub(&a.1, &b.1))
    }
}

/// `|x| = m·2^e`, `x` normal.
fn parts(x: f64) -> (u64, i32) {
    let bits = x.to_bits();
    let e = ((bits >> 52) & 0x7ff) as i32;
    ((bits & ((1 << 52) - 1)) | (1 << 52), e - 1075)
}

/// `m·2^e`, exactly, for `2^−FRAC ≤ 2^e` and a value below 2⁶⁴.
fn fix(m: u64, e: i32) -> Fix {
    let at = (e + FRAC as i32) as usize;
    let wide = u128::from(m) << (at % 64);
    let mut out = [0; 6];
    out[at / 64] = wide as u64;
    if at / 64 < 5 {
        out[at / 64 + 1] = (wide >> 64) as u64;
    }
    out
}

/// `x`'s magnitude, exactly; `x` normal, `2⁻²⁶⁰ < |x| < 2⁶³`.
fn magnitude(x: f64) -> Fix {
    let (m, e) = parts(x);
    fix(m, e)
}

/// The f64 nearest `±v·2^k`, `None` where that is not normal. A remainder within 2⁻¹²⁰ of half an ulp is a tie, to
/// even: only an exact result (a `pow`) comes that near one.
fn round(negative: bool, v: &Fix, k: i32) -> Option<f64> {
    let limb = (0..6).rev().find(|i| v[*i] != 0)?;
    let top = limb * 64 + 63 - v[limb].leading_zeros() as usize;
    let bits = |low: usize, n: usize| -> u128 {
        (0..n).fold(0, |a, i| {
            let at = low + i;
            a | u128::from((v[at / 64] >> (at % 64)) & 1) << i
        })
    };
    let low = top.checked_sub(173)?;
    let (mut m, rest) = (bits(top - 52, 53) as u64, bits(low, 121));
    let half = 1u128 << 120;
    let tie = rest == half || rest == half - 1;
    if (tie && m & 1 == 1) || (!tie && rest > half) {
        m += 1;
    }
    let mut e = top as i32 - FRAC as i32 + k;
    if m == 1 << 53 {
        m >>= 1;
        e += 1;
    }
    let sign = u64::from(negative) << 63;
    (-1022..=1023)
        .contains(&e)
        .then(|| f64::from_bits(sign | ((e + 1023) as u64) << 52 | (m & ((1 << 52) - 1))))
}

/// `e^r` for `r ≤ ln 2`.
fn exp_series(r: &Fix) -> Fix {
    let (mut sum, mut term) = (one(), one());
    for n in 1u64.. {
        term = over(&mul(&term, r), n);
        if is_zero(&term) {
            break;
        }
        sum = add(&sum, &term);
    }
    sum
}

/// `e^{±x}`, `x < 2³²`, as `(v, k)`, the value `v·2^k`.
fn exp_fix(negative: bool, x: &Fix) -> (Fix, i32) {
    let near = x[5] as f64 + x[4] as f64 / 18_446_744_073_709_551_616.0;
    let mut k = (near / std::f64::consts::LN_2) as u64;
    while less(x, &times(&constants().ln2, k)) {
        k -= 1;
    }
    let mut r = sub(x, &times(&constants().ln2, k));
    while !less(&r, &constants().ln2) {
        r = sub(&r, &constants().ln2);
        k += 1;
    }
    if negative {
        (exp_series(&sub(&constants().ln2, &r)), -(k as i32) - 1)
    } else {
        (exp_series(&r), k as i32)
    }
}

/// `ln x`, signed, `x` normal.
fn ln_fix(x: f64) -> (bool, Fix) {
    let (m, e) = parts(x);
    let mut e = e + 52;
    // `v` in (3/4, 3/2].
    let v = if m > 3 << 51 {
        e += 1;
        fix(m, -53)
    } else {
        fix(m, -52)
    };
    let below = less(&v, &one());
    let gap = if below {
        sub(&one(), &v)
    } else {
        sub(&v, &one())
    };
    let z = div(&gap, &add(&v, &one()));
    let sum = odd_series(&z, false);
    let whole = times(&constants().ln2, u64::from(e.unsigned_abs()));
    plus((e < 0, whole), (below, add(&sum, &sum)))
}

/// `(sin r, cos r)` for `r` in `[0, π/2)`.
fn sin_cos_series(r: &Fix) -> (Fix, Fix) {
    let mut sums = [one(), [0; 6], [0; 6], [0; 6]];
    let mut term = one();
    for n in 1usize.. {
        term = over(&mul(&term, r), n as u64);
        if is_zero(&term) {
            break;
        }
        sums[n % 4] = add(&sums[n % 4], &term);
    }
    (sub(&sums[1], &sums[3]), sub(&sums[0], &sums[2]))
}

/// `(quadrant mod 4, r)`, `|x| = quadrant·π/2 + r`, for `|x| < 2²⁰`.
fn quadrant(x: f64) -> (u64, Fix) {
    let half_pi = over(&constants().pi, 2);
    let v = magnitude(x);
    let mut k = (x.abs() / std::f64::consts::FRAC_PI_2) as u64;
    while less(&v, &times(&half_pi, k)) {
        k -= 1;
    }
    let mut r = sub(&v, &times(&half_pi, k));
    while !less(&r, &half_pi) {
        r = sub(&r, &half_pi);
        k += 1;
    }
    (k % 4, r)
}

/// `atan t` for `t ≤ 1`: the nearest of `atan 0, ¼, ½, ¾, 1` and the series of what is left.
fn atan_fix(t: &Fix) -> Fix {
    let quarters = (t[5] << 2 | t[4] >> 62) + ((t[4] >> 61) & 1);
    let (base, at) = match quarters {
        0 => ([0; 6], [0; 6]),
        1 => (constants().atan_quarters[0], over(&one(), 4)),
        2 => (constants().atan_quarters[1], over(&one(), 2)),
        3 => (constants().atan_quarters[2], over(&times(&one(), 3), 4)),
        _ => (over(&constants().pi, 4), one()),
    };
    let below = less(t, &at);
    let gap = if below { sub(&at, t) } else { sub(t, &at) };
    let u = div(&gap, &add(&one(), &mul(t, &at)));
    let rest = odd_series(&u, true);
    if below {
        sub(&base, &rest)
    } else {
        add(&base, &rest)
    }
}

/// A per-thread direct-mapped memo of one function's results, keyed by its arguments' bits. The functions here cost
/// microseconds a call, more in a debug build, and the GUI's view searches call them on the same few arguments millions
/// of times; a pure function's memo returns the same bits. Don't drop it: the view tests run fifty times longer.
struct Memo<K, V> {
    slots: Vec<Option<(K, V)>>,
}

/// The memo's slots, `2^memo_bits()`: a table's size, not a number of the physics, so it is not a register constant.
fn memo_bits() -> u32 {
    12
}

impl<K: Copy + PartialEq, V: Copy> Memo<K, V> {
    fn new() -> Self {
        Self {
            slots: vec![None; 1 << memo_bits()],
        }
    }

    fn get_or(&mut self, key: K, hash: u64, f: impl FnOnce() -> V) -> V {
        let i = (hash.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - memo_bits())) as usize;
        match self.slots[i] {
            Some((k, v)) if k == key => v,
            _ => {
                let v = f();
                self.slots[i] = Some((key, v));
                v
            }
        }
    }
}

/// `f`'s result at `key`, through the memo `memo` (a thread-local).
fn memoised<K: Copy + PartialEq + 'static, V: Copy + 'static>(
    memo: &'static std::thread::LocalKey<std::cell::RefCell<Memo<K, V>>>,
    key: K,
    hash: u64,
    f: impl FnOnce() -> V,
) -> V {
    memo.with(|m| m.borrow_mut().get_or(key, hash, f))
}

fn exp_uncached(x: f64) -> f64 {
    if x.abs() < TINY {
        return 1.0;
    }
    if x.is_nan() || x.abs() > 708.0 {
        return libm::exp(x);
    }
    let (v, k) = exp_fix(x < 0.0, &magnitude(x));
    round(false, &v, k).unwrap_or_else(|| libm::exp(x))
}

/// `eˣ`, correctly rounded for `|x| ≤ 708`.
pub fn exp(x: f64) -> f64 {
    thread_local! {
        static MEMO: std::cell::RefCell<Memo<u64, f64>> = std::cell::RefCell::new(Memo::new());
    }
    let key = x.to_bits();
    memoised(&MEMO, key, key, || exp_uncached(x))
}

fn exp2_uncached(x: f64) -> f64 {
    if x.abs() < TINY {
        return 1.0;
    }
    if x.is_nan() || x.abs() > 1021.0 {
        return libm::exp2(x);
    }
    let mut part = magnitude(x);
    let mut k = std::mem::take(&mut part[5]) as i32;
    if x < 0.0 {
        k = -k;
        if !is_zero(&part) {
            part = sub(&one(), &part);
            k -= 1;
        }
    }
    round(false, &exp_series(&mul(&part, &constants().ln2)), k).unwrap_or_else(|| libm::exp2(x))
}

/// `2ˣ`, correctly rounded for `|x| ≤ 1021`.
pub fn exp2(x: f64) -> f64 {
    thread_local! {
        static MEMO: std::cell::RefCell<Memo<u64, f64>> = std::cell::RefCell::new(Memo::new());
    }
    let key = x.to_bits();
    memoised(&MEMO, key, key, || exp2_uncached(x))
}

fn ln_uncached(x: f64) -> f64 {
    if !(x.is_normal() && x > 0.0) || x == 1.0 {
        return libm::log(x);
    }
    let (negative, v) = ln_fix(x);
    round(negative, &v, 0).unwrap_or_else(|| libm::log(x))
}

/// `ln x`, correctly rounded for a normal `x > 0`.
pub fn ln(x: f64) -> f64 {
    thread_local! {
        static MEMO: std::cell::RefCell<Memo<u64, f64>> = std::cell::RefCell::new(Memo::new());
    }
    let key = x.to_bits();
    memoised(&MEMO, key, key, || ln_uncached(x))
}

fn log2_uncached(x: f64) -> f64 {
    if !(x.is_normal() && x > 0.0) || x == 1.0 {
        return libm::log2(x);
    }
    let k = ((x.to_bits() >> 52) & 0x7ff) as i64 - 1023;
    let m = f64::from_bits((x.to_bits() & ((1 << 52) - 1)) | 0x3ff0_0000_0000_0000);
    let fraction = if m == 1.0 {
        [0; 6]
    } else {
        div(&ln_fix(m).1, &constants().ln2)
    };
    let (negative, v) = plus((k < 0, fix(k.unsigned_abs(), 0)), (false, fraction));
    round(negative, &v, 0).unwrap_or_else(|| libm::log2(x))
}
/// `log₂ x`, correctly rounded for a normal `x > 0`: `x = m·2ᵏ`, `m ∈ [1, 2)`, and `k + ln m / ln 2` in fixed point,
/// rounded once (the quotient below 1, as [`div`] needs).
pub fn log2(x: f64) -> f64 {
    thread_local! {
        static MEMO: std::cell::RefCell<Memo<u64, f64>> = std::cell::RefCell::new(Memo::new());
    }
    let key = x.to_bits();
    memoised(&MEMO, key, key, || log2_uncached(x))
}

fn pow_uncached(x: f64, y: f64) -> f64 {
    let y_held = y.abs() > TINY && y.abs() < 1_048_576.0;
    if !(x.is_normal() && x > 0.0) || x == 1.0 || !y_held {
        return libm::pow(x, y);
    }
    let (negative, l) = ln_fix(x);
    let power = mul(&l, &magnitude(y));
    if power[5] >= 708 {
        return libm::pow(x, y);
    }
    let (v, k) = exp_fix(negative != (y < 0.0), &power);
    round(false, &v, k).unwrap_or_else(|| libm::pow(x, y))
}

/// `xʸ`, correctly rounded for a normal `x > 0`, `ε² < |y| < 2²⁰` and a result within `e^±708`.
pub fn pow(x: f64, y: f64) -> f64 {
    thread_local! {
        static MEMO: std::cell::RefCell<Memo<(u64, u64), f64>> = std::cell::RefCell::new(Memo::new());
    }
    let key = (x.to_bits(), y.to_bits());
    memoised(&MEMO, key, key.0 ^ key.1.rotate_left(29), || {
        pow_uncached(x, y)
    })
}

fn hypot_uncached(x: f64, y: f64) -> f64 {
    let held = |v: f64| v.is_normal() || v == 0.0;
    if !held(x) || !held(y) {
        return libm::hypot(x, y);
    }
    let (big, small) = (x.abs().max(y.abs()), x.abs().min(y.abs()));
    if small == 0.0 {
        return big;
    }
    let ((mb, eb), (ms, es)) = (parts(big), parts(small));
    if eb - es > 60 {
        // Within `big·2⁻¹¹⁹` of `big`.
        return big;
    }
    // `big` scaled into [1, 2); the sum of squares exact, its last bit at 2⁻²²⁴ or above.
    let (b, s) = (fix(mb, -52), fix(ms, es - eb - 52));
    let sum = add(&mul(&b, &b), &mul(&s, &s));
    // The f64 square root of the sum in f64, as a multiple of 2⁻⁵² or (past 2) 2⁻⁵¹: at most an ulp or two off.
    let near = sum[5] as f64 + sum[4] as f64 / 18_446_744_073_709_551_616.0;
    let (mut m, mut e) = parts(near.sqrt());
    // `(m + ½)·2^e` squared against the sum.
    let mid = |m: u64, e: i32| {
        let v = fix(2 * m + 1, e - 1);
        mul(&v, &v)
    };
    loop {
        let (up, down) = (mid(m, e), mid(m - 1, e));
        if less(&up, &sum) || (up == sum && m & 1 == 1) {
            m += 1;
        } else if less(&sum, &down) || (down == sum && m & 1 == 1) {
            m -= 1;
        } else {
            break;
        }
        if m == 1 << 53 {
            m >>= 1;
            e += 1;
        }
    }
    let e = e + 52 + eb + 52;
    if !(-1022..=1023).contains(&e) {
        return libm::hypot(x, y);
    }
    f64::from_bits(((e + 1023) as u64) << 52 | (m & ((1 << 52) - 1)))
}

/// `√(x² + y²)`, correctly rounded for a normal result and normal or zero `x` and `y`.
pub fn hypot(x: f64, y: f64) -> f64 {
    thread_local! {
        static MEMO: std::cell::RefCell<Memo<(u64, u64), f64>> = std::cell::RefCell::new(Memo::new());
    }
    let key = (x.to_bits(), y.to_bits());
    memoised(&MEMO, key, key.0 ^ key.1.rotate_left(29), || {
        hypot_uncached(x, y)
    })
}

fn atan2_uncached(y: f64, x: f64) -> f64 {
    if !x.is_normal() || !y.is_normal() {
        return libm::atan2(y, x);
    }
    let ((my, ey), (mx, ex)) = (parts(y), parts(x));
    if (ey - ex).abs() > 60 {
        return libm::atan2(y, x);
    }
    let e = ey.max(ex);
    let (fy, fx) = (fix(my, ey - e - 52), fix(mx, ex - e - 52));
    let mut v = if less(&fx, &fy) {
        sub(&over(&constants().pi, 2), &atan_fix(&div(&fx, &fy)))
    } else {
        atan_fix(&div(&fy, &fx))
    };
    if x < 0.0 {
        v = sub(&constants().pi, &v);
    }
    round(y < 0.0, &v, 0).unwrap_or_else(|| libm::atan2(y, x))
}

/// `atan2(y, x)`, correctly rounded for normal `x` and `y` within a factor `2⁶⁰` of each other.
pub fn atan2(y: f64, x: f64) -> f64 {
    thread_local! {
        static MEMO: std::cell::RefCell<Memo<(u64, u64), f64>> = std::cell::RefCell::new(Memo::new());
    }
    let key = (y.to_bits(), x.to_bits());
    memoised(&MEMO, key, key.0 ^ key.1.rotate_left(29), || {
        atan2_uncached(y, x)
    })
}

fn sin_cos_uncached(x: f64) -> (f64, f64) {
    if x.abs() < 1.0 / 134_217_728.0 {
        return (x, 1.0);
    }
    if x.is_nan() || x.abs() >= 1_048_576.0 {
        return (libm::sin(x), libm::cos(x));
    }
    let (q, r) = quadrant(x);
    let (s, c) = sin_cos_series(&r);
    let (s, c) = if q % 2 == 0 { (s, c) } else { (c, s) };
    let sin = round((x < 0.0) != (q >= 2), &s, 0).unwrap_or_else(|| libm::sin(x));
    let cos = round(q == 1 || q == 2, &c, 0).unwrap_or_else(|| libm::cos(x));
    (sin, cos)
}

/// `(sin x, cos x)`, each correctly rounded for `|x| < 2²⁰`.
pub fn sin_cos(x: f64) -> (f64, f64) {
    thread_local! {
        static MEMO: std::cell::RefCell<Memo<u64, (f64, f64)>> = std::cell::RefCell::new(Memo::new());
    }
    let key = x.to_bits();
    memoised(&MEMO, key, key, || sin_cos_uncached(x))
}

/// `sin x`, as [`sin_cos`]'s.
pub fn sin(x: f64) -> f64 {
    sin_cos(x).0
}

/// `cos x`, as [`sin_cos`]'s.
pub fn cos(x: f64) -> f64 {
    sin_cos(x).1
}

/// `∛x`, correctly rounded: the pinned libm's, CORE-MATH's.
pub fn cbrt(x: f64) -> f64 {
    libm::cbrt(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `function x y exact`, as bits: mpmath at 600 bits rounded to the nearest f64 (`log2`'s: Python's `decimal` at 120
    /// digits), for 400 seeded draws a function, every call the curve tilt makes at the `ν` curve's centre, exact results,
    /// ties and reduction points.
    const CASES: &str = include_str!("exact_cases.txt");

    /// The functions, so a control can put another in a slot.
    struct Maths {
        exp: fn(f64) -> f64,
        sin: fn(f64) -> f64,
        hypot: fn(f64, f64) -> f64,
    }

    const EXACT: Maths = Maths { exp, sin, hypot };

    /// Panics naming the cases where `m` is not the exact value's bits.
    fn check(m: &Maths) {
        let mut wrong = Vec::new();
        let mut n = 0;
        for line in CASES.lines() {
            let w: Vec<&str> = line.split(' ').collect();
            let f = |s: &str| f64::from_bits(u64::from_str_radix(s, 16).expect("hex"));
            let (x, y, want) = (f(w[1]), f(w[2]), f(w[3]));
            let got = match w[0] {
                "exp" => (m.exp)(x),
                "exp2" => exp2(x),
                "ln" => ln(x),
                "log2" => log2(x),
                "pow" => pow(x, y),
                "hypot" => (m.hypot)(x, y),
                "atan2" => atan2(x, y),
                "sin" => (m.sin)(x),
                "cos" => cos(x),
                "cbrt" => cbrt(x),
                other => panic!("no function {other}"),
            };
            n += 1;
            if got.to_bits() != want.to_bits() {
                wrong.push(format!("{line}: {:016x}", got.to_bits()));
            }
        }
        assert!(n > 3000, "{n} cases");
        assert!(
            wrong.is_empty(),
            "not correctly rounded, {} of {n}:\n{}",
            wrong.len(),
            wrong[..wrong.len().min(20)].join("\n")
        );
    }

    /// **The criterion**, written before the run: every case is the exact value's bits. The control puts the pinned
    /// libm's `exp` in `exp`'s slot, which is an ulp off at one of the curve tilt's own calls.
    #[test]
    fn exact_maths_correctly_rounded() {
        check(&EXACT);
    }

    validation::negative_control!(
        exact_maths_correctly_rounded,
        "the pinned libm's exp, not correctly rounded, in exp's slot",
        expected = "not correctly rounded",
        check(&Maths {
            exp: libm::exp,
            ..EXACT
        })
    );

    /// Panics unless `get`, a memo's lookup, gives `exp2`'s own bits at every key: 20 000 keys sharing the 4096 slots,
    /// each read twice.
    fn check_memo(mut get: impl FnMut(f64) -> f64) {
        let mut s = 0x2545_F491_4F6C_DD1Du64;
        let keys: Vec<f64> = (0..20_000)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                (s >> 11) as f64 / (1u64 << 53) as f64 * 6.0 - 3.0
            })
            .collect();
        for &x in keys.iter().chain(&keys) {
            assert!(
                get(x).to_bits() == exp2_uncached(x).to_bits(),
                "the memo's value is not the function's at {x}"
            );
        }
    }

    /// **The criterion**: the memo gives the function's own bits. The control's memo reads a slot whatever key it holds.
    #[test]
    fn exact_memo_is_the_function() {
        let mut m = Memo::new();
        check_memo(|x| m.get_or(x.to_bits(), x.to_bits(), || exp2_uncached(x)));
    }

    validation::negative_control!(
        exact_memo_is_the_function,
        "a memo that reads a slot whatever key it holds",
        expected = "the memo's value is not the function's",
        {
            let mut m: Memo<u64, f64> = Memo::new();
            check_memo(|x| {
                let i = (x.to_bits().wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - memo_bits()))
                    as usize;
                match m.slots[i] {
                    Some((_, v)) => v,
                    None => {
                        let v = exp2_uncached(x);
                        m.slots[i] = Some((x.to_bits(), v));
                        v
                    }
                }
            })
        }
    );

    /// Panics unless the edges are as the module's note says.
    fn check_edges(m: &Maths) {
        assert!(
            (m.exp)(f64::NAN).is_nan() && (m.sin)(f64::INFINITY).is_nan(),
            "an edge: NaN"
        );
        for (got, want) in [
            ((m.sin)(-0.0), -0.0),
            ((m.exp)(0.0), 1.0),
            ((m.exp)(1000.0), f64::INFINITY),
            ((m.exp)(-1000.0), 0.0),
            ((m.hypot)(0.0, -0.0), 0.0),
            ((m.hypot)(f64::INFINITY, 1.0), f64::INFINITY),
            (atan2(0.0, -1.0), std::f64::consts::PI),
            (atan2(-0.0, 1.0), -0.0),
            (exp2(-20.0), 1.0 / 1_048_576.0),
            (exp2(2000.0), f64::INFINITY),
            (ln(1.0), 0.0),
            (ln(0.0), f64::NEG_INFINITY),
            (pow(3.0, 0.0), 1.0),
            (pow(0.0, 0.5), 0.0),
            (pow(0.1, 1.0), 0.1),
        ] {
            assert!(
                got.to_bits() == want.to_bits(),
                "an edge: {got:?}, want {want:?}"
            );
        }
    }

    #[test]
    fn exact_maths_edges() {
        check_edges(&EXACT);
    }

    validation::negative_control!(
        exact_maths_edges,
        "a sin that drops a zero's sign",
        expected = "an edge",
        check_edges(&Maths {
            sin: |x| sin(x) + 0.0,
            ..EXACT
        })
    );
}
