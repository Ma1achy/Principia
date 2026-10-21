//! The invariant-momentum construction (chart_reference §2.2; dd_decoder §3.4; REQ-DEC-020, REQ-DEC-021, REQ-DEC-022):
//! the momentum stage of the `(L_z, E)` and `(L_z, K)` charts, which build `pᵢ` from the decoded masses and
//! configuration and the pixel's targets `L_z` and `K*`, one source for the f32 kernel and the f64 CPU path (R-265).
//!
//! ```text
//! (i)   ω = L_z/I            vᵢ⁽ᴸ⁾ = ω·J rᵢ          K_min = L_z²/(2I)           J(x, y) = (−y, x)
//! (ii)  seeds, in order:     (ρ̇, λ̇) = (ρ, 0), (0, λ), (Jρ, 0), (0, Jλ)
//!       v₀ = −(m₂/M)λ̇ − (m₁/M₀₁)ρ̇     v₁ = −(m₂/M)λ̇ + (m₀/M₀₁)ρ̇     v₂ = (M₀₁/M)λ̇
//!       w⁽¹⁾ᵢ = vᵢ − c/M, c = Σ mᵢvᵢ       w⁽²⁾ᵢ = w⁽¹⁾ᵢ − β_L·J rᵢ, β_L = L(w⁽¹⁾)/I, L(w) = Σ mᵢ(rᵢ × wᵢ)_z
//!       among the seeds with ‖w⁽²⁾‖²_m > ε_w the largest, ties by seed order; none → DEGENERATE (R-82)
//!       wᵢ = w⁽²⁾ᵢ/‖w⁽²⁾‖_m
//! (iii) K* < K_min → infeasible;  else a = √(2(K* − K_min)),  vᵢ = vᵢ⁽ᴸ⁾ + a·wᵢ,  pᵢ = mᵢvᵢ
//! ```
//!
//! **`I` and `M`** (dd_decoder §3.4, "with `I = 1` at the canonical scale"; [`crate::decode::units`]): the
//! configuration block emits `M = Σmᵢ = 1` and `I = Σmᵢ‖rᵢ‖² = 1`, so the construction reads both as the dimensionless
//! system's 1, not recomputed from the rounded positions: `ω = L_z`, `β_L = L(w⁽¹⁾)`, `K_min = L_z²/2`. The
//! configuration's own `I` is 1 to rounding, so `L_z(p)` and `K(p)` meet their targets to rounding, and the feasibility
//! branch reads only the pixel's own `(L_z, K*)`.
//!
//! **The seed rule** (R-82): among the seeds with `‖w⁽²⁾‖²_m > ε_w` (the chart constant `eps_w`,
//! [`crate::generated::constants`]) the largest, ties by seed order; none → `DEGENERATE`. [`choose`] is the rule on given
//! norms, compared squared, the order of the norms: a seed is taken over the one held only if its norm is strictly
//! larger, so an exact tie keeps the earlier seed.
//!
//! **The rule on a decoded configuration is decided on `z_α`** ([`decoded_seed`], [`construct`]; dd_decoder §2,
//! "branch decisions … bit-identical across backends", §5 test 12; R-34, R-84). The canonical-frame decode
//! (chart_reference §0.2) has `ρ̃ = cos α·(1, 0)`, `λ̃ = sin α·(cos β, sin β)`, `I = M = 1`. The Jacobi velocities
//! `(ρ̇, λ̇)` carry no CoM drift and `‖v‖²_m = μ_ρ‖ρ̇‖² + μ_λ‖λ̇‖²`, `L(v) = μ_ρ ρ×ρ̇ + μ_λ λ×λ̇`, so in exact arithmetic
//! the four norms are
//!
//! ```text
//! (ρ, 0):   ‖ρ̃‖² = cos²α          L = 0, nothing projected
//! (0, λ):   ‖λ̃‖² = sin²α          L = 0
//! (Jρ, 0):  cos²α·sin²α           L = cos²α:  w⁽²⁾ = (sin²α·Jρ, −cos²α·Jλ)
//! (0, Jλ):  cos²α·sin²α           L = sin²α:  w⁽²⁾ = (−sin²α·Jρ, cos²α·Jλ)
//! ```
//!
//! `cos²α·sin²α ≤ min(cos²α, sin²α)`, and where one of them is 0 the other is 1, so the last two seeds are never the
//! largest, and the largest is at least ½ > `ε_w`: a decoded configuration is never `DEGENERATE` by the seed rule. The
//! rule takes `(ρ, 0)` exactly where `cos²α ≥ sin²α`, `α ≤ π/4` (a tie keeps the earlier), else `(0, λ)`. Every
//! registered α link is `α_min + c·(1 + u(z_α))`, `c = π/4 − α_min`, `u` odd and strictly increasing (`σ(x) =
//! (1 + tanh(x/2))/2`; dd_generation_root §3.9's table), so `α ≤ π/4` exactly where `z_α ≤ 0`. That comparison reads
//! the control itself, an input, on its bits (subnormals included, as [`ExactSign::binade`] reads them), so one control
//! takes one seed at f32 and f64 and on every backend; comparing the
//! computed `cos²α` and `sin²α` would fork where they differ in their last bits, near `z_α = 0`, and the seeds there
//! are orthogonal, so the momenta would differ at O(1). [`construct_by_norms`], the rule on computed norms, serves a configuration not from the canonical decode, and its
//! choice near a tie is not claimed branch-identical.
//!
//! **The infeasible pixel** (chart_reference §2.2, R-72; REQ-DEC-040): `K* < K_min` is
//! `DEGENERATE(INVARIANT_INFEASIBLE)`, recorded as `decode_failed`, detail 3 (other) ([`infeasible_label`]); an earlier
//! block's tag (the mass block's `M01_TINY`) is kept over it, and it takes precedence over the seed rule's
//! `DEGENERATE(NO_QUALIFYING_SEED)`. Both are members of [`crate::decode::degenerate::Degenerate`]'s closed set
//! ([`Outcome::tag`]), and a chart's decode carries them in its `degenerate` ([`crate::decode::Blocks::tag`]). It is kept,
//! never dropped, and its momenta are finite: `a` is selected as 0, so `pᵢ = mᵢvᵢ⁽ᴸ⁾`, the rigid rotation. A
//! `DEGENERATE` pixel likewise keeps `w = 0`, selected, so nothing divides by its zero norm.
//!
//! **The feasibility branch** ([`infeasible`]; R-34, R-84; gpu_determinism_note rules 1, 2 and 6). `K* < L_z²/(2I)` is
//! decided as `2·I·K* < L_z²` in exact arithmetic, so it is the inequality itself, with no rounding at the decision, and
//! one input takes one branch at f32 and at f64 and on every IEEE backend, fused or not:
//! - **The operands' binades** ([`ExactSign::binade`]): each of `I`, `K*` and `L_z` is read from its bits as a sign, a
//!   zero flag and `s·2^e`, `s ∈ [1, 2)`, exact, subnormals included (a subnormal's fraction converts exactly to a
//!   normal float). Signs and zeros are read on the bits, so a GPU that flushes subnormals in its float comparisons
//!   reads the same input. A zero product `I·K*` makes the test `0 < L_z²`, a negative one makes it true; otherwise
//!   `2IK* − L_z² = 2^{2e_L}·(2^E·s_I·s_K − s_L²)`, `E = e_I + e_K + 1 − 2e_L` an integer. `s_I·s_K` and `s_L²` are both
//!   in `[1, 4)`, so `E ≥ 2` is feasible and `E ≤ −2` infeasible, decided on the integer `E`; `E ∈ {−1, 0, 1}` is the
//!   exact test below, at the power of two `2^E`.
//! - **The limbs** ([`ExactSign::limbs`]): each `s ∈ [1, 2)` is split at its bits into three limbs of at most `w` bits
//!   (`w = 8` at f32, 18 at f64), `s = s₀ + s₁ + s₂` exactly, `sₐ` on the grid `2^{−(a+1)w+1}`. Every limb product has at
//!   most `2w` bits, so it is exact, and none is below `2^{−6w}`, so every one is normal: a GPU that flushes subnormals
//!   flushes none of them.
//! - **The columns.** `Cₙ = 2^E·Σ_{a+b=n} s_{I,a}s_{K,b} − Σ_{a+b=n} s_{L,a}s_{L,b}`, `n = 0 … 4`, each exact: its terms
//!   share the grid `2^{−(n+2)w+1}` and it has at most `2w + 4` bits. Their sum `(((C₀ + C₁) + C₂) + C₃) + C₄`, added in
//!   that order, has the sign of `2^E·s_I·s_K − s_L²` exactly: while a partial sum is below `2^{6−(n+1)w}` the next
//!   addition is exact (at most `2w + 6` bits, 22 of f32's 24), and once it reaches that bound the columns still to come
//!   sum to less than half of it, so no later rounding can change the sign.
//!
//! Every product is exact and every other operation a lone addition or subtraction, an integer operation on bits, a
//! select or a comparison: no `fma` (lavapipe does not fuse an explicit one, RQ-298), no `√`, no division. A backend
//! that contracts or fuses any `a·b + c` here rounds the same exact product. The claim holds on every backend that
//! keeps its additions and multiplications correctly rounded and unreassociated (fast-math off, R-84), for identical
//! finite inputs (PIT-10); non-finite inputs are the decode's non-finite case, not this test's.
//!
//! **What is not a branch here.** `a` (formed as `√max(fma(−L_z, L_z, 2K*), 0)`), `w` and the seeds' norms are
//! continuous values; the branches are the feasibility test and, on a decoded configuration, `z_α ≤ 0`.

