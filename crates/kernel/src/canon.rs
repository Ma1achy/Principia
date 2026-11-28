//! Canonicalisation `C`, the one seam to `(m, r, p)` (core_design §5; canonical_spec §3; dd_encode §3.2; inverse-encode
//! contract Parts 1, 2 and 6; dd_decoder §3.3, §3.5; chart_reference §0.4, §0.5): the quotient of a physical state onto
//! the decode's canonical section, CoM at the origin, total momentum zero, `ρ̃` on `+x` and `I = 1`, `λ̃` in either
//! half-plane, one source for the f32 kernel and the f64 CPU path. Decode and encode both run it (seam 8).
//!
//! **The order is normative** ([`canonicalise`]): every step acts on the full state, every `rᵢ` and every `pᵢ`.
//! ```text
//! 1a COM     r ← r − R_com                         R_com = Σ mᵢrᵢ  (M = 1)
//! 0  SCALE   I_in = Σ mᵢ‖rᵢ‖² in the CoM frame;  r ← r/√I_in,  p ← I_in^{1/4}·p;  λ = I_in^{−1/2} recorded;
//!            lookup_rescaled exactly when step 0 ran:
//!            |I_in − 1| ≤ τ_I → skipped (λ = 1, r and p unchanged);  else in two steps, by a power of two, then by
//!            the remainder;  the bodies coincident (I_in = 0) → divisor 1, λ = 1, p = 0
//! 1b BOOST   p ← p − mᵢ·P_tot                      P_tot = Σ pᵢ    (M = 1)
//! 2  ROTATE  φ = atan2(ρ_y, ρ_x), ρ = r₁ − r₀;  R(−φ) on every rᵢ and pᵢ;  ρ = 0 → lookup_clamped, φ = +0
//!            R(−φ)'s cos φ and sin φ formed as ρ/‖ρ‖, the same in exact arithmetic (rotation)
//! ```
//! CoM subtraction precedes the `I` computation because `I` is CoM-frame-defined.
//!
//! **No mirror step** (dd_encode §3.2, dd_decoder §3.3). `C` has four steps, and `λ̃_y` keeps
//! its sign: a state below the `x`-axis stays there, and the encode reads its `β` in `(π, 2π)` (latent `β` is a periodic
//! angle over the full circle). Which side that is, the encode does not read off the rotated floats: `C` decides it on
//! its input and records it ([`Canonical::signs`]; "The signs of `λ̃`" below), and moves no coordinate for it. Every caller, the shape sphere chart's decode among them, runs the one [`canonicalise`],
//! so there is one path with no special case.
//! [`reflect`] is the mirror itself, `Π`, the `y → −y` of every `rᵢ` and every `pᵢ`, which the encode is now
//! equivariant under (T1, inverse-encode contract Part 1).
//!
//! **Precondition.** The masses are positive and `Σmᵢ = 1`, as the decode emits them: `R_com` and `P_tot` are the
//! mass-weighted sums with `M = 1`. The one exception is `κ`'s mass columns (`encode::ingest`; inverse-encode contract
//! Part 4, "The mass columns"), which differentiate these formulas off the simplex, `M` still taken as 1.
//!
//! **The notices.** [`Canonical`] carries `λ` and `lookup_rescaled` (step 0 ran) and `lookup_clamped`, each raised iff
//! its operation did (seam 8). [`Canonical::signs`] is no notice: nothing was discarded.
//!
//! **Step 0** ([`scale_step`]; dd_encode §3.2; inverse-encode contract Parts 2 and 6).
//! Three cases, chosen by selects on its branch inputs (below), every one computed:
//! - **Skipped** where `|I_in − 1| ≤ τ_I`, `τ_I` the register's `tau_i` eps of the precision ([`IeeeBits::TAU_I`]):
//!   `λ = 1`, `r` and `p` leave the step bit for bit as they entered it, and `lookup_rescaled` is not raised.
//!   A canonical input, at `I = 1` only to rounding, raises no notice.
//! - **Run** elsewhere, the bodies apart, in two steps: `I_in = s·2^{4q}` with `q` an integer and `s` in `[1, 16)`
//!   (`I_in`'s mantissa times `2^b`, `b` the low two bits of its exponent), so `√I_in = √s·2^{2q}` and
//!   `I_in^{1/4} = √√s·2^q`: first by the power of two, `r·2^{−2q}` and `p·2^q`, exact (an integer added to an
//!   exponent, or a product by a power of two), then by the remainder, `r/√s` and `√√s·p`, with `√s` in `[1, 4)`.
//!   `λ = 2^{−2q}/√s`. `I_in` itself is never a float: it is carried as a mantissa and an integer exponent (`Wide`,
//!   below), so it neither underflows nor overflows for any finite state with the bodies apart, and the state leaves
//!   step 0 at `I = 1` to rounding. `lookup_rescaled` is raised.
//! - **The bodies coincident**, `r₀ = r₁ = r₂`: divisor 1, `λ = 1`, `p = 0`, `r` the CoM frame's, no
//!   `lookup_rescaled`; `ρ = 0` raises `lookup_clamped` and `φ = +0`. With positive masses `I_in = 0`
//!   happens there only. The select is on the coincidence itself, not on `I_in = 0`: `R_com` of three equal positions
//!   off the origin is rounded, so their CoM frame can be an ulp from 0 and `I_in` an ulp's square, not 0. The plan is
//!   unclear here (it names the case `I_in = 0`); chose the coincidence because that `I_in` need not round to 0.
//!
//! The recorded `I_in` ([`Canonical::i_in`]) is the input's, converted to the float: where it is past the float's
//! range, at f32 coordinates above about `1e19` or below about `4e-23` (f64 above about `1e154`, below about
//! `1e-162`), it reads `+∞` or a subnormal or 0, a record only; nothing in `C` reads it. `λ` is past the float's range
//! only where `I_in^{−1/2}` itself is, the bodies within about `1e-38` of each other at f32.
//!
//! **`φ` at the pole** (dd_decoder §3.2, §3.3; [`rotation_angle`]): at `ρ = 0`, `φ` is 0 for every sign of the
//! zeros, a select on `ρ_x = 0` and `ρ_y = 0`, never `atan2(0, 0)`, which GLSL.std.450 leaves undefined; `R(−φ)` is then
//! the identity on CPU-f64 and GPU-f32 alike. Only the exact `ρ = 0` reaches `lookup_clamped` here; a near-zero `ρ` is
//! the validation ladder's.
//!
//! **The branch inputs** (gpu_determinism_note § "The discipline", rules 2 and 6). Five branches must
//! agree across backends: step 0's skip, its coincidence select, step 2's guard `ρ = 0`, where `ρ_x` or `ρ_y` is
//! 0 in step 2's turn and angle, and the signs of `λ̃_x` and `λ̃_y` that choose the encode's case of `β`
//! (dd_encode §3.2, "The signed encode of `β`"). Each is built only from operations every IEEE backend rounds
//! alike, with no intermediate that can be subnormal. `√`, `/` and `atan2` carry ulp bounds in WGSL, not correct
//! rounding; `fma` is not fused on every backend (CI's lavapipe evaluates it as a rounded product and an add),
//! nor is `a·b + c` left uncontracted on every one; and a backend that flushes subnormal results (Metal) rounds a
//! subnormal intermediate differently from one that keeps it (the CPU), fused or not. So none of them is built from
//! those. Every quantity a branch reads is carried as a `Wide`, a mantissa in `[1, 2)` and an integer exponent, its
//! operations ([`IeeeBits::product`]'s pattern):
//! - **Into the form** (`wide`): an input coordinate, a mass, `τ_I` or 1 is scaled by `2^−k`, `k` its exponent
//!   field, as two normal powers of two, exactly, into `[1, 2)` (for a normal input; a subnormal one, which a flushing
//!   GPU reads as 0 before `C` runs, is taken in exactly on a backend that keeps it).
//! - **Products** (`wide_mul`): the mantissas' [`IeeeBits::product`], in `[1, 4)`: at f32 each operand split at its bits
//!   into parts of at most 12 significant bits, so the four partial products are exact and at least `2^−46`, summed in
//!   one fixed order; a backend that fuses or contracts a site rounds the same exact sum. The exponents add, as
//!   integers.
//! - **Sums and differences** (`wide_add`, `wide_sub`): aligned on the larger exponent, the smaller mantissa scaled by
//!   `2^−s`, `s` the exponents' gap capped at 60, below which it is under half an ulp of the larger, so the cap changes
//!   no rounding. That scaling is a product by a power of two, exact and at least `2^−60`, so a fused or contracted
//!   `big + small·2^−s` rounds the same. The sum is rounded once; its operands are multiples of `2^−83` (f32;
//!   `2^−112` at f64), so it is 0 or at least that, normal.
//! - **Renormalising** (`renorm`): a power-of-two scaling back into `[1, 2)`, exact, the exponent read from the bits.
//! - **Comparing** (`wide_greater`, `is_zero`): by exponent, then mantissa; integer operations on bits and exponents,
//!   selects and comparisons.
//!
//! So no float a branch input rounds is subnormal or overflows, for any state whose coordinates are 0 or normal, its
//! pair differences, CoM and `I_in` whatever their size: one input takes one branch on every IEEE backend that keeps
//! its additions and multiplications correctly rounded and unreassociated (fast-math off), whether it fuses,
//! contracts or flushes subnormals. Each branch input, in exact arithmetic the normative order's step:
//! - **The CoM and `I_in`** (`wide_com`, `wide_inertia`). `R_com = (m₀x₀ + m₁x₁) + m₂x₂` per axis, each product a `Wide`
//!   product and each sum a `Wide` sum; the CoM frame `xᵢ − R_com`, a `Wide` difference; `I_in = (m₀‖c₀‖² + m₁‖c₁‖²) +
//!   m₂‖c₂‖²`, `‖c‖² = c_x² + c_y²`, the same. [`centre_of_mass`] and [`inertia`] are these, converted to the float.
//! - **The skip** (`skipped`): `|I_in − 1| > τ_I` compared as `Wide`s; `I_in − 1` is a `Wide` difference.
//! - **The coincidence and the guard.** `ρ = r₁ − r₀` and `d02 = r₂ − r₀`, `Wide` differences of the input's
//!   coordinates: `r₁ − r₀` is 0 iff `r₁ = r₀`, each component a correctly rounded difference that is exact where it is
//!   small, never flushed. The guard is `ρ = 0`; the coincidence is `ρ = 0` and `d02 = 0`. `ρ` is the same in every
//!   frame, and 0 iff the scaled `ρ` is.
//! - **The turn and the angle** ([`rotation`], [`rotation_angle`]). From the `Wide` `ρ`, each component is scaled by
//!   `2^−e`, `e` the larger component's exponent, its gap to the smaller capped at 60, so the larger lies in `[1, 2)`
//!   and the smaller is 0 or at least `2^−60`: the zero tests read exact zeros, never a flushed one, and the squares
//!   are normal. In exact arithmetic `atan2` and `ρ/‖ρ‖` are unchanged by the positive factor; the cap moves the smaller
//!   component, where it is more than `2^60` below the larger, by under `2^−59` of `‖ρ‖`, below an ulp of the turn.
//!   The turn's and the angle's values are WGSL's `√`, `/` and `atan2`, read, not branched on.
//! - **The signs of `λ̃`** ([`signs_input`], [`signs_of`]; [`Canonical::signs`]). The rotated `λ̃_y` of a collinear
//!   state is a rounding residue: its sign turns on whether a backend fuses step 2's `fma`s and on whether it flushes
//!   a subnormal, and the encode's two ends there, `β = 0` and `β = 2π̂`, are one point of the circle but two values.
//!   So the signs are decided on `C`'s input, from the `Wide` pair differences `ρ`, `d02 = r₂ − r₀` and
//!   `d12 = r₂ − r₁` and the `Wide` masses: `Λ = m₀·d02 + m₁·d12` (`= M₀₁·λ` in exact arithmetic, `M₀₁ > 0`),
//!   `ρ × Λ = ρ_xΛ_y − ρ_yΛ_x`, whose sign is the rotated `λ̃_y`'s, and `ρ · Λ = ρ_xΛ_x + ρ_yΛ_y`, whose sign is
//!   the rotated `λ̃_x`'s (`R(−φ)` takes `ρ` to `+x`, and the weights `√μ_λ`, `1/M₀₁` and `1/‖ρ‖` are positive), with
//!   `ρ` read as `(1, 0)` at the guard, where step 2 is the identity. Each is `< 0`, `= 0` or `> 0` by a read of its
//!   mantissa. The decision is the same on every backend, not exact: the `Wide` products and sums round, so a state
//!   collinear only to its coordinates' rounding gets a cross product of either sign, the same one everywhere, and
//!   a state whose `Wide` terms are exact gets its exact sign and its exact zero. `C` moves no coordinate for it (no
//!   mirror): only `crate::encode::blocks::beta_at`'s selects read it, and they give `atan2` operands of the decided
//!   signs. Don't compare the rotated floats `λ̃_x`, `λ̃_y` ([`lambda_t_axis`]: an `fma`, a division and a `√`)
//!   with 0 for `β`: their signs at a collinear state can differ between backends.
//!
//! The cross-backend claim is for identical inputs only (PIT-10). A state with a subnormal coordinate is itself read
//! differently by a flushing GPU, before `C` runs; on a backend that keeps subnormals, the CPU's, such a coordinate is
//! taken into the `Wide` form exactly too.
//!
//! **The scale gauge** ([`scale_gauge`]; dd_decoder §3.5, chart_reference §0.5): `ℓ = √I`, `rᵢ ← rᵢ/ℓ`,
//! `pᵢ ← √ℓ·pᵢ` ([`similarity`]), at every `I`: dd_decoder §3.5's map, a no-op, to rounding, on canonical output. Step
//! 0 is the same map in exact arithmetic, skipped within `τ_I` and taken in two steps elsewhere.

