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
//!   range rescale, below).
//! - **`I_in` rounding to `+∞`** (f64 above about `1e154` per coordinate, f32 above about `1e19`): `√I_in = +∞`, so
//!   `r/√I_in` is `±0`, `λ = 1/∞ = 0`, and `I_in^{1/4}·p` is `±∞`, or NaN for a zero component; step 1b's sums and the
//!   rotation then make every momentum NaN on the CPU. `ρ` and the mirror are read from the input's differences before
//!   the division (below), so `lookup_clamped` is raised iff `ρ = r₁ − r₀` is 0, `φ` is that `ρ`'s angle, and the
//!   mirror is R-82's on the state's own `λ̃_y` (the decision's range rescale, below). On the GPU this arithmetic on `∞`
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
//! - **The range rescale.** `c²` and `‖ρ‖²d²` are both of degree 4 in the state's scale, so the test is unchanged by
//!   scaling the differences by `2^−k`, which [`mirror_decision`] does first, with `k` read from their exponent bits
//!   so that the largest component is in `[1, 2)`. A power-of-two product is exact wherever its result is normal, so
//!   for any state with finite differences the test neither overflows nor underflows: at f32 a state of size `1e18` or
//!   `1e-18`, and at f64 one of `1e150` or `1e-150`, takes R-82's branch, which the unscaled form loses (`m₂c²` and
//!   `δ_λ²M₀₁‖ρ‖²d²` both `+∞`, or both rounded to 0), and a GPU that flushes subnormals sees the same normal numbers as
//!   one that keeps them.
//! - **The products** ([`IeeeBits::product`]). At f32 every product is formed from its operands' bit-split halves,
//!   whose partial products are exact, summed in one fixed order: a backend that fuses or contracts any of its sites
//!   rounds the same exact sums, so the product's bits are the same everywhere. Every other operation is a lone
//!   subtraction or addition, an integer operation on bits, a select or a comparison.
//!
//! So `|λ̃_y| ≤ δ_λ`, `−δ_λ` itself included, is no mirror, and one input takes one branch on every IEEE backend that
//! keeps its additions and multiplications correctly rounded and unreassociated (fast-math off, R-84), fused or not.
//! The one gap left is a partial product below the smallest normal, where a GPU that flushes subnormals drops it: a
//! component more than about `2^−100` (f32) below the state's largest difference. [`Canonical::lambda_t_y`] is formed
//! after the rotation and read, not branched on; within rounding of `−δ_λ` it may disagree with the flag. The
//! cross-backend claim is for identical inputs only (PIT-10).
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
    let m01 = Float::mul_add(m[1], R::one(), m[0]);
    let inner = Float::mul_add(m[1], r[1][1], m[0] * r[0][1]) / m01;
    Float::sqrt(m[2] * m01) * (r[2][1] - inner)
}

/// R-82's test as it reads, on a given `λ̃_y`: mirror iff `λ̃_y < −δ_λ`; `|λ̃_y| ≤ δ_λ` is no mirror. The reference
/// [`mirror_decision`] equals in exact arithmetic.
#[inline]
pub fn mirror_taken<R: LinkReal>(lambda_t_y: R) -> bool {
    lambda_t_y < -R::DELTA_LAMBDA
}

/// `d²`'s three pair terms' sum, `Σ_{i<j} mᵢmⱼ‖rⱼ − rᵢ‖²` over the differences `d01 = r₁ − r₀`, `d02 = r₂ − r₀`,
/// `d12 = r₂ − r₁`: the CoM-frame moment of inertia when `Σmᵢ = 1`, from differences alone, every product an
/// [`IeeeBits::product`].
#[inline]
fn pair_sum<R: LinkReal + IeeeBits>(m: [R; 3], d01: [R; 2], d02: [R; 2], d12: [R; 2]) -> R {
    let sq = |v: [R; 2]| v[0].product(v[0]) + v[1].product(v[1]);
    let n01 = m[0].product(m[1]).product(sq(d01));
    let n02 = m[0].product(m[2]).product(sq(d02));
    let n12 = m[1].product(m[2]).product(sq(d12));
    (n01 + n02) + n12
}