use spirv_std::glam::UVec3;
use spirv_std::num_traits::Float;
use spirv_std::spirv;

use crate::decode::config::{self, Config};
use crate::decode::degenerate::Degenerate;
use crate::decode::mass::{self, Masses};
use crate::generated::links::{Defaults, LinkReal};

/// `t` where `c` holds, else `f`: both operands are computed before the choice, so it is a select, not a branch.
#[inline]
fn select<T: Copy>(c: bool, t: T, f: T) -> T {
    if c {
        t
    } else {
        f
    }
}

/// A finite float as its bits read it ([`ExactSign::binade`]): `self = ±s·2^e` unless `zero`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Binade<R> {
    /// The sign bit is set (`−0` included).
    pub negative: bool,
    /// Every bit but the sign is clear: `±0`.
    pub zero: bool,
    /// The significand `s ∈ [1, 2)`, exact, subnormals normalised; 1 at zero.
    pub s: R,
    /// The exponent `e`, `|self| = s·2^e`.
    pub e: i32,
}

/// The bit-level steps of the exact feasibility test, per float type (the module's note).
pub trait ExactSign: LinkReal {
    /// The sign, the zero flag and `s·2^e` of a finite `self`, read from its bits, exact, subnormals included.
    fn binade(self) -> Binade<Self>;
    /// `[s₀, s₁, s₂]` of `self ∈ [1, 2)`, split at its bits: `self = s₀ + s₁ + s₂` exactly, `sₐ` a multiple of
    /// `2^{−(a+1)w+1}` below `2^{−aw+1}`, of at most `w` bits.
    fn limbs(self) -> [Self; 3];
    /// `2^e` from its bits, for a normal exponent `e`.
    fn two_to(e: i32) -> Self;
}