use spirv_std::num_traits::Float;

use crate::decode::Ic;
use crate::generated::links::LinkReal;
use crate::real::IeeeBits;
use crate::Real;

/// A state on the canonical section, and what `C` did to reach it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canonical<R> {
    /// The canonical `(m, r, p)`.
    pub ic: Ic<R>,
    /// `I_in`, the input's moment of inertia in its CoM frame, computed after step 1a, converted to the float (the
    /// module's note: `+∞`, a subnormal or 0 where it is past the float's range).
    pub i_in: R,
    /// `λ = I_in^{−1/2}` where step 0 ran, 1 where it was skipped or the bodies coincide: `lookup_rescaled`'s value.
    pub lambda: R,
    /// Step 2's rotation angle `φ`, the rotation applied being `R(−φ)`.
    pub phi: R,
    /// `λ̃_y = √μ_λ·λ_y` after the rotation ([`lambda_t_y`]): recorded, not branched on. Its sign is a backend's own
    /// rounding at a collinear state; the half the encode reads is [`Canonical::signs`].
    pub lambda_t_y: R,
    /// `lookup_rescaled`: step 0 ran, `|I_in − 1| > τ_I` with the bodies apart.
    pub rescaled: bool,
    /// `lookup_clamped`: `ρ = 0` exactly at step 2, read before step 0's division.
    pub clamped: bool,
    /// The signs of the rotated `λ̃_x` and `λ̃_y`, decided on the input ([`signs_of`]; the module's note): the case the
    /// encode's `β` takes. The state is not moved for it.
    pub signs: LambdaSigns,
}

