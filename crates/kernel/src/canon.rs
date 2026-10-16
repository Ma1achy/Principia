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
//!   about `1e-19` where a GPU flushes subnormals): the same select, so `r` is left unscaled, at `I ≈ 0`, not 1, every momentum
//!   becomes `0·p` (`√√0 = 0`) and `λ = 1`; since `ρ ≠ 0`, no `lookup_clamped`. The state leaves `C` off the section,
//!   unflagged.
//! - **`I_in` rounding to `+∞`** (f64 above about `1e154` per coordinate, f32 above about `1e19`): `√I_in = +∞`, so
//!   `r/√I_in` is `±0`, `λ = 1/∞ = 0`, and `I_in^{1/4}·p` is `±∞`, or NaN for a zero component; step 1b's sums and the
//!   rotation then make every momentum NaN on the CPU. `ρ` and the mirror are read before the division (below), so
//!   `lookup_clamped` is raised iff the CoM-frame `ρ` is 0, `φ` is that `ρ`'s angle, and the mirror is never taken (the
//!   test's right side is `+∞`). On the GPU this arithmetic on `∞` is not pinned (WGSL may assume there are no
//!   infinities; R-436).
//!
//! **The branch inputs, read before step 0** (R-34, R-84; gpu_determinism_note rules 1, 2 and 6). Step 2's guard and
//! angle and step 3's decision are computed from the CoM-frame positions `x`, before step 0's division, in arithmetic
//! every backend rounds alike: correctly rounded differences and products and explicit `fma`s, with no `√`, division
//! or trigonometry, whose WGSL results carry ulp bounds, not correct rounding, so that a branch fed through them can
//! differ between backends on one input. Each is R-23's step in exact arithmetic, `d > 0` being step 0's divisor:
//! - **The guard and `φ`.** `ρ = x₁ − x₀` is 0 iff the scaled `r₁ − r₀ = ρ/d` is, and `atan2` is unchanged by the
//!   positive factor `1/d`.
//! - **The mirror** ([`mirror_decision`]). `R(−φ)` puts `ρ̂ = ρ/‖ρ‖` on `+x` and keeps cross products, so a vector's `y`
//!   after it is its cross product with `ρ̂`, `a × b = a_xb_y − a_yb_x`. In the scaled frame `λ = Λ/(M₀₁d)`, with
//!   `Λ = M₀₁x₂ − m₀x₀ − m₁x₁`; hence `λ̃_y = √(m₂M₀₁)·c/(‖ρ‖·M₀₁·d)`, `c = ρ × Λ`. The factor of `c` is positive, so
//!   `λ̃_y < −δ_λ` iff `c < 0` and `c² > δ_λ²·‖ρ‖²·d²·M₀₁/m₂`, that is `m₂c² > δ_λ²·M₀₁·‖ρ‖²·d²`: `δ_λ` carried into the
//!   units of `c²`. At `ρ = 0` step 2 is the identity and `λ̃_y = √(m₂M₀₁)·Λ_y/(M₀₁d)`, the same with `ρ` read as
//!   `(1, 0)`. `d² = I_in`, or 1 where `I_in = 0` (step 0's select, below). `M₀₁` is the one rounding of `m₀ + m₁`
//!   ([`lambda_t_y`]'s).
//!
//! So `|λ̃_y| ≤ δ_λ`, `−δ_λ` itself included, is no mirror, and one input takes one branch on every backend that keeps
//! an explicit `fma` fused (dd_decoder §3.1's premise) and keeps subnormals; a GPU that flushes them can differ only
//! where a difference or product in these tests is below the smallest normal (f32: about `1.2e-38`; `δ_λ² = 1e-24`, so
//! for `M₀₁‖ρ‖²` or `M₀₁‖ρ‖²I_in` below about `1e-14`). [`Canonical::lambda_t_y`] is formed after the rotation and
//! read, not branched on; within rounding of `−δ_λ` it may disagree with the flag. The cross-backend claim is for
//! identical inputs only (PIT-10).
//!
//! **The scale gauge** ([`scale_gauge`]; dd_decoder §3.5, chart_reference §0.5): `ℓ = √I`, `rᵢ ← rᵢ/ℓ`,
//! `pᵢ ← √ℓ·pᵢ`, the same similarity map as step 0 ([`similarity`]); a no-op, to rounding, on canonical output.

use spirv_std::num_traits::Float;

use crate::decode::Ic;
use crate::generated::links::LinkReal;

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

/// Step 3's decision (R-82), on the CoM-frame positions `x` before step 0 and `d2`, the square of step 0's divisor
/// (`I_in`, or 1 where `I_in = 0`): with `ρ = x₁ − x₀` (`(1, 0)` where `ρ = 0`, step 2's identity rotation),
/// `Λ = M₀₁x₂ − m₀x₀ − m₁x₁` and `c = ρ × Λ = ρ_xΛ_y − ρ_yΛ_x`, mirror iff `c < 0` and `m₂c² > δ_λ²·M₀₁·‖ρ‖²·d2`.
/// Every operation is a correctly rounded `fma`, product or difference, with no `√`, division or trigonometry, so
/// every backend that keeps an explicit `fma` fused takes the same branch on the same input (the module's note).
#[inline]
pub fn mirror_decision<R: LinkReal>(m: [R; 3], x: [[R; 2]; 3], d2: R) -> bool {
    let rho = [x[1][0] - x[0][0], x[1][1] - x[0][1]];
    let pole = coincident(rho);
    let axis = [
        select(pole, R::one(), rho[0]),
        select(pole, R::zero(), rho[1]),
    ];
    let m01 = Float::mul_add(m[1], R::one(), m[0]);
    let big =
        |k: usize| Float::mul_add(m01, x[2][k], -Float::mul_add(m[1], x[1][k], m[0] * x[0][k]));
    let c = Float::mul_add(axis[0], big(1), -(axis[1] * big(0)));
    let delta = R::DELTA_LAMBDA;
    (c < R::zero()) & (c * c * m[2] > delta * delta * m01 * norm2(axis) * d2)
}

/// `(v_x, −v_y)` where `flip` holds, else `v`, selected.
#[inline]
fn mirror<R: LinkReal>(v: [R; 2], flip: bool) -> [R; 2] {
    [v[0], select(flip, -v[1], v[1])]
}

/// `C`: `ic` onto the canonical section, in R-23's order (the module's note). `ic.m` is positive with `Σmᵢ = 1`.
#[inline]
pub fn canonicalise<R: LinkReal>(ic: &Ic<R>) -> Canonical<R> {
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
    // Step 2's guard and angle and step 3's decision, read off the CoM-frame state before step 0's division (the
    // module's note): `ρ = 0`, `φ` and the mirror test are scale-free in exact arithmetic.
    let rho = [centred[1][0] - centred[0][0], centred[1][1] - centred[0][1]];
    let clamped = coincident(rho);
    let phi = rotation_angle(rho);
    let mirrored = mirror_decision(m, centred, select(i_in == R::zero(), R::one(), i_in));
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