impl ExactSign for f32 {
    #[inline]
    fn binade(self) -> Binade<f32> {
        let bits = self.to_bits();
        let field = ((bits >> 23) & 0xFF) as i32;
        let fraction = bits & 0x007F_FFFF;
        // A subnormal is `fraction·2⁻¹⁴⁹`, and `fraction < 2²³` converts to f32 exactly, as a normal number.
        let lifted = (fraction as f32).to_bits();
        let normal = field != 0;
        let significand = select(normal, fraction, lifted & 0x007F_FFFF);
        let e = select(normal, field - 127, ((lifted >> 23) as i32) - 276);
        Binade {
            negative: bits >> 31 != 0,
            zero: bits & 0x7FFF_FFFF == 0,
            // `significand` holds fraction bits only, so adding 1.0's bits sets the exponent field to 1.0's.
            s: f32::from_bits(significand + 0x3F80_0000),
            e,
        }
    }

    #[inline]
    fn limbs(self) -> [f32; 3] {
        let bits = self.to_bits();
        let high = f32::from_bits(bits & 0xFFFF_0000);
        let upper = f32::from_bits(bits & 0xFFFF_FF00);
        [high, upper - high, self - upper]
    }

    #[inline]
    fn two_to(e: i32) -> f32 {
        f32::from_bits(((e + 127) as u32) << 23)
    }
}