/// What steps 1a and 0 give ([`scale_step`]): 1a's CoM-frame positions, step 0's output and its record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scaled<R> {
    /// Step 1a's output, `rᵢ − R_com`, step 0's input.
    pub centred: [[R; 2]; 3],
    /// Step 0's positions: `centred` where it was skipped or the bodies coincide, else at `I = 1`.
    pub r: [[R; 2]; 3],
    /// Step 0's momenta: the input's where it was skipped, 0 where the bodies coincide, else `I_in^{1/4}·p`.
    pub p: [[R; 2]; 3],
    /// `I_in`, as [`Canonical::i_in`].
    pub i_in: R,
    /// `λ`, as [`Canonical::lambda`].
    pub lambda: R,
    /// Step 0 ran: `lookup_rescaled`.
    pub rescaled: bool,
}

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// The exponent [`Wide`] gives zero: a sentinel meaning "no value", not a tuned number. Any value below the least
/// exponent `Wide` can produce serves (its longest product has nine factors, so about `−9700` at f64's least subnormal
/// exponent): a zero is then the smallest value in every comparison and alignment. `i16`'s bound is a choice of
/// meaning, a register-free sentinel (dd_generation_root §3.8, "Reading a constant"), as `decode_dispatch`'s
/// `UNWRITTEN = u32::MAX` is. No arithmetic on it overflows `i32`: a product's exponent sum is formed from two
/// operands, each a zero's sentinel or a real exponent, and a zero result is set back to the sentinel, so the least sum
/// formed is `2·i16::MIN − 1023` (two zeros, renormalised with f64's zero field); the alignment gap is at most about
/// `9216 − i16::MIN`; step 0's exponents, `I_in`'s quarter and its doubles, are within `±2^16`; all are within
/// `±10^5`.
const ZERO_EXPONENT: i32 = i16::MIN as i32;

/// A number `m·2^e` with its exponent carried as an integer: `|m|` in `[1, 2)`, or `m = 0` with `e` the
/// `ZERO_EXPONENT`. The arithmetic of `C`'s branch inputs (the module's note), so that no float it rounds lies outside
/// `[2^−83, 4)` in magnitude but 0 (f32; `[2^−112, 4)` at f64), and none is subnormal on any backend, whatever the
/// state's scale or its terms' spread.
#[derive(Clone, Copy)]
pub struct Wide<R> {
    m: R,
    e: i32,
}

impl Wide<f32> {
    /// The mantissa's bits and the exponent, as words: both, since a `Wide`'s value is the pair.
    #[inline]
    pub fn words(self) -> [u32; 2] {
        [self.m.to_bits(), self.e as u32]
    }
}

/// `x·2^k` normalised, for `x` 0 or a normal float within `[2^−83, 4)` in magnitude (f32; `[2^−112, 4)` at f64): its
/// exponent read from the bits, `x` scaled by its inverse, exact.
#[inline]
fn renorm<R: LinkReal + IeeeBits>(x: R, k: i32) -> Wide<R> {
    let n = x.exponent();
    Wide {
        m: x * R::pow2(-n),
        e: select(x == R::zero(), ZERO_EXPONENT, k + n),
    }
}

/// `x` as a [`Wide`], for `x` finite: scaled by `2^−k`, `k` its exponent field, as two normal powers of two, exactly,
/// into `[1, 2)` where `x` is normal and into `[2^−22, 2)` (f32; `[2^−51, 2)` at f64) where it is subnormal, then
/// renormalised, so a subnormal `x` (on a backend that keeps it) gets its own mantissa and exponent too.
#[inline]
pub(crate) fn wide<R: LinkReal + IeeeBits>(x: R) -> Wide<R> {
    let k = x.exponent();
    let half = k / 2;
    renorm(x * R::pow2(-half) * R::pow2(half - k), k)
}

/// `a·b`: the mantissas' [`IeeeBits::product`], in `[1, 4)`, whose partial products are at least `2^−46`, renormalised.
#[inline]
pub(crate) fn wide_mul<R: LinkReal + IeeeBits>(a: Wide<R>, b: Wide<R>) -> Wide<R> {
    renorm(a.m.product(b.m), a.e + b.e)
}

