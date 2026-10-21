//! Canonicalisation `C`, the one seam to `(m, r, p)` (core_design §5; canonical_spec §3; dd_encode §3.2; inverse-encode
//! contract Parts 1, 2 and 6; dd_decoder §3.3, §3.5; chart_reference §0.4, §0.5): the quotient of a physical state onto
//! the decode's canonical section, CoM at the origin, total momentum zero, `ρ̃` on `+x`, `λ̃_y ≥ −δ_λ` and `I = 1`, one
//! source for the f32 kernel and the f64 CPU path (R-265). Decode and encode both run it (seam 8).
//!
//! **The order is normative** (R-23; [`canonicalise`]): every step acts on the full state, every `rᵢ` and every `pᵢ`.
//! ```text
//! 1a COM     r ← r − R_com                         R_com = Σ mᵢrᵢ  (M = 1)
//! 0  SCALE   I_in = Σ mᵢ‖rᵢ‖² in the CoM frame;  r ← r/√I_in,  p ← I_in^{1/4}·p;  λ = I_in^{−1/2} recorded
//! 1b BOOST   p ← p − mᵢ·P_tot                      P_tot = Σ pᵢ    (M = 1)
//! 2  ROTATE  φ = atan2(ρ_y, ρ_x), ρ = r₁ − r₀;  R(−φ) on every rᵢ and pᵢ;  ρ = 0 → lookup_clamped (R-13)
//!            R(−φ)'s cos φ and sin φ formed as ρ/‖ρ‖, the same in exact arithmetic (rotation)
//! 3  MIRROR  iff λ̃_y < −δ_λ (λ̃ = √μ_λ·λ):  y → −y on every rᵢ and pᵢ;  lookup_mirrored;
//!            |λ̃_y| ≤ δ_λ → no mirror (R-82)
//! ```
//! CoM subtraction precedes the `I` computation because `I` is CoM-frame-defined. After step 3, `λ̃_y ≥ −δ_λ`, not
//! `λ̃_y ≥ 0`: the deadband is left unmirrored, `−0` included (R-82, R-438).
//!
//! **Precondition.** The masses are positive and `Σmᵢ = 1`, as the decode emits them: `R_com` and `P_tot` are the
//! mass-weighted sums with `M = 1`.
//!
//! **The notices.** [`Canonical`] carries `λ` (`lookup_rescaled`'s value), `lookup_clamped` and `lookup_mirrored`.
//! When `lookup_rescaled` is raised, for a state already at `I = 1` to rounding included, is open (RQ-295): the corpus
//! gives the step that records `λ` and the rule that a notice fires iff its operation did, but not whether a rescale by
//! `λ = 1` to rounding is one. So `λ` is recorded at every call and no raised/not-raised flag is formed here.
//!
//! **`φ` at the pole** (R-437; dd_decoder §3.2, §3.3; [`rotation_angle`]): at `ρ = 0`, `φ` is 0 for every sign of the
//! zeros, a select on `ρ_x = 0` and `ρ_y = 0`, never `atan2(0, 0)`, which GLSL.std.450 leaves undefined; `R(−φ)` is then
//! the identity on CPU-f64 and GPU-f32 alike. Only the exact `ρ = 0` reaches `lookup_clamped` here; a near-zero `ρ` is
//! the validation ladder's (TASK-M2-15).
//!
//! **`I_in` at 0 and at `+∞`** (RQ-296). What `C` outputs where `I_in` is 0 or `+∞` is open (RQ-296); what the code
//! does there is only kept finite where it can be, by one select:
//! - **`I_in = 0` exactly**, all three bodies coincident: step 0's divisor `√I_in` is selected as 1, so no `1/0` is
//!   formed (R-436's producer rule); `r` stays 0, every `p` becomes `0·p`, `λ = 1`, and `ρ = 0` raises `lookup_clamped`
//!   (R-13), `φ = 0` (R-437).
//! - **`I_in` rounding to 0 with the bodies apart** (f64 below about `1e-162` per coordinate; f32 below about `4e-23`,
//!   about `1e-19` where a GPU flushes subnormals): the same select, so `r` is left unscaled, at `I ≈ 0`, not 1, every
//!   momentum becomes `0·p` (`√√0 = 0`) and `λ = 1`; since `ρ ≠ 0`, no `lookup_clamped`. The state leaves `C` off the
//!   section, unflagged. The mirror is still R-82's on the state's own `λ̃_y`, which is scale-free (the decision's
//!   range, below).
//! - **`I_in` rounding to `+∞`** (f64 above about `1e154` per coordinate, f32 above about `1e19`): `√I_in = +∞`, so
//!   `r/√I_in` is `±0`, `λ = 1/∞ = 0`, and `I_in^{1/4}·p` is `±∞`, or NaN for a zero component; step 1b's sums and the
//!   rotation then make every momentum NaN on the CPU. `ρ` and the mirror are read from the input's differences before
//!   the division (below), so `lookup_clamped` is raised iff `ρ = r₁ − r₀` is 0, `φ` is that `ρ`'s angle, and the
//!   mirror is R-82's on the state's own `λ̃_y` (the decision's range, below). On the GPU this arithmetic on `∞`
//!   is not pinned (WGSL may assume there are no infinities; R-436).
//!
//! **The branch inputs, read before step 0** (R-34, R-84; gpu_determinism_note rules 1 and 2). Step 2's guard and
//! angle and step 3's decision are computed from the input's pair differences, before step 1a's subtraction and step
//! 0's division, in arithmetic every backend rounds alike. `√`, `/` and `atan2` carry ulp bounds in WGSL, not correct
//! rounding; and `fma` is not fused on every backend (CI's lavapipe evaluates it as a rounded product and an add,
//! RQ-298), nor is `a·b + c` left uncontracted on every one. So a branch fed through any of them can differ between
//! backends on one input. Each is R-23's step in exact arithmetic, `d > 0` being step 0's divisor:
//! - **The guard and `φ`.** `ρ = r₁ − r₀`, one subtraction, is the same in every frame, and is 0 iff the scaled
//!   `ρ/d` is; `atan2` is unchanged by the positive factor `1/d`.
//! - **The mirror** ([`mirror_decision`]). `R(−φ)` puts `ρ̂ = ρ/‖ρ‖` on `+x` and keeps cross products, so a vector's `y`
//!   after it is its cross product with `ρ̂`, `a × b = a_xb_y − a_yb_x`. In the scaled frame `λ = Λ/(M₀₁d)`, with
//!   `Λ = M₀₁r₂ − m₀r₀ − m₁r₁ = m₀(r₂ − r₀) + m₁(r₂ − r₁)`; hence `λ̃_y = √(m₂M₀₁)·c/(‖ρ‖·M₀₁·d)`, `c = ρ × Λ`. The
//!   factor of `c` is positive, so `λ̃_y < −δ_λ` iff `c < 0` and `c² > δ_λ²·‖ρ‖²·d²·M₀₁/m₂`, that is
//!   `m₂c² > δ_λ²·M₀₁·‖ρ‖²·d²`: `δ_λ` carried into the units of `c²`. At `ρ = 0` step 2 is the identity and
//!   `λ̃_y = √(m₂M₀₁)·Λ_y/(M₀₁d)`, the same with `ρ` read as `(1, 0)`. `d² = I_in = Σ_{i<j} mᵢmⱼ‖rⱼ − rᵢ‖²` (with
//!   `Σmᵢ = 1`), from the differences too, so the test reads no CoM.
//! - **The range.** `c²` and `‖ρ‖²d²` are both of degree 4 in the state's scale, and `‖ρ‖²` is of degree 2 in the
//!   inner pair's own size, so a float form of the test overflows, underflows or, on a GPU that flushes subnormals,
//!   drops terms, at large or small scales and on hierarchical states (an inner pair `10⁻³` of the outer distance is
//!   enough at f32: `δ_λ² ≈ 2^−80` takes the products' low parts below the smallest normal). So
//!   [`mirror_decision`] carries every quantity as a mantissa in `[1, 2)` and an integer exponent (`Wide`): products
//!   multiply mantissas and add exponents, sums align on the larger exponent, and the two sides are compared by
//!   exponent, then mantissa. `δ_λ`, the masses and each difference are taken into that form exactly from their bits.
//! - **The products** ([`IeeeBits::product`]). At f32 every mantissa product is formed from its operands' bit-split
//!   halves, whose partial products are exact and, the mantissas being in `[1, 2)`, at least `2^−46`; they are summed
//!   in one fixed order. A backend that fuses or contracts any site rounds the same exact sums. Every other float
//!   operation is an aligned sum of mantissas, the smaller scaled by at most `2^−60`, or a scaling by a power of two into
//!   `[1, 2)`; the rest are integer operations on bits and exponents, selects and comparisons.
//!
//! So `|λ̃_y| ≤ δ_λ`, `−δ_λ` itself included, is no mirror, and no float the decision rounds is subnormal or overflows,
//! for any state whose coordinates and pair differences are 0 or normal (at f32, none below about `1.2e-38` in
//! magnitude): one input takes one branch on every IEEE backend that keeps its additions and multiplications correctly
//! rounded and unreassociated (fast-math off, R-84), whether it fuses, contracts or flushes subnormals. A state with a
//! subnormal coordinate or difference is itself read differently by a flushing GPU, before `C` runs (RQ-298); on a
//! backend that keeps subnormals, the CPU's, such a difference is taken into the `Wide` form exactly too, so the
//! decision there is R-82's within the decision's rounding, as on any other state.
//! [`Canonical::lambda_t_y`] is formed after the rotation and read, not branched on; within rounding of `−δ_λ` it may
//! disagree with the flag. The cross-backend claim is for identical inputs only (PIT-10).
//!
//! **The scale gauge** ([`scale_gauge`]; dd_decoder §3.5, chart_reference §0.5): `ℓ = √I`, `rᵢ ← rᵢ/ℓ`,
//! `pᵢ ← √ℓ·pᵢ`, the same similarity map as step 0 ([`similarity`]); a no-op, to rounding, on canonical output.