impl ExactSign for f64 {
    #[inline]
    fn binade(self) -> Binade<f64> {
        let bits = self.to_bits();
        let field = ((bits >> 52) & 0x7FF) as i32;
        let fraction = bits & 0x000F_FFFF_FFFF_FFFF;
        // A subnormal is `fraction·2⁻¹⁰⁷⁴`, and `fraction < 2⁵²` converts to f64 exactly, as a normal number.
        let lifted = (fraction as f64).to_bits();
        let normal = field != 0;
        let significand = select(normal, fraction, lifted & 0x000F_FFFF_FFFF_FFFF);
        let e = select(normal, field - 1023, ((lifted >> 52) as i32) - 2097);
        Binade {
            negative: bits >> 63 != 0,
            zero: bits & 0x7FFF_FFFF_FFFF_FFFF == 0,
            // `significand` holds fraction bits only, so adding 1.0's bits sets the exponent field to 1.0's.
            s: f64::from_bits(significand + 0x3FF0_0000_0000_0000),
            e,
        }
    }

    #[inline]
    fn limbs(self) -> [f64; 3] {
        let bits = self.to_bits();
        let high = f64::from_bits(bits & 0xFFFF_FFF8_0000_0000);
        let upper = f64::from_bits(bits & 0xFFFF_FFFF_FFFE_0000);
        [high, upper - high, self - upper]
    }

    #[inline]
    fn two_to(e: i32) -> f64 {
        f64::from_bits(((e + 1023) as u64) << 52)
    }
}

/// The sign of `2^E·x·y − z²` for `x, y, z ∈ [1, 2)`, `scale = 2^E`, `E ∈ {−1, 0, 1}`, as a float of that sign (exactly
/// 0 where it is 0): the columns' sum, every step exact or sign-preserving (the module's note).
#[inline]
fn columns<R: ExactSign>(scale: R, x: R, y: R, z: R) -> R {
    let [x0, x1, x2] = x.limbs();
    let [y0, y1, y2] = y.limbs();
    let [z0, z1, z2] = z.limbs();
    let c0 = scale * (x0 * y0) - z0 * z0;
    let c1 = scale * (x0 * y1 + x1 * y0) - (z0 * z1 + z1 * z0);
    let c2 = scale * ((x0 * y2 + x1 * y1) + x2 * y0) - ((z0 * z2 + z1 * z1) + z2 * z0);
    let c3 = scale * (x1 * y2 + x2 * y1) - (z1 * z2 + z2 * z1);
    let c4 = scale * (x2 * y2) - z2 * z2;
    (((c0 + c1) + c2) + c3) + c4
}

/// Whether `2·i·k < l²`, in exact arithmetic, for finite `i`, `k`, `l`: `K* < L_z²/(2I)` at `I = i > 0`, `K* = k`,
/// `L_z = l` (the module's note). One input gives one answer at f32 and f64 and on every backend.
#[inline]
pub fn infeasible<R: ExactSign>(i: R, k: R, l: R) -> bool {
    let (bi, bk, bl) = (i.binade(), k.binade(), l.binade());
    let flat = bi.zero | bk.zero;
    let negative = bi.negative != bk.negative;
    let moving = !bl.zero;
    let e = bi.e + bk.e + 1 - 2 * bl.e;
    let scale = select(e < 0, R::two_to(-1), select(e > 0, R::two_to(1), R::one()));
    let near = columns(scale, bi.s, bk.s, bl.s) < R::zero();
    let below = select(e > 1, false, select(e < -1, true, near));
    select(flat, moving, negative | (moving & below))
}