/// `a + b`, aligned on the larger exponent: the smaller mantissa times `2^−s`, `s` the exponents' difference capped at
/// 60, below which it is under half an ulp of the larger (so the cap changes no rounding), then renormalised. A
/// nonzero sum is at least `2^−83` (f32; `2^−112` at f64), the operands being multiples of it: normal.
#[inline]
pub(crate) fn wide_add<R: LinkReal + IeeeBits>(a: Wide<R>, b: Wide<R>) -> Wide<R> {
    let a_big = a.e >= b.e;
    let (big, small) = (select(a_big, a, b), select(a_big, b, a));
    let gap = big.e - small.e;
    let shift = select(gap > 60, 60, gap);
    renorm(big.m + small.m * R::pow2(-shift), big.e)
}

/// `−a`.
#[inline]
fn wide_neg<R: LinkReal + IeeeBits>(a: Wide<R>) -> Wide<R> {
    Wide { m: -a.m, e: a.e }
}

/// `a − b`, one rounding: [`wide_add`] of `−b`.
#[inline]
pub(crate) fn wide_sub<R: LinkReal + IeeeBits>(a: Wide<R>, b: Wide<R>) -> Wide<R> {
    wide_add(a, wide_neg(b))
}

/// `|a|`.
#[inline]
fn wide_abs<R: LinkReal + IeeeBits>(a: Wide<R>) -> Wide<R> {
    Wide {
        m: Float::abs(a.m),
        e: a.e,
    }
}

/// `a > b` for `a, b ≥ 0`: by exponent, then by mantissa; a zero, at the [`ZERO_EXPONENT`], is below every other value.
#[inline]
pub(crate) fn wide_greater<R: LinkReal + IeeeBits>(a: Wide<R>, b: Wide<R>) -> bool {
    (a.e > b.e) | ((a.e == b.e) & (a.m > b.m))
}

/// `a = 0`, on the mantissa, which a `Wide` holds 0 only for zero.
#[inline]
fn is_zero<R: LinkReal + IeeeBits>(a: Wide<R>) -> bool {
    a.m == R::zero()
}

/// `‖v‖² = v_x² + v_y²`.
#[inline]
pub(crate) fn wide_norm2<R: LinkReal + IeeeBits>(v: [Wide<R>; 2]) -> Wide<R> {
    wide_add(wide_mul(v[0], v[0]), wide_mul(v[1], v[1]))
}

/// `v`'s components as [`Wide`]s.
#[inline]
pub(crate) fn vector<R: LinkReal + IeeeBits>(v: [R; 2]) -> [Wide<R>; 2] {
    [wide(v[0]), wide(v[1])]
}

/// `b − a`, per component.
#[inline]
pub(crate) fn difference<R: LinkReal + IeeeBits>(a: [Wide<R>; 2], b: [Wide<R>; 2]) -> [Wide<R>; 2] {
    [wide_sub(b[0], a[0]), wide_sub(b[1], a[1])]
}

/// `R`'s least and greatest normal exponents, read from the bits of its least positive normal and greatest finite
/// values.
#[inline]
fn exponent_range<R: LinkReal + IeeeBits>() -> (i32, i32) {
    (
        <R as Float>::min_positive_value().exponent(),
        <R as Float>::max_value().exponent(),
    )
}

/// `x·2^e` for any integer `e`, by two powers of two, each clamped to `R`'s normal exponents: for `x` in `[1, 2)`, exact
/// where the result is normal, rounded once where it is subnormal (0 on a backend that flushes it), 0 below, and past
/// `R`'s range where `2^e` is. A value, not a branch input.
#[inline]
fn ldexp<R: LinkReal + IeeeBits>(x: R, e: i32) -> R {
    let (lo, hi) = exponent_range::<R>();
    let clamp = |k: i32| select(k < lo, lo, select(k > hi, hi, k));
    let first = clamp(e);
    x * R::pow2(first) * R::pow2(clamp(e - first))
}

/// `a` as a float: [`ldexp`] of its mantissa and exponent.
#[inline]
pub(crate) fn to_float<R: LinkReal + IeeeBits>(a: Wide<R>) -> R {
    ldexp(a.m, a.e)
}

/// `a` as a float where its exponent is in `R`'s normal range, where [`ldexp`] is exact, and 0 below it: decided on the
/// exponent, before anything rounds, so no subnormal is formed for a branch to read on any backend
/// (gpu_determinism_note § "The discipline", rule 6). A `Wide` just below the range whose float would round up to the
/// least normal reads 0 like the rest; a backend's flushing or keeping never reaches it.
#[inline]
pub(crate) fn to_normal_float<R: LinkReal + IeeeBits>(a: Wide<R>) -> R {
    if a.e < exponent_range::<R>().0 {
        R::zero()
    } else {
        to_float(a)
    }
}

/// `Σ mᵢxᵢ` on one axis, `(m₀x₀ + m₁x₁) + m₂x₂`, as [`Wide`]s.
#[inline]
fn wide_weighted<R: LinkReal + IeeeBits>(mass: [Wide<R>; 3], x: [Wide<R>; 3]) -> Wide<R> {
    wide_add(
        wide_add(wide_mul(mass[0], x[0]), wide_mul(mass[1], x[1])),
        wide_mul(mass[2], x[2]),
    )
}

/// Step 1a's `R_com = Σ mᵢrᵢ` (`M` taken as 1), as [`Wide`]s.
#[inline]
fn wide_com<R: LinkReal + IeeeBits>(mass: [Wide<R>; 3], x: [[Wide<R>; 2]; 3]) -> [Wide<R>; 2] {
    [
        wide_weighted(mass, [x[0][0], x[1][0], x[2][0]]),
        wide_weighted(mass, [x[0][1], x[1][1], x[2][1]]),
    ]
}

/// `Σ mᵢ‖xᵢ‖²` about the origin, `(m₀‖x₀‖² + m₁‖x₁‖²) + m₂‖x₂‖²`, as [`Wide`]s.
#[inline]
fn wide_inertia<R: LinkReal + IeeeBits>(mass: [Wide<R>; 3], x: [[Wide<R>; 2]; 3]) -> Wide<R> {
    wide_weighted(mass, [wide_norm2(x[0]), wide_norm2(x[1]), wide_norm2(x[2])])
}

/// The masses and the coordinates of `r` as [`Wide`]s.
#[inline]
pub fn wide_state<R: LinkReal + IeeeBits>(
    m: [R; 3],
    r: [[R; 2]; 3],
) -> ([Wide<R>; 3], [[Wide<R>; 2]; 3]) {
    (
        [wide(m[0]), wide(m[1]), wide(m[2])],
        [vector(r[0]), vector(r[1]), vector(r[2])],
    )
}

/// The centre of mass `Σ mᵢrᵢ` of `r` under the masses `m`, `Σmᵢ = 1`: step 1a's, formed as `C` forms it (the module's
/// note), converted to the float.
#[inline]
pub fn centre_of_mass<R: LinkReal + IeeeBits>(m: [R; 3], r: [[R; 2]; 3]) -> [R; 2] {
    let (mass, x) = wide_state(m, r);
    let com = wide_com(mass, x);
    [to_float(com[0]), to_float(com[1])]
}

