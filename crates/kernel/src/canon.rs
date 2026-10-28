//! Canonicalisation `C`, the one seam to `(m, r, p)` (core_design §5; canonical_spec §3; dd_encode §3.2; inverse-encode
//! contract Parts 1, 2 and 6; dd_decoder §3.3, §3.5; chart_reference §0.4, §0.5): the quotient of a physical state onto
//! the decode's canonical section, CoM at the origin, total momentum zero, `ρ̃` on `+x`, `λ̃_y ≥ −δ_λ` and `I = 1`, one
//! source for the f32 kernel and the f64 CPU path (R-265). Decode and encode both run it (seam 8).
//!
//! **The order is normative** (R-23; [`canonicalise`]): every step acts on the full state, every `rᵢ` and every `pᵢ`.
//! ```text
//! 1a COM     r ← r − R_com                         R_com = Σ mᵢrᵢ  (M = 1)
//! 0  SCALE   I_in = Σ mᵢ‖rᵢ‖² in the CoM frame;  r ← r/√I_in,  p ← I_in^{1/4}·p;  λ = I_in^{−1/2} recorded;
//!            lookup_rescaled exactly when step 0 ran (R-547):
//!            |I_in − 1| ≤ τ_I → skipped (λ = 1, r and p unchanged);  else in two steps, by a power of two, then by
//!            the remainder;  the bodies coincident (I_in = 0) → divisor 1, λ = 1, p = 0
//! 1b BOOST   p ← p − mᵢ·P_tot                      P_tot = Σ pᵢ    (M = 1)
//! 2  ROTATE  φ = atan2(ρ_y, ρ_x), ρ = r₁ − r₀;  R(−φ) on every rᵢ and pᵢ;  ρ = 0 → lookup_clamped (R-13), φ = +0
//!            R(−φ)'s cos φ and sin φ formed as ρ/‖ρ‖, the same in exact arithmetic (rotation)
//! 3  MIRROR  iff λ̃_y < −δ_λ (λ̃ = √μ_λ·λ):  y → −y on every rᵢ and pᵢ;  lookup_mirrored;
//!            |λ̃_y| ≤ δ_λ → no mirror (R-82)
//! ```
//! CoM subtraction precedes the `I` computation because `I` is CoM-frame-defined. After step 3, `λ̃_y ≥ −δ_λ`, not
//! `λ̃_y ≥ 0`: the deadband is left unmirrored, `−0` included (R-82, R-438).
//!
//! **Step 3 not taken** (R-544; chart_reference §3, §0.4; chart_decoder_contract Part 2). `C` is one function, its
//! steps written once, in one private function: [`canonicalise`] runs all five, as encode and every other caller do, and
//! [`canonicalise_signed`] runs 1a, 0, 1b and 2 and not 3, so `λ̃_y` keeps its sign and the state its signed shape.
//! The shape sphere chart's decode is its one caller: a lower-hemisphere pixel decodes to its own system, never to its
//! mirror. Its `lookup_mirrored` is never raised, there being no mirror. [`reflect`] is the mirror itself, `Π`, the
//! `y → −y` of every `rᵢ` and every `pᵢ`, which step 3 applies.
//!
//! **Precondition.** The masses are positive and `Σmᵢ = 1`, as the decode emits them: `R_com` and `P_tot` are the
//! mass-weighted sums with `M = 1`. The one exception is `κ`'s mass columns (`encode::ingest`; inverse-encode contract
//! Part 4, "The mass columns"), which differentiate these formulas off the simplex, `M` still taken as 1.
//!
//! **The notices.** [`Canonical`] carries `λ` and `lookup_rescaled` (step 0 ran), `lookup_clamped` and
//! `lookup_mirrored`, each raised iff its operation did (seam 8; REQ-ENC-014).
//!
//! **Step 0** ([`scale_step`]; R-547, closing RQ-295 and RQ-296; dd_encode §3.2; inverse-encode contract Parts 2
//! and 6). Three cases, chosen by selects on its branch inputs (below), every one computed:
//! - **Skipped** where `|I_in − 1| ≤ τ_I`, `τ_I` the register's `tau_i` eps of the precision ([`IeeeBits::TAU_I`];
//!   REQ-ENC-035): `λ = 1`, `r` and `p` leave the step bit for bit as they entered it, and `lookup_rescaled` is not
//!   raised. A canonical input, at `I = 1` only to rounding, raises no notice.
//! - **Run** elsewhere, the bodies apart, in two steps: `I_in = s·2^{4q}` with `q` an integer and `s` in `[1, 16)`
//!   (`I_in`'s mantissa times `2^b`, `b` the low two bits of its exponent), so `√I_in = √s·2^{2q}` and
//!   `I_in^{1/4} = √√s·2^q`: first by the power of two, `r·2^{−2q}` and `p·2^q`, exact (an integer added to an
//!   exponent, or a product by a power of two), then by the remainder, `r/√s` and `√√s·p`, with `√s` in `[1, 4)`.
//!   `λ = 2^{−2q}/√s`. `I_in` itself is never a float: it is carried as a mantissa and an integer exponent (`Wide`,
//!   below), so it neither underflows nor overflows for any finite state with the bodies apart, and the state leaves
//!   step 0 at `I = 1` to rounding. `lookup_rescaled` is raised.
//! - **The bodies coincident**, `r₀ = r₁ = r₂`: divisor 1, `λ = 1`, `p = 0`, `r` the CoM frame's, no
//!   `lookup_rescaled`; `ρ = 0` raises `lookup_clamped` (R-13) and `φ = +0` (R-437). With positive masses `I_in = 0`
//!   happens there only. The select is on the coincidence itself, not on `I_in = 0`: `R_com` of three equal positions
//!   off the origin is rounded, so their CoM frame can be an ulp from 0 and `I_in` an ulp's square, not 0, and the
//!   case R-547 names is the coincidence (decided per R-369).
//!
//! The recorded `I_in` ([`Canonical::i_in`]) is the input's, converted to the float: where it is past the float's
//! range, at f32 coordinates above about `1e19` or below about `4e-23` (f64 above about `1e154`, below about
//! `1e-162`), it reads `+∞` or a subnormal or 0, a record only; nothing in `C` reads it. `λ` is past the float's range
//! only where `I_in^{−1/2}` itself is, the bodies within about `1e-38` of each other at f32.
//!
//! **`φ` at the pole** (R-437; dd_decoder §3.2, §3.3; [`rotation_angle`]): at `ρ = 0`, `φ` is 0 for every sign of the
//! zeros, a select on `ρ_x = 0` and `ρ_y = 0`, never `atan2(0, 0)`, which GLSL.std.450 leaves undefined; `R(−φ)` is then
//! the identity on CPU-f64 and GPU-f32 alike. Only the exact `ρ = 0` reaches `lookup_clamped` here; a near-zero `ρ` is
//! the validation ladder's (TASK-M2-15).
//!
//! **The branch inputs** (R-547, correcting R-34 and the determinism note's rules 2 and 6; R-84). Five branches must
//! agree across backends: step 0's skip, its coincidence select, step 2's guard `ρ = 0`, step 3's mirror decision, and
//! where `ρ_x` or `ρ_y` is 0 in step 2's turn and angle. Each is built only from operations every IEEE backend rounds
//! alike, with no intermediate that can be subnormal. `√`, `/` and `atan2` carry ulp bounds in WGSL, not correct
//! rounding; `fma` is not fused on every backend (CI's lavapipe evaluates it as a rounded product and an add, RQ-298),
//! nor is `a·b + c` left uncontracted on every one; and a backend that flushes subnormal results (Metal) rounds a
//! subnormal intermediate differently from one that keeps it (the CPU), fused or not. So none of them is built from
//! those. Every quantity a branch reads is carried as a `Wide`, a mantissa in `[1, 2)` and an integer exponent, its
//! operations ([`IeeeBits::product`]'s pattern):
//! - **Into the form** (`wide`): an input coordinate, a mass, `δ_λ`, `τ_I` or 1 is scaled by `2^−k`, `k` its exponent
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
//! its additions and multiplications correctly rounded and unreassociated (fast-math off, R-84), whether it fuses,
//! contracts or flushes subnormals. Each branch input, in exact arithmetic R-23's step:
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
//! - **The mirror** ([`mirror_decision`]). `R(−φ)` puts `ρ̂ = ρ/‖ρ‖` on `+x` and keeps cross products, so a vector's `y`
//!   after it is its cross product with `ρ̂`, `a × b = a_xb_y − a_yb_x`. In the scaled frame `λ = Λ/(M₀₁d)`, with
//!   `Λ = M₀₁r₂ − m₀r₀ − m₁r₁ = m₀(r₂ − r₀) + m₁(r₂ − r₁)`; hence `λ̃_y = √(m₂M₀₁)·c/(‖ρ‖·M₀₁·d)`, `c = ρ × Λ`. The
//!   factor of `c` is positive, so `λ̃_y < −δ_λ` iff `c < 0` and `c² > δ_λ²·‖ρ‖²·d²·M₀₁/m₂`, that is
//!   `m₂c² > δ_λ²·M₀₁·‖ρ‖²·d²`: `δ_λ` carried into the units of `c²`. At `ρ = 0` step 2 is the identity and
//!   `λ̃_y = √(m₂M₀₁)·Λ_y/(M₀₁d)`, the same with `ρ` read as `(1, 0)`. `d² = I_in = Σ_{i<j} mᵢmⱼ‖rⱼ − rᵢ‖²` (with
//!   `Σmᵢ = 1`), from the differences too, so the test reads no CoM. `c²` and `‖ρ‖²d²` are both of degree 4 in the
//!   state's scale, and `‖ρ‖²` of degree 2 in the inner pair's own size, so a float form of the test overflows,
//!   underflows or, on a GPU that flushes subnormals, drops terms, at large or small scales and on hierarchical states
//!   (an inner pair `10⁻³` of the outer distance is enough at f32: `δ_λ² ≈ 2^−80` takes the products' low parts below
//!   the smallest normal); every quantity is a `Wide`. The pair differences `ρ`, `d02` and `d12` are `Wide` differences
//!   of the coordinates, not float subtractions, which are subnormal for normal coordinates closer than about
//!   `1.2e-38` at f32 and which a flushing backend returns as 0.
//!
//! `|λ̃_y| ≤ δ_λ`, `−δ_λ` itself included, is no mirror. [`Canonical::lambda_t_y`] is formed after the rotation and read,
//! not branched on; within rounding of `−δ_λ` it may disagree with the flag. The cross-backend claim is for identical
//! inputs only (PIT-10). A state with a subnormal coordinate is itself read differently by a flushing GPU, before `C`
//! runs; on a backend that keeps subnormals, the CPU's, such a coordinate is taken into the `Wide` form exactly too.
//!
//! **The scale gauge** ([`scale_gauge`]; dd_decoder §3.5, chart_reference §0.5): `ℓ = √I`, `rᵢ ← rᵢ/ℓ`,
//! `pᵢ ← √ℓ·pᵢ` ([`similarity`]), at every `I`: dd_decoder §3.5's map, a no-op, to rounding, on canonical output. Step
//! 0 is the same map in exact arithmetic, skipped within `τ_I` and taken in two steps elsewhere.