/// The label of an infeasible pixel, `DEGENERATE(INVARIANT_INFEASIBLE)`, as `(state, detail)` of payload §2's
/// `sample_descriptor`: `decode_failed` (5), detail 3 (other) (chart_reference §2.2; REQ-DEC-040).
#[inline]
pub fn infeasible_label() -> (u32, u32) {
    Degenerate::InvariantInfeasible.label()
}

/// `J v = (−v_y, v_x)`.
#[inline]
pub fn j<R: LinkReal>(v: [R; 2]) -> [R; 2] {
    [-v[1], v[0]]
}

/// The seeds, in their order, the tie-break order (chart_reference §2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Seed {
    /// `(ρ̇, λ̇) = (ρ, 0)`, the primary.
    Rho,
    /// `(0, λ)`.
    Lambda,
    /// `(Jρ, 0)`.
    JRho,
    /// `(0, Jλ)`.
    JLambda,
}

/// What the construction did with a pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Outcome {
    /// `K* ≥ K_min` and a seed qualified: `p` meets `L_z` and `K*`.
    Feasible,
    /// `K* < K_min`: labelled [`infeasible_label`], momenta the rigid rotation.
    Infeasible,
    /// Feasible, but no seed has `‖w⁽²⁾‖²_m > ε_w`: `DEGENERATE` (R-82), momenta the rigid rotation.
    Degenerate,
}

impl Outcome {
    /// The `DEGENERATE` reason the outcome raises: `INVARIANT_INFEASIBLE` where infeasible, `NO_QUALIFYING_SEED` where
    /// no seed qualified, none where feasible.
    #[inline]
    pub fn tag(self) -> Option<Degenerate> {
        match self {
            Outcome::Feasible => None,
            Outcome::Infeasible => Some(Degenerate::InvariantInfeasible),
            Outcome::Degenerate => Some(Degenerate::NoQualifyingSeed),
        }
    }
}

/// The construction's output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Construction<R> {
    /// The particle momenta `pᵢ = mᵢvᵢ`.
    pub p: [[R; 2]; 3],
    /// The unit direction `w`, `‖w‖_m = 1`; 0 where no seed qualified.
    pub w: [[R; 2]; 3],
    /// The mixing amplitude `a`; 0 where infeasible or `DEGENERATE`.
    pub a: R,
    /// Each seed's `‖w⁽²⁾‖²_m`, in seed order.
    pub norms: [R; 4],
    /// The seed taken, `None` where none qualified.
    pub seed: Option<Seed>,
    /// What the construction did.
    pub outcome: Outcome,
}

/// The seed `(ρ̇, λ̇)` as particle velocities (chart_reference §2.2), `M = 1`.
#[inline]
pub fn velocities<R: LinkReal>(
    masses: &Masses<R>,
    rho_dot: [R; 2],
    lambda_dot: [R; 2],
) -> [[R; 2]; 3] {
    let [m0, m1, m2] = masses.m;
    let m01 = masses.m01;
    let (w0, w1) = (m1 / m01, m0 / m01);
    [
        [
            -(m2 * lambda_dot[0]) - w0 * rho_dot[0],
            -(m2 * lambda_dot[1]) - w0 * rho_dot[1],
        ],
        [
            -(m2 * lambda_dot[0]) + w1 * rho_dot[0],
            -(m2 * lambda_dot[1]) + w1 * rho_dot[1],
        ],
        [m01 * lambda_dot[0], m01 * lambda_dot[1]],
    ]
}

/// `L(w) = Σ mᵢ(rᵢ × wᵢ)_z`.
#[inline]
pub fn angular<R: LinkReal>(m: [R; 3], r: &[[R; 2]; 3], w: &[[R; 2]; 3]) -> R {
    let cross = |a: [R; 2], b: [R; 2]| a[0] * b[1] - a[1] * b[0];
    m[0] * cross(r[0], w[0]) + m[1] * cross(r[1], w[1]) + m[2] * cross(r[2], w[2])
}