/// The moment of inertia `Σ mᵢ‖rᵢ‖²` of `r` about the origin, formed as step 0 forms `I_in` (the module's note),
/// converted to the float.
#[inline]
pub fn inertia<R: LinkReal + IeeeBits>(m: [R; 3], r: [[R; 2]; 3]) -> R {
    let (mass, x) = wide_state(m, r);
    to_float(wide_inertia(mass, x))
}

/// Step 0's skip test's two sides, the branch input: `|I_in − 1|` and `τ_I`, as [`Wide`]s.
#[inline]
pub fn skip_input<R: LinkReal + IeeeBits>(i: Wide<R>) -> [Wide<R>; 2] {
    [wide_abs(wide_sub(i, wide(R::one()))), wide(R::TAU_I)]
}

/// Step 0's skip decision on [`skip_input`]'s sides: `|I_in − 1| ≤ τ_I`.
#[inline]
pub fn skip_taken<R: LinkReal + IeeeBits>(sides: [Wide<R>; 2]) -> bool {
    !wide_greater(sides[0], sides[1])
}

/// Whether step 0 runs: `lookup_rescaled`, neither skipped nor the bodies coincident.
#[inline]
pub fn step_runs(skip: bool, coincident: bool) -> bool {
    !(skip | coincident)
}

/// `v·k`.
#[inline]
fn times<R: LinkReal>(v: [R; 2], k: R) -> [R; 2] {
    [v[0] * k, v[1] * k]
}

/// `v/k`.
#[inline]
fn over<R: LinkReal>(v: [R; 2], k: R) -> [R; 2] {
    [v[0] / k, v[1] / k]
}

/// The similarity map at the moment of inertia `i` (dd_decoder §3.5, the scale gauge's): `rᵢ/√i`, `i^{1/4}·pᵢ`, and
/// `λ = 1/√i`, at every `i`; at `i = 0` the divisor is selected as 1, so no `1/0` is formed. Step 0 is this map in exact
/// arithmetic, in its own form ([`scale_step`]).
#[inline]
pub fn similarity<R: LinkReal>(
    r: [[R; 2]; 3],
    p: [[R; 2]; 3],
    i: R,
) -> ([[R; 2]; 3], [[R; 2]; 3], R) {
    let root = Float::sqrt(i);
    let quarter = Float::sqrt(root);
    let d = select(i == R::zero(), R::one(), root);
    (
        [over(r[0], d), over(r[1], d), over(r[2], d)],
        [
            times(p[0], quarter),
            times(p[1], quarter),
            times(p[2], quarter),
        ],
        R::one() / d,
    )
}

/// One CoM-frame coordinate `c` after step 0: `c·2^{−2q}/root`, the power of two on the exponent, exact, the remainder
/// on the mantissa, converted to the float. `root` is 1 and `q` 0 where step 0 does not run, which returns `c` itself.
#[inline]
fn scaled_coordinate<R: LinkReal + IeeeBits>(c: Wide<R>, root: R, q: i32) -> R {
    to_float(renorm(c.m / root, c.e - 2 * q))
}

/// Step 1a's CoM-frame coordinates of the [`Wide`] state `(mass, x)`, and step 0's `I_in` in that frame.
#[inline]
pub fn com_frame<R: LinkReal + IeeeBits>(
    mass: [Wide<R>; 3],
    x: [[Wide<R>; 2]; 3],
) -> ([[Wide<R>; 2]; 3], Wide<R>) {
    let com = wide_com(mass, x);
    let c = [
        [wide_sub(x[0][0], com[0]), wide_sub(x[0][1], com[1])],
        [wide_sub(x[1][0], com[0]), wide_sub(x[1][1], com[1])],
        [wide_sub(x[2][0], com[0]), wide_sub(x[2][1], com[1])],
    ];
    (c, wide_inertia(mass, c))
}

/// Steps 1a and 0 on the [`Wide`] state `(mass, x)` and the momenta `p` (the module's note); `coincident` is the
/// bodies' coincidence, `ρ = 0` and `d02 = 0`.
#[inline]
fn scale_wide<R: LinkReal + IeeeBits>(
    mass: [Wide<R>; 3],
    x: [[Wide<R>; 2]; 3],
    p: [[R; 2]; 3],
    coincident: bool,
) -> Scaled<R> {
    // 1a: the CoM frame, and I_in in it.
    let (c, i) = com_frame(mass, x);
    // 0: the skip, and the two steps, `I_in = s·2^{4q}`, `s` in [1, 16).
    let run = step_runs(skip_taken(skip_input(i)), coincident);
    let q = select(run, i.e >> 2, 0);
    let s = i.m * R::pow2(i.e & 3);
    let root = select(run, Float::sqrt(s), R::one());
    let quarter = Float::sqrt(root);
    let coordinate = |v: [Wide<R>; 2]| {
        [
            scaled_coordinate(v[0], root, q),
            scaled_coordinate(v[1], root, q),
        ]
    };
    let momentum = |v: [R; 2]| {
        [
            select(coincident, R::zero(), ldexp(v[0], q) * quarter),
            select(coincident, R::zero(), ldexp(v[1], q) * quarter),
        ]
    };
    let lambda = to_float(renorm(R::one() / root, -2 * q));
    Scaled {
        centred: [
            [to_float(c[0][0]), to_float(c[0][1])],
            [to_float(c[1][0]), to_float(c[1][1])],
            [to_float(c[2][0]), to_float(c[2][1])],
        ],
        r: [coordinate(c[0]), coordinate(c[1]), coordinate(c[2])],
        p: [momentum(p[0]), momentum(p[1]), momentum(p[2])],
        i_in: to_float(i),
        lambda,
        rescaled: run,
    }
}

/// Steps 1a and 0 of `C` on `ic` (the module's note): the CoM frame, then `I = 1` in two steps, skipped where
/// `|I_in − 1| ≤ τ_I`, and the coincident bodies' select. `ic.m` is positive with `Σmᵢ = 1`.
#[inline]
pub fn scale_step<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> Scaled<R> {
    let (mass, x) = wide_state(ic.m, ic.r);
    let coincident = is_pole(difference(x[0], x[1])) & is_pole(difference(x[0], x[2]));
    scale_wide(mass, x, ic.p, coincident)
}

/// Both components 0: `ρ = 0`, step 2's guard (`lookup_clamped`), true for `+0` and `−0` alike.
#[inline]
pub fn is_pole<R: LinkReal + IeeeBits>(v: [Wide<R>; 2]) -> bool {
    is_zero(v[0]) & is_zero(v[1])
}