/// The CoM-frame moment of inertia of `r` under `m` (`Σmᵢ = 1`), as [`mirror_decision`] forms it, from the pair
/// differences: `Σ_{i<j} mᵢmⱼ‖rⱼ − rᵢ‖²`, before the decision's rescale.
#[inline]
pub fn pair_inertia<R: LinkReal + IeeeBits>(m: [R; 3], r: [[R; 2]; 3]) -> R {
    let diff = |a: [R; 2], b: [R; 2]| [b[0] - a[0], b[1] - a[1]];
    pair_sum(m, diff(r[0], r[1]), diff(r[0], r[2]), diff(r[1], r[2]))
}

/// Step 3's decision (R-82), on the positions `r` in any frame, before step 0 (the module's note):
/// 1. the pair differences `d01 = r₁ − r₀ = ρ`, `d02 = r₂ − r₀`, `d12 = r₂ − r₁`, each a lone subtraction;
/// 2. the range rescale: `k`, the greatest exponent field among their six components, read from the bits, and every
///    difference times `2^−k` (two powers of two formed from their bits, each a normal number), exact wherever the
///    result is normal, so the largest component is in `[1, 2)`;
/// 3. `Λ = m₀d02 + m₁d12` (`= M₀₁r₂ − m₀r₀ − m₁r₁`), `c = ρ × Λ = ρ_xΛ_y − ρ_yΛ_x` with `ρ` read as `(1, 0)` where it is
///    0 (step 2's identity), `d² = Σ_{i<j} mᵢmⱼ‖dᵢⱼ‖²` ([`pair_inertia`]'s), and mirror iff `c < 0` and
///    `m₂c² > δ_λ²·M₀₁·‖ρ‖²·d²`.
///
/// Every product is an [`IeeeBits::product`], exact in its partial products, and every other operation a lone
/// difference or sum, an integer operation on bits or a comparison: no `fma`, `√`, division or trigonometry. So no
/// backend can round it differently, fusing or not, contracting or not, and the same input takes the same branch
/// everywhere.
#[inline]
pub fn mirror_decision<R: LinkReal + IeeeBits>(m: [R; 3], r: [[R; 2]; 3]) -> bool {
    let diff = |a: [R; 2], b: [R; 2]| [b[0] - a[0], b[1] - a[1]];
    let (d01, d02, d12) = (diff(r[0], r[1]), diff(r[0], r[2]), diff(r[1], r[2]));
    let wider = |a: i32, b: i32| select(b > a, b, a);
    let k = wider(
        wider(
            wider(d01[0].exponent(), d01[1].exponent()),
            wider(d02[0].exponent(), d02[1].exponent()),
        ),
        wider(d12[0].exponent(), d12[1].exponent()),
    );
    let half = k / 2;
    let (s1, s2) = (R::pow2(-half), R::pow2(half - k));
    let shrink = |v: [R; 2]| [v[0] * s1 * s2, v[1] * s1 * s2];
    let (rho, d02, d12) = (shrink(d01), shrink(d02), shrink(d12));
    let d2 = pair_sum(m, rho, d02, d12);
    let pole = coincident(rho);
    let axis = [
        select(pole, R::one(), rho[0]),
        select(pole, R::zero(), rho[1]),
    ];
    let lam = |k: usize| m[0].product(d02[k]) + m[1].product(d12[k]);
    let c = axis[0].product(lam(1)) - axis[1].product(lam(0));
    let m01 = m[0] + m[1];
    let delta = R::DELTA_LAMBDA;
    let p = axis[0].product(axis[0]) + axis[1].product(axis[1]);
    let left = c.product(c).product(m[2]);
    let right = delta.product(delta).product(m01).product(p).product(d2);
    (c < R::zero()) & (left > right)
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
    // 2: ρ onto +x.
    let (s, c) = (Float::sin(phi), Float::cos(phi));
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