/// `‖w‖²_m = Σ mᵢ‖wᵢ‖²`.
#[inline]
pub fn mass_norm2<R: LinkReal>(m: [R; 3], w: &[[R; 2]; 3]) -> R {
    let sq = |v: [R; 2]| v[0] * v[0] + v[1] * v[1];
    m[0] * sq(w[0]) + m[1] * sq(w[1]) + m[2] * sq(w[2])
}

/// `w⁽²⁾` of the velocities `v` at the positions `r`: the CoM drift `c/M`, then the rigid rotation `β_L·J rᵢ`,
/// projected out, `M = I = 1` (the module's note).
#[inline]
pub fn project<R: LinkReal>(m: [R; 3], r: &[[R; 2]; 3], v: &[[R; 2]; 3]) -> [[R; 2]; 3] {
    let c = [
        m[0] * v[0][0] + m[1] * v[1][0] + m[2] * v[2][0],
        m[0] * v[0][1] + m[1] * v[1][1] + m[2] * v[2][1],
    ];
    let w1 = [
        [v[0][0] - c[0], v[0][1] - c[1]],
        [v[1][0] - c[0], v[1][1] - c[1]],
        [v[2][0] - c[0], v[2][1] - c[1]],
    ];
    let beta = angular(m, r, &w1);
    let off = |w: [R; 2], ri: [R; 2]| {
        let jr = j(ri);
        [w[0] - beta * jr[0], w[1] - beta * jr[1]]
    };
    [off(w1[0], r[0]), off(w1[1], r[1]), off(w1[2], r[2])]
}

/// One step of [`choose`]: `(best, held, found)` after considering `seed`, of norm `n`: taken where it is above `eps`
/// and either nothing is held or it is strictly larger than the norm held.
#[inline]
fn consider<R: LinkReal>(state: (Seed, R, bool), seed: Seed, n: R, eps: R) -> (Seed, R, bool) {
    let (best, held, found) = state;
    let take = (n > eps) & (!found | (n > held));
    (
        select(take, seed, best),
        select(take, n, held),
        found | take,
    )
}

/// The seed rule (R-82) on the seeds' `‖w⁽²⁾‖²_m`, in seed order: among those above `ε_w`, the largest; an exact tie
/// keeps the earlier; `None` where none is above `ε_w`.
#[inline]
pub fn choose<R: LinkReal>(norms: [R; 4]) -> Option<Seed> {
    let eps = R::EPS_W;
    let state = (Seed::Rho, norms[0], norms[0] > eps);
    let state = consider(state, Seed::Lambda, norms[1], eps);
    let state = consider(state, Seed::JRho, norms[2], eps);
    let (best, _, found) = consider(state, Seed::JLambda, norms[3], eps);
    select(found, Some(best), None)
}

/// The seed R-82's rule takes on a configuration of the canonical-frame decode, from its configuration control `z_α`:
/// `(ρ, 0)` where `z_α ≤ 0`, else `(0, λ)` (the module's note). `z_α ≤ 0` is read on its bits, the sign bit set or every
/// other bit clear, as the feasibility test reads its signs: a GPU that flushes a subnormal `z_α` to 0 in its float
/// comparisons reads the same input.
#[inline]
pub fn decoded_seed<R: ExactSign>(z_alpha: R) -> Seed {
    let b = z_alpha.binade();
    select(b.negative | b.zero, Seed::Rho, Seed::Lambda)
}