/// `ρ` scaled for step 2 (the module's note): each component times `2^−e`, `e` the larger one's exponent, the gap to the
/// smaller capped at 60, so the larger is in `[1, 2)` and the smaller 0 or at least `2^−60` in magnitude; and whether
/// `ρ = 0`, the pole. At the pole both are `±0`.
#[inline]
pub fn scaled_rho<R: LinkReal + IeeeBits>(rho: [Wide<R>; 2]) -> ([R; 2], bool) {
    let e = select(rho[0].e > rho[1].e, rho[0].e, rho[1].e);
    let gap = |k: i32| {
        let g = e - k;
        select(g > 60, 60, g)
    };
    (
        [
            rho[0].m * R::pow2(-gap(rho[0].e)),
            rho[1].m * R::pow2(-gap(rho[1].e)),
        ],
        is_pole(rho),
    )
}

/// A scaled `ρ` component is 0, the zero test step 2 reads an exact zero with ([`scaled_rho`]).
#[inline]
pub fn axis_zero<R: LinkReal>(x: R) -> bool {
    x == R::zero()
}

/// Step 2's angle from the scaled `ρ` (`x`, `y`) and its pole: `atan2(y, x)`, a zero `x` taken as `+0`, and `+0` at the
/// pole, each a select ([`rotation_angle`]).
#[inline]
fn angle_of<R: LinkReal>(v: [R; 2], pole: bool) -> R {
    let x = select(axis_zero(v[0]), R::zero(), v[0]);
    select(pole, R::zero(), Real::arctangent2(v[1], x))
}

/// Step 2's turn from the scaled `ρ` and its pole: `ρ̂ = ρ/‖ρ‖`, `(1, 0)` at the pole ([`rotation`]).
#[inline]
fn turn_of<R: LinkReal>(v: [R; 2], pole: bool) -> (R, R) {
    let n2 = Float::mul_add(v[0], v[0], v[1] * v[1]);
    let k = R::one() / Float::sqrt(select(pole, R::one(), n2));
    (
        select(pole, R::one(), v[0] * k),
        select(pole, R::zero(), v[1] * k),
    )
}

/// Step 2's angle `φ = atan2(ρ_y, ρ_x)`, and 0 at `ρ = 0` for every sign of the zeros, selected. `ρ` is taken
/// into the `Wide` form and scaled as `C` scales it (the module's note), so the zero tests read exact zeros; `atan2` is
/// unchanged by the positive factor. A zero `ρ_x` is taken as `+0` before `atan2`, by a select on it against 0: Metal's
/// `atan2` at its default math mode gives `atan2(ρ_y, −0)` the opposite sign of `π/2` to IEEE's (pitfalls §11). Where
/// `ρ_y ≠ 0` IEEE's `atan2(ρ_y, ±0)` is one value, and where `ρ_y = 0` the pole's select discards it.
#[inline]
pub fn rotation_angle<R: LinkReal + IeeeBits>(rho: [R; 2]) -> R {
    let (v, pole) = scaled_rho(vector(rho));
    angle_of(v, pole)
}

/// Step 2's turn `(c, s) = (cos φ, sin φ)` for `φ` = [`rotation_angle`]`(ρ)`, formed as `ρ̂ = ρ/‖ρ‖`, not through `φ`;
/// at `ρ = 0`, `(1, 0)`, so `R(−φ)` is the identity there. In exact arithmetic `ρ̂` is `(cos φ, sin φ)`. In
/// rounded arithmetic `sin` and `cos` of a rounded `atan2` turn `ρ` off `+x` by their error in `φ`, which WGSL bounds
/// only loosely (`atan2` to 4096 ulp, `sin` and `cos` to an absolute `2⁻¹¹`) and `R(−φ)` multiplies by each `‖rᵢ‖`; a
/// light body is far out at `I = 1` (`‖r₁‖ ≈ 27` at `m₁ ≈ 1.2·10⁻³`), and through the sines CI's lavapipe put that
/// body's `r_y` 273.88 eps from the f64 decode where the native f32 decode is 1.12 (PR #262). `ρ̂`'s direction is
/// rounded only by the two products `ρ_x·k` and `ρ_y·k`, half an ulp each, the error of `k = 1/√(ρ_x² + ρ_y²)`
/// scaling both alike. `ρ` is first scaled as `C` scales it (the module's note): its larger component into `[1, 2)`,
/// exactly, and the smaller to 0 or at least `2^−60`, so `ρ_x² + ρ_y²` neither overflows nor underflows for a finite
/// `ρ`, and the pole is read from the exact zeros.
#[inline]
pub fn rotation<R: LinkReal + IeeeBits>(rho: [R; 2]) -> (R, R) {
    let (v, pole) = scaled_rho(vector(rho));
    turn_of(v, pole)
}

/// `R(−φ)·v` with `c = cos φ`, `s = sin φ`: `(c·v_x + s·v_y, c·v_y − s·v_x)`, each an explicit `fma`. A value, not a
/// branch input.
#[inline]
fn rotate<R: LinkReal>(v: [R; 2], c: R, s: R) -> [R; 2] {
    [
        Float::mul_add(c, v[0], s * v[1]),
        Float::mul_add(c, v[1], -(s * v[0])),
    ]
}

/// `λ̃_y = √μ_λ·λ_y` of `r` under `m`, `μ_λ = m₂M₀₁`, `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`, `M₀₁` the one rounding of
/// `m₀ + m₁` (`fma(m₁, 1, m₀)`, as the mass block forms it), read on `C`'s rotated output and recorded in
/// [`Canonical::lambda_t_y`]. `C` does not branch on it.
#[inline]
pub fn lambda_t_y<R: LinkReal>(m: [R; 3], r: [[R; 2]; 3]) -> R {
    lambda_t_axis(m, [r[0][1], r[1][1], r[2][1]])
}

/// One component of `λ̃ = √μ_λ·λ` under `m`, from the three bodies' coordinates `x` on that axis:
/// `√(m₂M₀₁)·(x₂ − fma(m₁, x₁, m₀x₀)/M₀₁)`, `M₀₁ = fma(m₁, 1, m₀)`. The one place this arithmetic is written:
/// [`lambda_t_y`] reads it on `y`, and the encode's Jacobi map (`crate::encode::blocks::jacobi`) on both axes, so the
/// recorded `λ̃_y` and `β`'s operands are the same bits. A value: no case is chosen on its sign ([`signs_of`]).
#[inline]
pub fn lambda_t_axis<R: LinkReal>(m: [R; 3], x: [R; 3]) -> R {
    let m01 = Float::mul_add(m[1], R::one(), m[0]);
    let inner = Float::mul_add(m[1], x[1], m[0] * x[0]) / m01;
    Float::sqrt(m[2] * m01) * (x[2] - inner)
}