use spirv_std::num_traits::Float;

use crate::decode::Ic;
use crate::generated::links::LinkReal;
use crate::real::IeeeBits;

/// A state on the canonical section, and what `C` did to reach it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canonical<R> {
    /// The canonical `(m, r, p)`.
    pub ic: Ic<R>,
    /// `I_in`, the input's moment of inertia in its CoM frame, computed after step 1a.
    pub i_in: R,
    /// `λ = I_in^{−1/2}`, recorded at step 0: `lookup_rescaled`'s value (RQ-295 holds when it is raised).
    pub lambda: R,
    /// Step 2's rotation angle `φ`, the rotation applied being `R(−φ)`.
    pub phi: R,
    /// `λ̃_y = √μ_λ·λ_y` after the rotation, before the mirror ([`lambda_t_y`]): recorded, not branched on.
    pub lambda_t_y: R,
    /// `lookup_clamped`: `ρ = 0` exactly at step 2 (R-13), read before step 0's division.
    pub clamped: bool,
    /// `lookup_mirrored`: step 3 mirrored the state, `λ̃_y < −δ_λ` (R-82), by [`mirror_decision`].
    pub mirrored: bool,
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

/// `m₀x₀ + m₁x₁ + m₂x₂`, as `fma(m₂, x₂, fma(m₁, x₁, m₀x₀))`.
#[inline]
fn weighted<R: LinkReal>(m: [R; 3], x0: R, x1: R, x2: R) -> R {
    Float::mul_add(m[2], x2, Float::mul_add(m[1], x1, m[0] * x0))
}