/// The seeds' `w⁽²⁾`, in seed order, and their `‖w⁽²⁾‖²_m`.
#[inline]
fn seeds<R: LinkReal>(masses: &Masses<R>, config: &Config<R>) -> ([[[R; 2]; 3]; 4], [R; 4]) {
    let (m, r) = (masses.m, config.r);
    let none = [R::zero(); 2];
    let (rho, lambda) = (config.rho, config.lambda);
    let w = [
        project(m, &r, &velocities(masses, rho, none)),
        project(m, &r, &velocities(masses, none, lambda)),
        project(m, &r, &velocities(masses, j(rho), none)),
        project(m, &r, &velocities(masses, none, j(lambda))),
    ];
    let norms = [
        mass_norm2(m, &w[0]),
        mass_norm2(m, &w[1]),
        mass_norm2(m, &w[2]),
        mass_norm2(m, &w[3]),
    ];
    (w, norms)
}

/// The construction with the seed taken (`None`: none qualified), from the seeds' fields and norms.
#[inline]
fn finish<R: ExactSign>(
    masses: &Masses<R>,
    config: &Config<R>,
    (seeds, norms): ([[[R; 2]; 3]; 4], [R; 4]),
    seed: Option<Seed>,
    lz: R,
    k_star: R,
) -> Construction<R> {
    let m = masses.m;
    let r = config.r;
    let zero = R::zero();
    let none = [zero, zero];
    let (w, n2) = match seed {
        Some(Seed::Rho) => (seeds[0], norms[0]),
        Some(Seed::Lambda) => (seeds[1], norms[1]),
        Some(Seed::JRho) => (seeds[2], norms[2]),
        Some(Seed::JLambda) => (seeds[3], norms[3]),
        None => ([none, none, none], R::one()),
    };
    let norm = Float::sqrt(n2);
    let unit = |v: [R; 2]| [v[0] / norm, v[1] / norm];
    let w = [unit(w[0]), unit(w[1]), unit(w[2])];
    let short = infeasible(R::one(), k_star, lz);
    let outcome = select(
        short,
        Outcome::Infeasible,
        select(seed.is_none(), Outcome::Degenerate, Outcome::Feasible),
    );
    let a2 = Float::mul_add(-lz, lz, k_star + k_star);
    let a = select(
        outcome == Outcome::Feasible,
        Float::sqrt(Float::max(a2, zero)),
        zero,
    );
    let body = |mi: R, ri: [R; 2], wi: [R; 2]| {
        let jr = j(ri);
        [mi * (lz * jr[0] + a * wi[0]), mi * (lz * jr[1] + a * wi[1])]
    };
    Construction {
        p: [
            body(m[0], r[0], w[0]),
            body(m[1], r[1], w[1]),
            body(m[2], r[2], w[2]),
        ],
        w,
        a,
        norms,
        seed,
        outcome,
    }
}

/// The invariant-momentum construction (chart_reference §2.2), the decoder's, on a configuration of the canonical-frame
/// decode from the configuration control `z_alpha`: the momenta realising `L_z = lz` and `K = k_star`, the seed R-82's,
/// decided on `z_α` ([`decoded_seed`]), so one input takes one seed on every backend (the module's note).
#[inline]
pub fn construct<R: ExactSign>(
    masses: &Masses<R>,
    config: &Config<R>,
    z_alpha: R,
    lz: R,
    k_star: R,
) -> Construction<R> {
    construct_seeded(masses, config, decoded_seed(z_alpha), lz, k_star)
}

/// The invariant-momentum construction (chart_reference §2.2) with the seed `seed` given: the seed R-82's rule takes,
/// decided by the chart on an input of its own so that one input takes one seed on every backend, as [`construct`]
/// decides it on `z_α` (the module's note); the Burrau `(ν, K)` chart decides it on `ν`
/// (`crate::chart::burrau::seed`).
#[inline]
pub fn construct_seeded<R: ExactSign>(
    masses: &Masses<R>,
    config: &Config<R>,
    seed: Seed,
    lz: R,
    k_star: R,
) -> Construction<R> {
    let fields = seeds(masses, config);
    finish(masses, config, fields, Some(seed), lz, k_star)
}