/// A plane vector in `Wide`, `x` then `y`.
pub type WideVec<R> = [Wide<R>; 2];

/// The pair differences `ρ = r₁ − r₀`, `d02 = r₂ − r₀` and `d12 = r₂ − r₁` of the [`Wide`] coordinates `x`.
#[inline]
pub fn pair_differences<R: LinkReal + IeeeBits>(
    x: [[Wide<R>; 2]; 3],
) -> (WideVec<R>, WideVec<R>, WideVec<R>) {
    (
        difference(x[0], x[1]),
        difference(x[0], x[2]),
        difference(x[1], x[2]),
    )
}

/// The signs of the rotated `λ̃ = (λ̃_x, λ̃_y)`, each below 0, 0 or above 0 (neither flag): the case of the encode's
/// `β` (dd_encode §3.2, "The signed encode of `β`"; `crate::encode::blocks::beta_at`). [`signs_of`] decides them on
/// `C`'s input (the module's note); [`LambdaSigns::of_floats`] reads them off a given `λ̃`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LambdaSigns {
    /// `λ̃_y < 0`: the lower half, `β` in `(π, 2π]`.
    pub y_negative: bool,
    /// `λ̃_y = 0`: on the `x`-axis, `β` exactly 0 or `π̂`.
    pub y_zero: bool,
    /// `λ̃_x < 0`.
    pub x_negative: bool,
    /// `λ̃_x = 0`.
    pub x_zero: bool,
}

impl LambdaSigns {
    /// The signs of a given `λ̃`, each component compared with 0, which `+0` and `−0` pass alike: exact on the operands
    /// as given, for a caller that holds `λ̃` and no state `C` saw.
    #[inline]
    pub fn of_floats<R: LinkReal>(lambda_t: [R; 2]) -> LambdaSigns {
        let zero = R::zero();
        LambdaSigns {
            y_negative: lambda_t[1] < zero,
            y_zero: lambda_t[1] == zero,
            x_negative: lambda_t[0] < zero,
            x_zero: lambda_t[0] == zero,
        }
    }

    /// The signs as a word: bit 0 `λ̃_y < 0`, bit 1 `λ̃_y = 0`, bit 2 `λ̃_x < 0`, bit 3 `λ̃_x = 0`.
    #[inline]
    pub fn word(self) -> u32 {
        select(self.y_negative, 1, 0)
            | select(self.y_zero, 2, 0)
            | select(self.x_negative, 4, 0)
            | select(self.x_zero, 8, 0)
    }

    /// The signs [`LambdaSigns::word`] wrote in `word`'s low four bits.
    #[inline]
    pub fn of_word(word: u32) -> LambdaSigns {
        LambdaSigns {
            y_negative: word & 1 != 0,
            y_zero: word & 2 != 0,
            x_negative: word & 4 != 0,
            x_zero: word & 8 != 0,
        }
    }
}

/// The branch input of `λ̃`'s signs ([`signs_of`]): `ρ × Λ` and `ρ · Λ`, `Λ = m₀·d02 + m₁·d12`.
#[derive(Clone, Copy)]
pub struct SignsInput<R> {
    /// `ρ × Λ = ρ_xΛ_y − ρ_yΛ_x`: its sign is the rotated `λ̃_y`'s.
    pub cross: Wide<R>,
    /// `ρ · Λ = ρ_xΛ_x + ρ_yΛ_y`: its sign is the rotated `λ̃_x`'s.
    pub dot: Wide<R>,
}

/// The branch input of `λ̃`'s signs on the [`Wide`] masses and pair differences `ρ = r₁ − r₀`, `d02 = r₂ − r₀`,
/// `d12 = r₂ − r₁` (the module's note): `Λ = m₀·d02 + m₁·d12`, and `ρ × Λ` and `ρ · Λ` with `ρ` read as `(1, 0)` where
/// it is 0, step 2's guard on the same `ρ`. Every float it rounds is a mantissa product in `[1, 4)` or an aligned sum
/// of mantissas, 0 or normal, so no backend rounds it differently, whether it fuses, contracts or flushes subnormals.
#[inline]
pub fn signs_input<R: LinkReal + IeeeBits>(
    mass: [Wide<R>; 3],
    rho: [Wide<R>; 2],
    d02: [Wide<R>; 2],
    d12: [Wide<R>; 2],
) -> SignsInput<R> {
    let pole = is_pole(rho);
    let one = wide(R::one());
    let zero = wide(R::zero());
    let axis = [select(pole, one, rho[0]), select(pole, zero, rho[1])];
    let lam = |k: usize| wide_add(wide_mul(mass[0], d02[k]), wide_mul(mass[1], d12[k]));
    SignsInput {
        cross: wide_add(
            wide_mul(axis[0], lam(1)),
            wide_neg(wide_mul(axis[1], lam(0))),
        ),
        dot: wide_add(wide_mul(axis[0], lam(0)), wide_mul(axis[1], lam(1))),
    }
}

/// The signs of the rotated `λ̃` on their branch input: each of `ρ × Λ` and `ρ · Λ` below 0 or 0, read on its mantissa,
/// which a `Wide` holds 0 only for zero.
#[inline]
pub fn signs_of<R: LinkReal + IeeeBits>(input: &SignsInput<R>) -> LambdaSigns {
    LambdaSigns {
        y_negative: input.cross.m < R::zero(),
        y_zero: is_zero(input.cross),
        x_negative: input.dot.m < R::zero(),
        x_zero: is_zero(input.dot),
    }
}

/// The signs of the rotated `λ̃` of the state `(m, r)`, in any frame and at any scale, as `C` decides them on its
/// input ([`signs_input`], [`signs_of`]).
#[inline]
pub fn lambda_signs<R: LinkReal + IeeeBits>(m: [R; 3], r: [[R; 2]; 3]) -> LambdaSigns {
    let (mass, x) = wide_state(m, r);
    let (rho, d02, d12) = pair_differences(x);
    signs_of(&signs_input(mass, rho, d02, d12))
}

/// `(v_x, −v_y)` where `flip` holds, else `v`, selected.
#[inline]
fn mirror<R: LinkReal>(v: [R; 2], flip: bool) -> [R; 2] {
    [v[0], select(flip, -v[1], v[1])]
}

/// `Π·ic` where `flip` holds, else `ic`: the mirror through the `x`-axis on the full state, `y → −y` on every `rᵢ` and
/// every `pᵢ` (inverse-encode contract Part 1, "The classic bug"; T1's `Π`), selected; the masses unchanged.
#[inline]
pub fn reflect<R: LinkReal>(ic: &Ic<R>, flip: bool) -> Ic<R> {
    Ic {
        m: ic.m,
        r: [
            mirror(ic.r[0], flip),
            mirror(ic.r[1], flip),
            mirror(ic.r[2], flip),
        ],
        p: [
            mirror(ic.p[0], flip),
            mirror(ic.p[1], flip),
            mirror(ic.p[2], flip),
        ],
    }
}