use spirv_std::num_traits::Float;

use crate::decode::Ic;
use crate::generated::links::LinkReal;
use crate::real::IeeeBits;

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
    /// `λ̃_y = √μ_λ·λ_y` after the rotation, before the mirror ([`lambda_t_y`]): recorded, not branched on.
    pub lambda_t_y: R,
    /// `lookup_rescaled`: step 0 ran, `|I_in − 1| > τ_I` with the bodies apart (R-547).
    pub rescaled: bool,
    /// `lookup_clamped`: `ρ = 0` exactly at step 2 (R-13), read before step 0's division.
    pub clamped: bool,
    /// `lookup_mirrored`: step 3 mirrored the state, `λ̃_y < −δ_λ` (R-82), by [`mirror_decision`].
    pub mirrored: bool,
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
/// [`ZERO_EXPONENT`]. The arithmetic of `C`'s branch inputs (the module's note), so that no float it rounds lies outside
/// `[2^−83, 4)` in magnitude but 0 (f32; `[2^−112, 4)` at f64), and none is subnormal on any backend, whatever the
/// state's scale or its terms' spread.
#[derive(Clone, Copy)]
struct Wide<R> {
    m: R,
    e: i32,
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
fn wide<R: LinkReal + IeeeBits>(x: R) -> Wide<R> {
    let k = x.exponent();
    let half = k / 2;
    renorm(x * R::pow2(-half) * R::pow2(half - k), k)
}

/// `a·b`: the mantissas' [`IeeeBits::product`], in `[1, 4)`, whose partial products are at least `2^−46`, renormalised.
#[inline]
fn wide_mul<R: LinkReal + IeeeBits>(a: Wide<R>, b: Wide<R>) -> Wide<R> {
    renorm(a.m.product(b.m), a.e + b.e)
}

/// `a + b`, aligned on the larger exponent: the smaller mantissa times `2^−s`, `s` the exponents' difference capped at
/// 60, below which it is under half an ulp of the larger (so the cap changes no rounding), then renormalised. A
/// nonzero sum is at least `2^−83` (f32; `2^−112` at f64), the operands being multiples of it: normal.
#[inline]
fn wide_add<R: LinkReal + IeeeBits>(a: Wide<R>, b: Wide<R>) -> Wide<R> {
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
fn wide_sub<R: LinkReal + IeeeBits>(a: Wide<R>, b: Wide<R>) -> Wide<R> {
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
fn wide_greater<R: LinkReal + IeeeBits>(a: Wide<R>, b: Wide<R>) -> bool {
    (a.e > b.e) | ((a.e == b.e) & (a.m > b.m))
}

/// `a = 0`, on the mantissa, which a `Wide` holds 0 only for zero.
#[inline]
fn is_zero<R: LinkReal + IeeeBits>(a: Wide<R>) -> bool {
    a.m == R::zero()
}

/// `‖v‖² = v_x² + v_y²`.
#[inline]
fn wide_norm2<R: LinkReal + IeeeBits>(v: [Wide<R>; 2]) -> Wide<R> {
    wide_add(wide_mul(v[0], v[0]), wide_mul(v[1], v[1]))
}

/// `v`'s components as [`Wide`]s.
#[inline]
fn vector<R: LinkReal + IeeeBits>(v: [R; 2]) -> [Wide<R>; 2] {
    [wide(v[0]), wide(v[1])]
}

/// `b − a`, per component.
#[inline]
fn difference<R: LinkReal + IeeeBits>(a: [Wide<R>; 2], b: [Wide<R>; 2]) -> [Wide<R>; 2] {
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
fn to_float<R: LinkReal + IeeeBits>(a: Wide<R>) -> R {
    ldexp(a.m, a.e)
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
fn wide_state<R: LinkReal + IeeeBits>(
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

/// Step 0's skip test, the branch input: `|I_in − 1| ≤ τ_I`, compared as [`Wide`]s.
#[inline]
fn skipped<R: LinkReal + IeeeBits>(i: Wide<R>) -> bool {
    let off = wide_abs(wide_sub(i, wide(R::one())));
    !wide_greater(off, wide(R::TAU_I))
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

/// Steps 1a and 0 on the [`Wide`] state `(mass, x)` and the momenta `p` (the module's note); `coincident` is the
/// bodies' coincidence, `ρ = 0` and `d02 = 0`.
#[inline]
fn scale_wide<R: LinkReal + IeeeBits>(
    mass: [Wide<R>; 3],
    x: [[Wide<R>; 2]; 3],
    p: [[R; 2]; 3],
    coincident: bool,
) -> Scaled<R> {
    // 1a: the CoM frame.
    let com = wide_com(mass, x);
    let c = [
        [wide_sub(x[0][0], com[0]), wide_sub(x[0][1], com[1])],
        [wide_sub(x[1][0], com[0]), wide_sub(x[1][1], com[1])],
        [wide_sub(x[2][0], com[0]), wide_sub(x[2][1], com[1])],
    ];
    // 0: I_in, the skip, and the two steps, `I_in = s·2^{4q}`, `s` in [1, 16).
    let i = wide_inertia(mass, c);
    let run = !(skipped(i) | coincident);
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

/// Both components 0: `ρ = 0`, step 2's guard (`lookup_clamped`, R-13), true for `+0` and `−0` alike.
#[inline]
fn is_pole<R: LinkReal + IeeeBits>(v: [Wide<R>; 2]) -> bool {
    is_zero(v[0]) & is_zero(v[1])
}

/// `ρ` scaled for step 2 (the module's note): each component times `2^−e`, `e` the larger one's exponent, the gap to the
/// smaller capped at 60, so the larger is in `[1, 2)` and the smaller 0 or at least `2^−60` in magnitude; and whether
/// `ρ = 0`, the pole. At the pole both are `±0`.
#[inline]
fn scaled_rho<R: LinkReal + IeeeBits>(rho: [Wide<R>; 2]) -> ([R; 2], bool) {
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

/// Step 2's angle from the scaled `ρ` (`x`, `y`) and its pole: `atan2(y, x)`, a zero `x` taken as `+0`, and `+0` at the
/// pole, each a select ([`rotation_angle`]).
#[inline]
fn angle_of<R: LinkReal>(v: [R; 2], pole: bool) -> R {
    let x = select(v[0] == R::zero(), R::zero(), v[0]);
    select(pole, R::zero(), Float::atan2(v[1], x))
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

/// Step 2's angle `φ = atan2(ρ_y, ρ_x)`, and 0 at `ρ = 0` for every sign of the zeros, selected (R-437). `ρ` is taken
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
/// at `ρ = 0`, `(1, 0)`, so `R(−φ)` is the identity there (R-437). In exact arithmetic `ρ̂` is `(cos φ, sin φ)`. In
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
/// `m₀ + m₁` (`fma(m₁, 1, m₀)`, as the mass block forms it): step 3's quantity, read on `C`'s rotated output and
/// recorded in [`Canonical::lambda_t_y`]. `C` does not branch on it ([`mirror_decision`] does the branching).
#[inline]
pub fn lambda_t_y<R: LinkReal>(m: [R; 3], r: [[R; 2]; 3]) -> R {
    lambda_t_axis(m, [r[0][1], r[1][1], r[2][1]])
}

/// One component of `λ̃ = √μ_λ·λ` under `m`, from the three bodies' coordinates `x` on that axis:
/// `√(m₂M₀₁)·(x₂ − fma(m₁, x₁, m₀x₀)/M₀₁)`, `M₀₁ = fma(m₁, 1, m₀)`. The one place this arithmetic is written:
/// [`lambda_t_y`] reads it on `y`, and the encode's Jacobi map (`crate::encode::blocks::jacobi`) on both axes, so the
/// mirror decision and `β` read the same bits.
#[inline]
pub fn lambda_t_axis<R: LinkReal>(m: [R; 3], x: [R; 3]) -> R {
    let m01 = Float::mul_add(m[1], R::one(), m[0]);
    let inner = Float::mul_add(m[1], x[1], m[0] * x[0]) / m01;
    Float::sqrt(m[2] * m01) * (x[2] - inner)
}

/// R-82's test as it reads, on a given `λ̃_y`: mirror iff `λ̃_y < −δ_λ`; `|λ̃_y| ≤ δ_λ` is no mirror. The reference
/// [`mirror_decision`] equals in exact arithmetic.
#[inline]
pub fn mirror_taken<R: LinkReal>(lambda_t_y: R) -> bool {
    lambda_t_y < -R::DELTA_LAMBDA
}

/// Step 3's decision on the [`Wide`] masses and pair differences `ρ = r₁ − r₀`, `d02 = r₂ − r₀`, `d12 = r₂ − r₁`
/// ([`mirror_decision`]).
#[inline]
fn decide<R: LinkReal + IeeeBits>(
    mass: [Wide<R>; 3],
    rho: [Wide<R>; 2],
    d02: [Wide<R>; 2],
    d12: [Wide<R>; 2],
) -> bool {
    let pole = is_pole(rho);
    let one = wide(R::one());
    let zero = wide(R::zero());
    let axis = [select(pole, one, rho[0]), select(pole, zero, rho[1])];
    let lam = |k: usize| wide_add(wide_mul(mass[0], d02[k]), wide_mul(mass[1], d12[k]));
    let c = wide_add(
        wide_mul(axis[0], lam(1)),
        wide_neg(wide_mul(axis[1], lam(0))),
    );
    let d2 = wide_add(
        wide_add(
            wide_mul(wide_mul(mass[0], mass[1]), wide_norm2(rho)),
            wide_mul(wide_mul(mass[0], mass[2]), wide_norm2(d02)),
        ),
        wide_mul(wide_mul(mass[1], mass[2]), wide_norm2(d12)),
    );
    let m01 = wide_add(mass[0], mass[1]);
    let delta = wide(R::DELTA_LAMBDA);
    let left = wide_mul(wide_mul(c, c), mass[2]);
    let right = wide_mul(
        wide_mul(wide_mul(wide_mul(delta, delta), m01), wide_norm2(axis)),
        d2,
    );
    (c.m < R::zero()) & wide_greater(left, right)
}

/// Step 3's decision (R-82), on the positions `r` in any frame, before step 0 (the module's note):
/// 1. every coordinate and mass taken as a `Wide`, its exponent carried as an integer, and the pair differences
///    `ρ = r₁ − r₀`, `d02 = r₂ − r₀`, `d12 = r₂ − r₁` formed as `Wide` differences;
/// 2. `Λ = m₀d02 + m₁d12` (`= M₀₁r₂ − m₀r₀ − m₁r₁`), `c = ρ × Λ = ρ_xΛ_y − ρ_yΛ_x` with `ρ` read as `(1, 0)` where it is
///    0 (step 2's guard, on the same `ρ`), `d² = Σ_{i<j} mᵢmⱼ‖dᵢⱼ‖²`, and mirror iff `c < 0` and
///    `m₂c² > δ_λ²·M₀₁·‖ρ‖²·d²`, compared by exponent, then mantissa.
///
/// Every float the decision rounds is a mantissa product in `[1, 4)` (an [`IeeeBits::product`], its partial products
/// at least `2^−46`), an aligned sum of mantissas scaled by at most `2^−60`, 0 or at least `2^−83` (f32), or a
/// power-of-two scaling into `[1, 2)`: none is subnormal and none overflows, for any state whose coordinates are 0 or
/// normal, its pair differences whatever their size. So no backend rounds it differently, whether it fuses, contracts
/// or flushes subnormals, and the same input takes the same branch everywhere.
#[inline]
pub fn mirror_decision<R: LinkReal + IeeeBits>(m: [R; 3], r: [[R; 2]; 3]) -> bool {
    let (mass, x) = wide_state(m, r);
    decide(
        mass,
        difference(x[0], x[1]),
        difference(x[0], x[2]),
        difference(x[1], x[2]),
    )
}

/// `(v_x, −v_y)` where `flip` holds, else `v`, selected.
#[inline]
fn mirror<R: LinkReal>(v: [R; 2], flip: bool) -> [R; 2] {
    [v[0], select(flip, -v[1], v[1])]
}

/// `Π·ic` where `flip` holds, else `ic`: the mirror through the `x`-axis on the full state, `y → −y` on every `rᵢ` and
/// every `pᵢ` (inverse-encode contract Part 1, "The classic bug"; step 3's), selected; the masses unchanged.
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

/// `C`: `ic` onto the canonical section, in R-23's order (the module's note). `ic.m` is positive with `Σmᵢ = 1`.
#[inline]
pub fn canonicalise<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> Canonical<R> {
    canonical(ic, true)
}

/// `C` with its step 3, the mirror, not taken (R-544; the module's note): 1a, 0, 1b and 2, so the state is at its CoM,
/// at rest, at `I = 1` with `ρ̃` on `+x`, and `λ̃_y` keeps its sign. `mirrored` is `false`. The shape sphere chart's
/// decode; every other caller runs [`canonicalise`].
#[inline]
pub fn canonicalise_signed<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> Canonical<R> {
    canonical(ic, false)
}

/// `C`'s steps, written once: 1a, 0, 1b and 2, then step 3 where `mirror_step` holds and R-82's test mirrors.
#[inline]
fn canonical<R: LinkReal + IeeeBits>(ic: &Ic<R>, mirror_step: bool) -> Canonical<R> {
    let m = ic.m;
    let (mass, x) = wide_state(m, ic.r);
    // The pair differences every branch reads (the module's note): step 2's guard and angle, step 3's decision and
    // step 0's coincidence, all translation- and scale-free in exact arithmetic.
    let (rho, d02, d12) = (
        difference(x[0], x[1]),
        difference(x[0], x[2]),
        difference(x[1], x[2]),
    );
    let (turned, clamped) = scaled_rho(rho);
    let phi = angle_of(turned, clamped);
    let mirrored = mirror_step & decide(mass, rho, d02, d12);
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
    // 2: ρ onto +x, by ρ̂, `φ`'s cosine and sine.
    let (c, s) = turn_of(turned, clamped);
    let r = scaled.r;
    let r = [rotate(r[0], c, s), rotate(r[1], c, s), rotate(r[2], c, s)];
    let p = [rotate(p[0], c, s), rotate(p[1], c, s), rotate(p[2], c, s)];
    // 3: the mirror, in encode's frame, as decided above; λ̃_y recorded.
    let lambda_t_y = lambda_t_y(m, r);
    Canonical {
        ic: reflect(&Ic { m, r, p }, mirrored),
        i_in: scaled.i_in,
        lambda: scaled.lambda,
        phi,
        lambda_t_y,
        rescaled: scaled.rescaled,
        clamped,
        mirrored,
    }
}

/// The scale gauge (dd_decoder §3.5): `ℓ = √I`, `rᵢ ← rᵢ/ℓ`, `pᵢ ← √ℓ·pᵢ`, `I = Σmᵢ‖rᵢ‖²` of `ic` as given (CoM at the
/// origin, as the decode emits it), through [`similarity`].
#[inline]
pub fn scale_gauge<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> Ic<R> {
    let (r, p, _) = similarity(ic.r, ic.p, inertia(ic.m, ic.r));
    Ic { m: ic.m, r, p }
}