/// The invariant-momentum construction (chart_reference §2.2) on a configuration not from the canonical-frame decode:
/// the seed R-82's rule ([`choose`]) on the computed norms, `DEGENERATE` where none qualifies. Its choice near a tie is
/// not claimed branch-identical; a decoded configuration takes [`construct`] (the module's note).
#[inline]
pub fn construct_by_norms<R: ExactSign>(
    masses: &Masses<R>,
    config: &Config<R>,
    lz: R,
    k_star: R,
) -> Construction<R> {
    let fields = seeds(masses, config);
    let seed = choose(fields.1);
    finish(masses, config, fields, seed, lz, k_star)
}

/// The word of an outcome: 0 feasible, 1 infeasible, 2 `DEGENERATE`.
#[inline]
pub fn outcome_word(outcome: Outcome) -> u32 {
    match outcome {
        Outcome::Feasible => 0,
        Outcome::Infeasible => 1,
        Outcome::Degenerate => 2,
    }
}

/// The word of a seed: its index in seed order, 4 for none.
#[inline]
pub fn seed_word(seed: Option<Seed>) -> u32 {
    match seed {
        Some(Seed::Rho) => 0,
        Some(Seed::Lambda) => 1,
        Some(Seed::JRho) => 2,
        Some(Seed::JLambda) => 3,
        None => 4,
    }
}

/// The f32 whose bits are word `at` of `words`. A function, not a closure: no closure holds the slice, so the kernel's
/// SPIR-V keeps no slice pointer in a struct, which naga refuses.
#[inline]
fn word(words: &[u32], at: usize) -> f32 {
    f32::from_bits(words[at])
}

/// [`infeasible`] at f32 on given inputs, so a test reads the GPU's branch: invocation `n` reads `I`, `K*` and `L_z`,
/// the bits at words `3n … 3n + 2` of `input`, and writes 1 at `output[n]` where the pixel is infeasible, else 0; an
/// invocation past the last triple writes nothing.
#[spirv(compute(threads(64)))]
pub fn feasibility(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] output: &mut [u32],
) {
    let n = id.x as usize;
    if n < input.len() / 3 {
        let at = 3 * n;
        let short = infeasible(word(input, at), word(input, at + 1), word(input, at + 2));
        output[n] = select(short, 1, 0);
    }
}

/// The words of one [`construct_defaults`] sample's input: `z_μ₁ z_μ₂ z_α z_β L_z K*`.
#[inline]
pub fn construct_in_words() -> usize {
    6
}

/// The words of one [`construct_defaults`] sample's output: `p₀ p₁ p₂` (each `x` then `y`), the outcome's word and the
/// seed's ([`outcome_word`], [`seed_word`]).
#[inline]
pub fn construct_out_words() -> usize {
    8
}

/// The decoder's construction at f32 under the default links, so a test reads the GPU's: invocation `n` reads sample
/// `n`'s [`construct_in_words`] words of `input` (f32 bits), decodes the masses and configuration from them, constructs
/// ([`construct`], its seed decided on `z_α`), and writes [`construct_out_words`] words at sample `n` of
/// `output`; an invocation past the last sample writes nothing.
#[spirv(compute(threads(64)))]
pub fn construct_defaults(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] output: &mut [u32],
) {
    let n = id.x as usize;
    if n < input.len() / construct_in_words() {
        let at = construct_in_words() * n;
        let masses = mass::decode::<Defaults, f32>([word(input, at), word(input, at + 1)]);
        let z_cfg = [word(input, at + 2), word(input, at + 3)];
        let config = config::decode::<Defaults, f32>(z_cfg, &masses);
        let (lz, k_star) = (word(input, at + 4), word(input, at + 5));
        let c = construct(&masses, &config, z_cfg[0], lz, k_star);
        let out = construct_out_words() * n;
        output[out] = c.p[0][0].to_bits();
        output[out + 1] = c.p[0][1].to_bits();
        output[out + 2] = c.p[1][0].to_bits();
        output[out + 3] = c.p[1][1].to_bits();
        output[out + 4] = c.p[2][0].to_bits();
        output[out + 5] = c.p[2][1].to_bits();
        output[out + 6] = outcome_word(c.outcome);
        output[out + 7] = seed_word(c.seed);
    }
}