/// `C`: `ic` onto the canonical section, in the normative order (the module's note): 1a, 0, 1b and 2, so the state is
/// at its CoM, at rest, at `I = 1` with `ρ̃` on `+x`, and `λ̃_y` keeps its sign; the signs of `λ̃` are decided on `ic`
/// and recorded, the state not moved for them. `ic.m` is positive with `Σmᵢ = 1`.
#[inline]
pub fn canonicalise<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> Canonical<R> {
    let out = canonicalise_branches(ic);
    if R::CARRIES_SLOPE {
        smooth_slopes(ic, out)
    } else {
        out
    }
}

/// `out` with the slopes of `C`'s smooth form: the CoM frame, `I = 1` and `ρ̃` on `+x`, always applied. The branches
/// of [`canonicalise_branches`] skip a step where it is a no-op in value (`I_in` within `τ_I` of 1, a zero CoM or `ρ_y`),
/// and a slope through a skipped step is not the slope of the step: don't restore the skip for a type that carries a
/// slope, since the rotation to `ρ_y = 0` has angle 0 there and a derivative that is not. The values are `out`'s.
fn smooth_slopes<R: LinkReal + IeeeBits>(ic: &Ic<R>, out: Canonical<R>) -> Canonical<R> {
    let [m0, m1, m2] = ic.m;
    let (zero, one) = (R::zero(), R::one());
    let com = |k: usize| m0 * ic.r[0][k] + m1 * ic.r[1][k] + m2 * ic.r[2][k];
    let c = [com(0), com(1)];
    let centred = ic.r.map(|v| [v[0] - c[0], v[1] - c[1]]);
    let inertia = m0 * (centred[0][0] * centred[0][0] + centred[0][1] * centred[0][1])
        + m1 * (centred[1][0] * centred[1][0] + centred[1][1] * centred[1][1])
        + m2 * (centred[2][0] * centred[2][0] + centred[2][1] * centred[2][1]);
    let rho = [centred[1][0] - centred[0][0], centred[1][1] - centred[0][1]];
    let norm = Real::hypotenuse(rho[0], rho[1]);
    // A coincident pair or a zero `I` has no smooth form; they are the decode's own poles.
    if out.clamped || !(inertia > zero) || !(norm > zero) || !Float::is_finite(inertia) {
        return out;
    }
    let root = Float::sqrt(inertia);
    let quarter = Float::sqrt(root);
    let scaled = centred.map(|v| [v[0] / root, v[1] / root]);
    let momenta = ic.p.map(|v| [v[0] * quarter, v[1] * quarter]);
    let total = [
        momenta[0][0] + momenta[1][0] + momenta[2][0],
        momenta[0][1] + momenta[1][1] + momenta[2][1],
    ];
    let boosted = [
        [momenta[0][0] - m0 * total[0], momenta[0][1] - m0 * total[1]],
        [momenta[1][0] - m1 * total[0], momenta[1][1] - m1 * total[1]],
        [momenta[2][0] - m2 * total[0], momenta[2][1] - m2 * total[1]],
    ];
    let (cos, sin) = (rho[0] / norm, rho[1] / norm);
    let r = scaled.map(|v| rotate(v, cos, sin));
    let p = boosted.map(|v| rotate(v, cos, sin));
    let from = |a: [[R; 2]; 3], b: [[R; 2]; 3]| {
        [0, 1, 2].map(|i| [a[i][0].slope_from(b[i][0]), a[i][1].slope_from(b[i][1])])
    };
    Canonical {
        ic: Ic {
            m: out.ic.m,
            r: from(out.ic.r, r),
            p: from(out.ic.p, p),
        },
        i_in: out.i_in.slope_from(inertia),
        lambda: out.lambda.slope_from(one / root),
        phi: out.phi.slope_from(Real::arctangent2(rho[1], rho[0])),
        lambda_t_y: out.lambda_t_y.slope_from(lambda_t_y(ic.m, r)),
        ..out
    }
}

/// `C` by the branches the module's note describes, with their skips: the values [`canonicalise`] returns, and at a
/// type that carries a slope not its slopes.
#[inline]
pub fn canonicalise_branches<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> Canonical<R> {
    let m = ic.m;
    let (mass, x) = wide_state(m, ic.r);
    // The pair differences every branch reads (the module's note): step 2's guard and angle, step 0's coincidence and
    // the signs of λ̃, all translation- and scale-free in exact arithmetic.
    let (rho, d02, d12) = pair_differences(x);
    let (turned, clamped) = scaled_rho(rho);
    let phi = angle_of(turned, clamped);
    let signs = signs_of(&signs_input(mass, rho, d02, d12));
    // 1a and 0.
    let scaled = scale_wide(mass, x, ic.p, clamped & is_pole(d02));
    // 1b: zero total momentum.
    let p = scaled.p;
    let total = [p[0][0] + p[1][0] + p[2][0], p[0][1] + p[1][1] + p[2][1]];
    let unboost = |pi: [R; 2], mi: R| {
        [
            Float::mul_add(-mi, total[0], pi[0]),
            Float::mul_add(-mi, total[1], pi[1]),
        ]
    };
    let p = [
        unboost(p[0], m[0]),
        unboost(p[1], m[1]),
        unboost(p[2], m[2]),
    ];
    // 2: ρ onto +x, by ρ̂, `φ`'s cosine and sine; λ̃_y recorded.
    let (c, s) = turn_of(turned, clamped);
    let r = scaled.r;
    let r = [rotate(r[0], c, s), rotate(r[1], c, s), rotate(r[2], c, s)];
    let p = [rotate(p[0], c, s), rotate(p[1], c, s), rotate(p[2], c, s)];
    Canonical {
        ic: Ic { m, r, p },
        i_in: scaled.i_in,
        lambda: scaled.lambda,
        phi,
        lambda_t_y: lambda_t_y(m, r),
        rescaled: scaled.rescaled,
        clamped,
        signs,
    }
}

/// The scale gauge (dd_decoder §3.5): `ℓ = √I`, `rᵢ ← rᵢ/ℓ`, `pᵢ ← √ℓ·pᵢ`, `I = Σmᵢ‖rᵢ‖²` of `ic` as given (CoM at the
/// origin, as the decode emits it), through [`similarity`].
#[inline]
pub fn scale_gauge<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> Ic<R> {
    let (r, p, _) = similarity(ic.r, ic.p, inertia(ic.m, ic.r));
    Ic { m: ic.m, r, p }
}