/// `‖v‖² = fma(v_x, v_x, v_y·v_y)`.
#[inline]
fn norm2<R: LinkReal>(v: [R; 2]) -> R {
    Float::mul_add(v[0], v[0], v[1] * v[1])
}

/// The centre of mass `Σ mᵢrᵢ` of `r` under the masses `m`, `Σmᵢ = 1`.
#[inline]
pub fn centre_of_mass<R: LinkReal>(m: [R; 3], r: [[R; 2]; 3]) -> [R; 2] {
    [
        weighted(m, r[0][0], r[1][0], r[2][0]),
        weighted(m, r[0][1], r[1][1], r[2][1]),
    ]
}

/// The moment of inertia `Σ mᵢ‖rᵢ‖²` of `r` about the origin.
#[inline]
pub fn inertia<R: LinkReal>(m: [R; 3], r: [[R; 2]; 3]) -> R {
    weighted(m, norm2(r[0]), norm2(r[1]), norm2(r[2]))
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

/// The similarity map at the moment of inertia `i` (step 0; dd_decoder §3.5): `rᵢ/√i`, `i^{1/4}·pᵢ`, and
/// `λ = 1/√i`. At `i = 0` the divisor is selected as 1, so no `1/0` is formed (RQ-296 holds the output there).
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

/// Step 2's guard: `ρ = 0` exactly, `ρ_x = 0` and `ρ_y = 0`, each true for `+0` and `−0` alike (`lookup_clamped`, R-13).
#[inline]
pub fn coincident<R: LinkReal>(rho: [R; 2]) -> bool {
    (rho[0] == R::zero()) & (rho[1] == R::zero())
}

/// Step 2's angle `φ = atan2(ρ_y, ρ_x)`, and 0 at `ρ = 0` for every sign of the zeros, selected (R-437).
#[inline]
pub fn rotation_angle<R: LinkReal>(rho: [R; 2]) -> R {
    select(coincident(rho), R::zero(), Float::atan2(rho[1], rho[0]))
}

/// Step 2's turn `(c, s) = (cos φ, sin φ)` for `φ` = [`rotation_angle`]`(ρ)`, formed as `ρ̂ = ρ/‖ρ‖`, not through `φ`;
/// at `ρ = 0`, `(1, 0)`, so `R(−φ)` is the identity there (R-437). In exact arithmetic `ρ̂` is `(cos φ, sin φ)`. In
/// rounded arithmetic `sin` and `cos` of a rounded `atan2` turn `ρ` off `+x` by their error in `φ`, which WGSL bounds
/// only loosely (`atan2` to 4096 ulp, `sin` and `cos` to an absolute `2⁻¹¹`) and `R(−φ)` multiplies by each `‖rᵢ‖`; a
/// light body is far out at `I = 1` (`‖r₁‖ ≈ 27` at `m₁ ≈ 1.2·10⁻³`), and through the sines CI's lavapipe put that
/// body's `r_y` 273.88 eps from the f64 decode where the native f32 decode is 1.12 (PR #262). `ρ̂`'s direction is
/// rounded only by the two products `ρ_x·k` and `ρ_y·k`, half an ulp each, the error of `k = 1/√(ρ_x² + ρ_y²)`
/// scaling both alike. `ρ` is first scaled by a power of two, in two exact steps that keep each factor normal, its
/// larger component into `[1, 2)` (from a subnormal, at least `2⁻²³` at f32), so `ρ_x² + ρ_y²` neither overflows nor
/// underflows for a finite `ρ`, and is 0 only at `ρ = 0` (or where a GPU flushes a subnormal `ρ` to 0, RQ-298).
#[inline]
pub fn rotation<R: LinkReal + IeeeBits>(rho: [R; 2]) -> (R, R) {
    let (ex, ey) = (rho[0].exponent(), rho[1].exponent());
    let e = select(ex > ey, ex, ey);
    let h = e / 2;
    let (a, b) = (R::pow2(-h), R::pow2(h - e));
    let (x, y) = (rho[0] * a * b, rho[1] * a * b);
    let n2 = Float::mul_add(x, x, y * y);
    let pole = n2 == R::zero();
    let k = R::one() / Float::sqrt(select(pole, R::one(), n2));
    (
        select(pole, R::one(), x * k),
        select(pole, R::zero(), y * k),
    )
}

/// `R(−φ)·v` with `c = cos φ`, `s = sin φ`: `(c·v_x + s·v_y, c·v_y − s·v_x)`, each an explicit `fma`.
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

/// The exponent [`Wide`] gives zero: a sentinel meaning "no value", not a tuned number. Any value below the least
/// exponent `Wide` can produce serves (its longest product has nine factors, so about `−9700` at f64's least subnormal
/// exponent): a zero is then the smallest value in every comparison and alignment. `i16`'s bound is a choice of
/// meaning, a register-free sentinel (dd_generation_root §3.8, "Reading a constant"), as `decode_dispatch`'s
/// `UNWRITTEN = u32::MAX` is. No arithmetic on it overflows `i32`: a product's exponent sum is formed from two
/// operands, each a zero's sentinel or a real exponent, and a zero result is set back to the sentinel, so the least sum
/// formed is `2·i16::MIN − 1023` (two zeros, renormalised with f64's zero field); the alignment gap is at most about
/// `9216 − i16::MIN`; both are within `±10^5`.
const ZERO_EXPONENT: i32 = i16::MIN as i32;

/// A number `m·2^e` with its exponent carried as an integer: `|m|` in `[1, 2)`, or `m = 0` with `e` the
/// [`ZERO_EXPONENT`]. The mirror decision's arithmetic, so that no float it rounds lies outside `[2^−60, 4)` in
/// magnitude, and none is subnormal on any backend, whatever the state's scale or its terms' spread.
#[derive(Clone, Copy)]
struct Wide<R> {
    m: R,
    e: i32,
}

/// `x·2^k` normalised, for `x` 0 or a normal float within `[2^−60, 4)` in magnitude: its exponent read from the bits,
/// `x` scaled by its inverse, exact.
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
/// cancellation leaves an exact difference of at least `2^−24` (f32; `2^−53` at f64), normal.
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

/// `a > b` for `a, b ≥ 0`: by exponent, then by mantissa; a zero, at the [`ZERO_EXPONENT`], is below every other value.
#[inline]
fn wide_greater<R: LinkReal + IeeeBits>(a: Wide<R>, b: Wide<R>) -> bool {
    (a.e > b.e) | ((a.e == b.e) & (a.m > b.m))
}

/// `‖v‖² = v_x² + v_y²`.
#[inline]
fn wide_norm2<R: LinkReal + IeeeBits>(v: [Wide<R>; 2]) -> Wide<R> {
    wide_add(wide_mul(v[0], v[0]), wide_mul(v[1], v[1]))
}

/// Step 3's decision (R-82), on the positions `r` in any frame, before step 0 (the module's note):
/// 1. the pair differences `ρ = r₁ − r₀`, `d02 = r₂ − r₀`, `d12 = r₂ − r₁`, each a lone subtraction, and every
///    component and mass taken as a `Wide`, its exponent carried as an integer;
/// 2. `Λ = m₀d02 + m₁d12` (`= M₀₁r₂ − m₀r₀ − m₁r₁`), `c = ρ × Λ = ρ_xΛ_y − ρ_yΛ_x` with `ρ` read as `(1, 0)` where it is
///    0 (step 2's guard, on the same `ρ`), `d² = Σ_{i<j} mᵢmⱼ‖dᵢⱼ‖²`, and mirror iff `c < 0` and
///    `m₂c² > δ_λ²·M₀₁·‖ρ‖²·d²`, compared by exponent, then mantissa.
///
/// Every float the decision rounds is a mantissa product in `[1, 4)` (an [`IeeeBits::product`], its partial products
/// at least `2^−46`), an aligned sum of mantissas scaled by at most `2^−60`, or a power-of-two scaling into `[1, 2)`:
/// none is subnormal and none overflows, for any state whose coordinates and pair differences are 0 or normal. So no
/// backend rounds it differently, whether it fuses, contracts or flushes subnormals, and the same input takes the same
/// branch everywhere.
#[inline]
pub fn mirror_decision<R: LinkReal + IeeeBits>(m: [R; 3], r: [[R; 2]; 3]) -> bool {
    let diff = |a: [R; 2], b: [R; 2]| [b[0] - a[0], b[1] - a[1]];
    let (rho, d02, d12) = (diff(r[0], r[1]), diff(r[0], r[2]), diff(r[1], r[2]));
    let pole = coincident(rho);
    let vector = |v: [R; 2]| [wide(v[0]), wide(v[1])];
    let axis = vector([
        select(pole, R::one(), rho[0]),
        select(pole, R::zero(), rho[1]),
    ]);
    let (rho, d02, d12) = (vector(rho), vector(d02), vector(d12));
    let mass = [wide(m[0]), wide(m[1]), wide(m[2])];
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

/// `(v_x, −v_y)` where `flip` holds, else `v`, selected.
#[inline]
fn mirror<R: LinkReal>(v: [R; 2], flip: bool) -> [R; 2] {
    [v[0], select(flip, -v[1], v[1])]
}

/// `C`: `ic` onto the canonical section, in R-23's order (the module's note). `ic.m` is positive with `Σmᵢ = 1`.
#[inline]
pub fn canonicalise<R: LinkReal + IeeeBits>(ic: &Ic<R>) -> Canonical<R> {
    let m = ic.m;
    // 1a: the CoM frame.
    let com = centre_of_mass(m, ic.r);
    let centred = [
        [ic.r[0][0] - com[0], ic.r[0][1] - com[1]],
        [ic.r[1][0] - com[0], ic.r[1][1] - com[1]],
        [ic.r[2][0] - com[0], ic.r[2][1] - com[1]],
    ];
    // 0: I = 1, from the CoM-frame I.
    let i_in = inertia(m, centred);
    // Step 2's guard and angle and step 3's decision, read off the input's differences before step 0's division (the
    // module's note): `ρ = 0`, `φ` and the mirror test are translation- and scale-free in exact arithmetic.
    let rho = [ic.r[1][0] - ic.r[0][0], ic.r[1][1] - ic.r[0][1]];
    let clamped = coincident(rho);
    let phi = rotation_angle(rho);
    let mirrored = mirror_decision(m, ic.r);
    let (r, p, lambda) = similarity(centred, ic.p, i_in);
    // 1b: zero total momentum.
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
    let (c, s) = rotation(rho);
    let r = [rotate(r[0], c, s), rotate(r[1], c, s), rotate(r[2], c, s)];
    let p = [rotate(p[0], c, s), rotate(p[1], c, s), rotate(p[2], c, s)];
    // 3: the mirror, in encode's frame, as decided above; λ̃_y recorded.
    let lambda_t_y = lambda_t_y(m, r);
    Canonical {
        ic: Ic {
            m,
            r: [
                mirror(r[0], mirrored),
                mirror(r[1], mirrored),
                mirror(r[2], mirrored),
            ],
            p: [
                mirror(p[0], mirrored),
                mirror(p[1], mirrored),
                mirror(p[2], mirrored),
            ],
        },
        i_in,
        lambda,
        phi,
        lambda_t_y,
        clamped,
        mirrored,
    }
}

/// The scale gauge (dd_decoder §3.5): `ℓ = √I`, `rᵢ ← rᵢ/ℓ`, `pᵢ ← √ℓ·pᵢ`, `I = Σmᵢ‖rᵢ‖²` of `ic` as given (CoM at the
/// origin, as the decode emits it), through step 0's [`similarity`].
#[inline]
pub fn scale_gauge<R: LinkReal>(ic: &Ic<R>) -> Ic<R> {
    let (r, p, _) = similarity(ic.r, ic.p, inertia(ic.m, ic.r));
    Ic { m: ic.m, r, p }
}
