//! The read-back of canon's and the decode's branches (TASK-M4-27; R-690; parity contract § 4, "The driver condition"):
//! one compute entry for canon and one for each decode variant, `stage_branch_read_*`, each emitting, per sample, every
//! branch's outcome and, for a branch input of kind `+ and ×`, the expression's own operands and the input's bits, both
//! sides of a comparison, a `Wide`'s mantissa and exponent both. The levels of R-690's test are judged on operands, bits
//! and outcome from one compiled entry, so all three come from here. It is a module of its own, not a `stage_decode_`
//! entry, because `decode_cross_backend_registry` takes every one of those for a decode variant it must run.
//!
//! **No parity-only physics.** Each branch's input and decision is a function its production site calls too
//! (`crate::canon`'s [`skip_input`](canon::skip_input) and [`mirror_input`](canon::mirror_input), `mass::m01_input`,
//! `invariant::feasibility_input`, `invariant::alpha_sign`, `burrau::seed_input`, the descriptor's poles). The CPU's
//! f32 record calls these same `write_*` functions, and the arbiter evaluates a branch input through them, at f32, at
//! given operands, so there is no second copy of any form.
//!
//! **The words** are fixed: [`CANON_WORDS`] per state, [`DECODE_WORDS`] per decode sample, laid out by the `*_AT`
//! constants. A canon sample leads with its operands, the `(m, r)` words as the entry read them, then per branch its
//! outcome and input bits. A decode branch has a `present` word first, 0 where the variant has no such branch or `η_E`
//! is off; its operands are the entry's own emitted values, `decode/mass.rs`'s the masses it emitted. A branch of kind
//! `other operation` (division, `√`, `sin`, `cos`, `atan2`, or a value reached through them) emits its outcome alone.
//! A kind is the expression's own, read at its own operands (R-690): `rho_angle`'s cross and dot are `+`, `−` and `×` on
//! the emitted `ρ̃`, `λ̃`, however those were reached, so their slots are of the first kind.
//!
//! **The module.** These entries are built with the rest of the kernel, and `cargo xtask build-kernel` parts them into
//! `target/spirv/branch_read.wgsl`: no runtime path runs them, so they stay out of the production `kernel.wgsl`.
//!
//! **Domain** (pitfalls §10): fixed inputs through two stages; no trajectory. A read-back entry is a different shader from
//! the production one, and what it shows holds for that entry's compile.

use spirv_std::glam::UVec3;
use spirv_std::spirv;

use core::mem::offset_of;

use crate::canon::{self, Wide};
use crate::chart::burrau::{seed_input, seed_of_sum};
use crate::chart::invariant::InvariantChart;
use crate::chart::mixed::{Alpha, ZMu1};
use crate::chart::shape_sphere::{inverse_pole, inverse_pq};
use crate::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, ChartDecode, EuclidPlane,
    InvariantLE, InvariantLK, Latent, MassSimplex, Mixed, ShapeSphere,
};
use crate::decode::invariant::{
    alpha_sign, feasibility_input, feasibility_short, seed_of_sign, seed_word,
};
use crate::decode::{descriptor, energy_norm, mass, Ic};
use crate::stage::decode::ic_words;
use crate::stage::decode::{
    read_ic, target, BurrauDefaults, BurrauThetaDeltaMDefaults, BurrauThetaKDefaults,
    BurrauThetaLzDefaults, DecodeUniforms, DecodeVariant, EuclidPlaneDefaults, InvariantLEDefaults,
    InvariantLKDefaults, LatentDefaults, LatentStickBreaking, MassSimplexDefaults,
    MixedMu1AlphaDefaults, ShapeSphereDefaults, UniformChart,
};

/// A `Wide`: mantissa, then exponent.
type WideWords = [u32; 2];
/// The outcome word of a slot, and its `present` word.
type OutcomeWord = u32;
/// `(m, r)`: `m₀ m₁ m₂`, then `r₀ r₁ r₂`, each `x` then `y`.
type CanonOperandWords = [u32; 9];
/// `|I_in − 1|` and `τ_I`, each a `Wide`, and the coincidence.
type SkipWords = ([WideWords; 2], OutcomeWord);
/// `ρ` and `d02`, four `Wide`s.
type CoincidenceWords = [WideWords; 4];
/// `ρ`, two `Wide`s.
type GuardWords = [WideWords; 2];
/// `c` and the two sides, three `Wide`s.
type MirrorWords = [WideWords; 3];
/// A scaled `ρ` component.
type AxisWords = [u32; 1];
/// `M01_TINY`'s operands: `m₀`, `m₁`.
type M01Operands = [u32; 2];
/// `M01_TINY`'s input: `M₀₁` and `ε`, each a `Wide`.
type M01Bits = [WideWords; 2];
/// `INVARIANT_INFEASIBLE`'s operands: `K*`, `L_z`.
type FeasibilityOperands = [u32; 2];
/// `INVARIANT_INFEASIBLE`'s input: the sign flags, `E` and the columns' sum.
type FeasibilityBits = [u32; 5];
/// One operand.
type OneOperand = [u32; 1];
/// A seed's input: `z_α`'s two sign reads, or `3ν`'s `s` and `e`.
type SeedBits = [u32; 2];
/// `rho_angle`'s operands: `ρ̃`, then `λ̃`, each `x` then `y`.
type AngleOperands = [u32; 4];
/// `rho_angle`'s input to one test: the cross or the dot.
type AngleBits = [u32; 1];
/// No words: an `other operation` slot holds its `present` and outcome alone.
type NoWords = [u32; 0];

/// A slot of canon's words: its outcome, then its input's bits.
#[repr(C)]
pub struct CanonSlot<B> {
    /// The outcome.
    pub outcome: OutcomeWord,
    /// The input's bits.
    pub bits: B,
}

/// One canon sample's words, whose shape gives the `*_AT` offsets below.
#[repr(C)]
pub struct CanonRead {
    /// The sample's `(m, r)`.
    pub operands: CanonOperandWords,
    /// The skip's slot; its outcome is `lookup_rescaled`.
    pub skip: CanonSlot<SkipWords>,
    /// The coincidence's slot.
    pub coincidence: CanonSlot<CoincidenceWords>,
    /// The guard `ρ = 0`'s slot; its outcome is `lookup_clamped`.
    pub guard: CanonSlot<GuardWords>,
    /// The mirror's slot; its outcome is `lookup_mirrored`.
    pub mirror: CanonSlot<MirrorWords>,
    /// `ρ_x`'s zero test's slot.
    pub rho_x: CanonSlot<AxisWords>,
    /// `ρ_y`'s zero test's slot.
    pub rho_y: CanonSlot<AxisWords>,
}

/// A slot of a decode's words: whether the branch ran, its outcome, then its own operands and its input's bits.
#[repr(C)]
pub struct DecodeSlot<O, B> {
    /// 1 where the branch ran on this sample.
    pub present: OutcomeWord,
    /// The outcome.
    pub outcome: OutcomeWord,
    /// The expression's operands.
    pub operands: O,
    /// The input's bits.
    pub bits: B,
}

/// An `other operation` slot: present, outcome.
pub type OtherSlot = DecodeSlot<NoWords, NoWords>;

/// A slot of `rho_angle`'s, of kind `+ and ×`: its `ρ̃`, `λ̃` and the product its test reads.
pub type AngleSlot = DecodeSlot<AngleOperands, AngleBits>;

/// One decode sample's words, whose shape gives the `*_AT` offsets below.
#[repr(C)]
pub struct DecodeRead {
    /// `M01_TINY`'s slot.
    pub m01: DecodeSlot<M01Operands, M01Bits>,
    /// `INVARIANT_INFEASIBLE`'s slot.
    pub feasibility: DecodeSlot<FeasibilityOperands, FeasibilityBits>,
    /// The seed on `z_α`'s slot; its outcome is the seed's word.
    pub alpha_seed: DecodeSlot<OneOperand, SeedBits>,
    /// The seed on `ν`'s slot; its outcome is the seed's word.
    pub nu_seed: DecodeSlot<OneOperand, SeedBits>,
    /// `ENERGY_INFEASIBLE`'s slot.
    pub energy_infeasible: OtherSlot,
    /// `ENERGY_UNSCALABLE`'s slot.
    pub energy_unscalable: OtherSlot,
    /// `rho_ratio`'s pole's slot (R-436).
    pub ratio_pole: OtherSlot,
    /// `V_0`'s pole's slot (R-436).
    pub collision_pole: OtherSlot,
    /// `virial_ratio`'s pole's slot (R-436).
    pub virial_pole: OtherSlot,
    /// `rho_angle`'s `ρ̃ × λ̃ = 0` slot (R-437).
    pub cross_zero: AngleSlot,
    /// `rho_angle`'s `ρ̃ · λ̃ = 0` slot (R-437).
    pub dot_zero: AngleSlot,
    /// `rho_angle`'s `ρ̃ · λ̃ < 0` slot.
    pub dot_negative: AngleSlot,
    /// The shape sphere inverse's `p = q = 0` slot (R-437).
    pub sphere_pole: OtherSlot,
}

/// The bytes of a word.
const WORD: usize = size_of::<u32>();

/// The words of a `T`.
const fn words<T>() -> usize {
    size_of::<T>() / WORD
}

/// The word `field` of `layout` starts at.
macro_rules! word_of {
    ($layout:ty, $field:ident) => {
        offset_of!($layout, $field) / WORD
    };
}

/// Canon's operand words.
pub const CANON_OPERANDS: usize = words::<CanonOperandWords>();
/// [`CanonRead::skip`].
pub const SKIP_AT: usize = word_of!(CanonRead, skip);
/// The skip's input words.
pub const SKIP_BITS: usize = words::<SkipWords>();
/// [`CanonRead::coincidence`].
pub const COINCIDENCE_AT: usize = word_of!(CanonRead, coincidence);
/// The coincidence's input words.
pub const COINCIDENCE_BITS: usize = words::<CoincidenceWords>();
/// [`CanonRead::guard`].
pub const GUARD_AT: usize = word_of!(CanonRead, guard);
/// The guard's input words.
pub const GUARD_BITS: usize = words::<GuardWords>();
/// [`CanonRead::mirror`].
pub const MIRROR_AT: usize = word_of!(CanonRead, mirror);
/// The mirror's input words.
pub const MIRROR_BITS: usize = words::<MirrorWords>();
/// [`CanonRead::rho_x`].
pub const RHO_X_AT: usize = word_of!(CanonRead, rho_x);
/// [`CanonRead::rho_y`].
pub const RHO_Y_AT: usize = word_of!(CanonRead, rho_y);
/// A scaled component's input words.
pub const AXIS_BITS: usize = words::<AxisWords>();
/// The words of one canon sample.
pub const CANON_WORDS: usize = words::<CanonRead>();

/// [`DecodeRead::m01`].
pub const M01_AT: usize = word_of!(DecodeRead, m01);
/// `M01_TINY`'s operand words, then its input words.
pub const M01_SHAPE: (usize, usize) = (words::<M01Operands>(), words::<M01Bits>());
/// [`DecodeRead::feasibility`].
pub const FEASIBILITY_AT: usize = word_of!(DecodeRead, feasibility);
/// `INVARIANT_INFEASIBLE`'s operand words, then its input words.
pub const FEASIBILITY_SHAPE: (usize, usize) =
    (words::<FeasibilityOperands>(), words::<FeasibilityBits>());
/// [`DecodeRead::alpha_seed`].
pub const ALPHA_SEED_AT: usize = word_of!(DecodeRead, alpha_seed);
/// The seed on `z_α`'s operand words, then its input words.
pub const ALPHA_SEED_SHAPE: (usize, usize) = (words::<OneOperand>(), words::<SeedBits>());
/// [`DecodeRead::nu_seed`].
pub const NU_SEED_AT: usize = word_of!(DecodeRead, nu_seed);
/// The seed on `ν`'s operand words, then its input words.
pub const NU_SEED_SHAPE: (usize, usize) = (words::<OneOperand>(), words::<SeedBits>());
/// [`DecodeRead::energy_infeasible`].
pub const ENERGY_INFEASIBLE_AT: usize = word_of!(DecodeRead, energy_infeasible);
/// [`DecodeRead::energy_unscalable`].
pub const ENERGY_UNSCALABLE_AT: usize = word_of!(DecodeRead, energy_unscalable);
/// [`DecodeRead::ratio_pole`].
pub const RATIO_POLE_AT: usize = word_of!(DecodeRead, ratio_pole);
/// [`DecodeRead::collision_pole`].
pub const COLLISION_POLE_AT: usize = word_of!(DecodeRead, collision_pole);
/// [`DecodeRead::virial_pole`].
pub const VIRIAL_POLE_AT: usize = word_of!(DecodeRead, virial_pole);
/// [`DecodeRead::cross_zero`].
pub const CROSS_ZERO_AT: usize = word_of!(DecodeRead, cross_zero);
/// [`DecodeRead::dot_zero`].
pub const DOT_ZERO_AT: usize = word_of!(DecodeRead, dot_zero);
/// [`DecodeRead::dot_negative`].
pub const DOT_NEGATIVE_AT: usize = word_of!(DecodeRead, dot_negative);
/// `rho_angle`'s operand words, then a test's input words.
pub const ANGLE_SHAPE: (usize, usize) = (words::<AngleOperands>(), words::<AngleBits>());
/// [`DecodeRead::sphere_pole`].
pub const SPHERE_POLE_AT: usize = word_of!(DecodeRead, sphere_pole);
/// The words of one decode sample.
pub const DECODE_WORDS: usize = words::<DecodeRead>();

/// 1 where `c` holds, else 0.
#[inline]
fn word(c: bool) -> u32 {
    if c {
        1
    } else {
        0
    }
}

/// `w`'s two words at word `at` of `out`.
#[inline]
fn put_wide(out: &mut [u32], at: usize, w: Wide<f32>) {
    let [m, e] = w.words();
    out[at] = m;
    out[at + 1] = e;
}

/// Writes canon's [`CANON_WORDS`] words for the state `(m, r)` from word `at` of `out`: the kernel's branch inputs and
/// decisions, as `crate::canon::canonicalise` makes them. The arbiter evaluates canon's branch inputs through this.
#[inline]
pub fn write_canon(out: &mut [u32], at: usize, m: [f32; 3], r: [[f32; 2]; 3]) {
    out[at] = m[0].to_bits();
    out[at + 1] = m[1].to_bits();
    out[at + 2] = m[2].to_bits();
    out[at + 3] = r[0][0].to_bits();
    out[at + 4] = r[0][1].to_bits();
    out[at + 5] = r[1][0].to_bits();
    out[at + 6] = r[1][1].to_bits();
    out[at + 7] = r[2][0].to_bits();
    out[at + 8] = r[2][1].to_bits();
    let (mass, x) = canon::wide_state(m, r);
    let (rho, d02, d12) = canon::pair_differences(x);
    let (turned, clamped) = canon::scaled_rho(rho);
    let coincident = clamped & canon::is_pole(d02);
    let (_, i) = canon::com_frame(mass, x);
    let sides = canon::skip_input(i);
    let rescaled = canon::step_runs(canon::skip_taken(sides), coincident);
    let at_skip = at + SKIP_AT;
    out[at_skip] = word(rescaled);
    put_wide(out, at_skip + 1, sides[0]);
    put_wide(out, at_skip + 3, sides[1]);
    out[at_skip + 5] = word(coincident);
    let at_coincidence = at + COINCIDENCE_AT;
    out[at_coincidence] = word(coincident);
    put_wide(out, at_coincidence + 1, rho[0]);
    put_wide(out, at_coincidence + 3, rho[1]);
    put_wide(out, at_coincidence + 5, d02[0]);
    put_wide(out, at_coincidence + 7, d02[1]);
    let at_guard = at + GUARD_AT;
    out[at_guard] = word(clamped);
    put_wide(out, at_guard + 1, rho[0]);
    put_wide(out, at_guard + 3, rho[1]);
    let input = canon::mirror_input(mass, rho, d02, d12);
    let at_mirror = at + MIRROR_AT;
    out[at_mirror] = word(canon::mirror_side(&input));
    put_wide(out, at_mirror + 1, input.c);
    put_wide(out, at_mirror + 3, input.left);
    put_wide(out, at_mirror + 5, input.right);
    out[at + RHO_X_AT] = word(canon::axis_zero(turned[0]));
    out[at + RHO_X_AT + 1] = turned[0].to_bits();
    out[at + RHO_Y_AT] = word(canon::axis_zero(turned[1]));
    out[at + RHO_Y_AT + 1] = turned[1].to_bits();
}

/// Writes `M01_TINY`'s slot from word `at` of `out` on the emitted masses `m₀`, `m₁`.
#[inline]
pub fn write_m01(out: &mut [u32], at: usize, m0: f32, m1: f32) {
    let sides = mass::m01_input(m0, m1);
    out[at] = 1;
    out[at + 1] = word(mass::m01_tiny(sides));
    out[at + 2] = m0.to_bits();
    out[at + 3] = m1.to_bits();
    put_wide(out, at + 4, sides[0]);
    put_wide(out, at + 6, sides[1]);
}

/// Writes `INVARIANT_INFEASIBLE`'s slot from word `at` of `out` on the targets `K*`, `L_z`, at the canonical `I = 1`.
#[inline]
pub fn write_feasibility(out: &mut [u32], at: usize, k: f32, lz: f32) {
    let f = feasibility_input(1.0, k, lz);
    out[at] = 1;
    out[at + 1] = word(feasibility_short(&f));
    out[at + 2] = k.to_bits();
    out[at + 3] = lz.to_bits();
    out[at + 4] = word(f.flat);
    out[at + 5] = word(f.negative);
    out[at + 6] = word(f.moving);
    out[at + 7] = f.e as u32;
    out[at + 8] = f.columns.to_bits();
}

/// Writes the slot of the seed on `z_α` from word `at` of `out`.
#[inline]
pub fn write_alpha_seed(out: &mut [u32], at: usize, z_alpha: f32) {
    let (negative, zero) = alpha_sign(z_alpha);
    out[at] = 1;
    out[at + 1] = seed_word(Some(seed_of_sign(negative, zero)));
    out[at + 2] = z_alpha.to_bits();
    out[at + 3] = word(negative);
    out[at + 4] = word(zero);
}

/// Writes the slot of the seed on `ν` from word `at` of `out`.
#[inline]
pub fn write_nu_seed(out: &mut [u32], at: usize, nu: f32) {
    let (s, e) = seed_input(nu);
    out[at] = 1;
    out[at + 1] = seed_word(Some(seed_of_sum(s, e)));
    out[at + 2] = nu.to_bits();
    out[at + 3] = s.to_bits();
    out[at + 4] = e.to_bits();
}

/// Writes one of `rho_angle`'s slots from word `at` of `out`: its outcome `c` on `input`, at `ρ̃`, `λ̃`.
#[inline]
fn write_angle_slot(
    out: &mut [u32],
    at: usize,
    (rho, lambda): ([f32; 2], [f32; 2]),
    c: bool,
    input: f32,
) {
    out[at] = 1;
    out[at + 1] = word(c);
    out[at + 2] = rho[0].to_bits();
    out[at + 3] = rho[1].to_bits();
    out[at + 4] = lambda[0].to_bits();
    out[at + 5] = lambda[1].to_bits();
    out[at + 6] = input.to_bits();
}

/// Writes `rho_angle`'s three slots of the sample at word `at` of `out`, on the configuration block's own `ρ̃` and `λ̃`,
/// through the functions `descriptor::config_angle` calls.
#[inline]
pub fn write_angle(out: &mut [u32], at: usize, rho: [f32; 2], lambda: [f32; 2]) {
    let (cross, dot) = descriptor::cross_dot(rho, lambda);
    let vectors = (rho, lambda);
    let zero = descriptor::cross_zero(cross);
    write_angle_slot(out, at + CROSS_ZERO_AT, vectors, zero, cross);
    let zero = descriptor::dot_zero(dot);
    write_angle_slot(out, at + DOT_ZERO_AT, vectors, zero, dot);
    let negative = descriptor::dot_negative(dot);
    write_angle_slot(out, at + DOT_NEGATIVE_AT, vectors, negative, dot);
}

/// Writes an `other operation` slot, its outcome `c`, at word `at` of `out`.
#[inline]
fn write_other(out: &mut [u32], at: usize, c: bool) {
    out[at] = 1;
    out[at + 1] = word(c);
}

/// What a chart's decode reads beyond the shared blocks, at f32: its construction's targets and seed input, and the shape
/// sphere inverse's `p`, `q`; each flag is false on a chart with no such thing. The values are those its production
/// decode forms.
#[derive(Clone, Copy)]
pub struct ChartReads {
    /// The chart runs the invariant-momentum construction.
    pub construction: bool,
    /// The construction's `K*`.
    pub k: f32,
    /// The construction's `L_z`.
    pub lz: f32,
    /// The seed is decided on `z_α`.
    pub alpha_seed: bool,
    /// `z_α`.
    pub z_alpha: f32,
    /// The seed is decided on `ν`.
    pub nu_seed: bool,
    /// `ν`.
    pub nu: f32,
    /// The chart runs the shape sphere's inverse.
    pub sphere: bool,
    /// The inverse's `p`, `q`.
    pub pq: [f32; 2],
}

impl ChartReads {
    /// A chart that reads none of them.
    #[inline]
    pub fn none() -> Self {
        ChartReads {
            construction: false,
            k: 0.0,
            lz: 0.0,
            alpha_seed: false,
            z_alpha: 0.0,
            nu_seed: false,
            nu: 0.0,
            sphere: false,
            pq: [0.0; 2],
        }
    }

    /// A construction at `(k, lz)` whose seed is decided on `ν`.
    #[inline]
    fn burrau(k: f32, lz: f32, nu: f32) -> Self {
        ChartReads {
            construction: true,
            k,
            lz,
            nu_seed: true,
            nu,
            ..Self::none()
        }
    }

    /// The invariant construction at `chart`'s targets, its seed decided on its frozen `z_α`.
    #[inline]
    fn invariant<C: InvariantChart<Param = f32>>(chart: &C, u: f32, v: f32) -> Self {
        let t = chart.invariant().targets::<f32>(u, v);
        ChartReads {
            construction: true,
            k: t.k,
            lz: t.lz,
            alpha_seed: true,
            z_alpha: chart.map::<f32>(u, v).z.config()[0],
            ..Self::none()
        }
    }
}

/// A chart the read-back decodes: [`ChartReads`] at `(u, v)`; by default none.
pub trait BranchChart: UniformChart {
    /// What the chart's decode reads at `(u, v)`.
    #[inline]
    fn reads(&self, _u: f32, _v: f32) -> ChartReads {
        ChartReads::none()
    }
}

impl BranchChart for Latent<f32> {}
impl BranchChart for MassSimplex<f32> {}
impl BranchChart for EuclidPlane<f32> {}
impl BranchChart for Mixed<ZMu1, Alpha, f32> {}
impl BranchChart for BurrauThetaDeltaM<f32> {}

impl BranchChart for InvariantLE<f32> {
    #[inline]
    fn reads(&self, u: f32, v: f32) -> ChartReads {
        ChartReads::invariant(self, u, v)
    }
}

impl BranchChart for InvariantLK<f32> {
    #[inline]
    fn reads(&self, u: f32, v: f32) -> ChartReads {
        ChartReads::invariant(self, u, v)
    }
}

impl BranchChart for Burrau<f32> {
    #[inline]
    fn reads(&self, u: f32, v: f32) -> ChartReads {
        ChartReads::burrau(self.kinetic(v), 0.0, self.nu(u))
    }
}

impl BranchChart for BurrauThetaK<f32> {
    #[inline]
    fn reads(&self, u: f32, v: f32) -> ChartReads {
        ChartReads::burrau(self.kinetic(v), 0.0, self.nu(u))
    }
}

impl BranchChart for BurrauThetaLz<f32> {
    #[inline]
    fn reads(&self, u: f32, v: f32) -> ChartReads {
        ChartReads::burrau(self.kinetic(), self.lz(v), self.nu(u))
    }
}

impl BranchChart for ShapeSphere<f32> {
    #[inline]
    fn reads(&self, u: f32, v: f32) -> ChartReads {
        ChartReads {
            sphere: true,
            pq: inverse_pq(self.point(u, v), self.inertia),
            ..ChartReads::none()
        }
    }
}

/// A decode variant the read-back is built for: its compute entry's name.
pub trait BranchVariant: DecodeVariant<Chart: BranchChart> {
    /// The read-back's compute entry point.
    const READ_ENTRY: &'static str;
}

/// Writes one decode sample's [`DECODE_WORDS`] words from word `at` of `out`, zero `present` words already there: the
/// decode of `V`'s `chart` at `(u, v)` toward `e_star`, as the stage's `sample_with` forms it, and its branches, read on
/// what that decode emitted.
#[inline]
pub fn write_decode<V: BranchVariant>(
    out: &mut [u32],
    at: usize,
    chart: &V::Chart,
    (u, v): (f32, f32),
    e_star: Option<f32>,
) {
    let forbids = chart.forbids_energy_normalisation();
    let (d, energy) =
        energy_norm::apply_shared(chart.decode_at::<V::Links, f32>(u, v), e_star, forbids);
    let reads = chart.reads(u, v);
    // M01_TINY's slot is the sample's first, `M01_AT`.
    write_m01(out, at, d.ic.m[0], d.ic.m[1]);
    if reads.construction {
        write_feasibility(out, at + FEASIBILITY_AT, reads.k, reads.lz);
    }
    if reads.alpha_seed {
        write_alpha_seed(out, at + ALPHA_SEED_AT, reads.z_alpha);
    }
    if reads.nu_seed {
        write_nu_seed(out, at + NU_SEED_AT, reads.nu);
    }
    if energy != energy_norm::Outcome::Off {
        write_other(
            out,
            at + ENERGY_INFEASIBLE_AT,
            energy == energy_norm::Outcome::Infeasible,
        );
        write_other(
            out,
            at + ENERGY_UNSCALABLE_AT,
            energy == energy_norm::Outcome::Unscalable,
        );
    }
    let p = d.descriptor;
    write_other(
        out,
        at + RATIO_POLE_AT,
        descriptor::ratio_pole(p.lambda_mag),
    );
    write_other(
        out,
        at + COLLISION_POLE_AT,
        descriptor::collision_pole(p.r_min_pair_0),
    );
    write_other(out, at + VIRIAL_POLE_AT, descriptor::virial_pole(p.V_0));
    write_angle(out, at, d.config.rho_t, d.config.lambda_t);
    if reads.sphere {
        write_other(
            out,
            at + SPHERE_POLE_AT,
            inverse_pole(reads.pq[0], reads.pq[1]),
        );
    }
}

/// The read-back's body for `V`: invocation `i` reads sample `i`'s `(u, v)` and writes its words at sample `i` of `out`,
/// the chart and `η_E`'s target read from `uniforms` as the decode stage reads them; past the last sample it writes
/// nothing.
#[inline]
fn entry<V: BranchVariant>(i: u32, uniforms: &DecodeUniforms, uv: &[f32], out: &mut [u32]) {
    let i = i as usize;
    if i < uv.len() / 2 {
        let chart = V::Chart::from_uniform(&uniforms.chart);
        let e_star = target(uniforms.energy);
        write_decode::<V>(
            out,
            i * DECODE_WORDS,
            &chart,
            (uv[2 * i], uv[2 * i + 1]),
            e_star,
        );
    }
}

/// Canon's read-back: invocation `i` reads sample `i` of `ic` in the decode stage's word layout and writes its
/// [`CANON_WORDS`] words at sample `i` of `out`; past the last sample it writes nothing.
#[spirv(compute(threads(64)))]
pub fn canon(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] ic: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] out: &mut [u32],
) {
    let i = id.x as usize;
    if i < ic.len() / ic_words() {
        let state: Ic<f32> = read_ic(ic, i);
        write_canon(out, i * CANON_WORDS, state.m, state.r);
    }
}

/// A variant's [`BranchVariant`] and its compute entry point.
macro_rules! entries {
    ($($variant:ident $entry:ident $name:literal),* $(,)?) => {$(
        impl BranchVariant for $variant {
            const READ_ENTRY: &'static str = $name;
        }

        /// The read-back's compute entry point for one decode variant.
        #[spirv(compute(threads(64)))]
        pub fn $entry(
            #[spirv(global_invocation_id)] id: UVec3,
            #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
            #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
            #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] out: &mut [u32],
        ) {
            entry::<$variant>(id.x, uniforms, uv, out);
        }
    )*};
}

entries!(
    LatentDefaults latent_defaults "stage_branch_read_latent_defaults",
    LatentStickBreaking latent_stick_breaking "stage_branch_read_latent_stick_breaking",
    InvariantLEDefaults invariant_le_defaults "stage_branch_read_invariant_le_defaults",
    InvariantLKDefaults invariant_lk_defaults "stage_branch_read_invariant_lk_defaults",
    MassSimplexDefaults mass_simplex_defaults "stage_branch_read_mass_simplex_defaults",
    BurrauDefaults burrau_defaults "stage_branch_read_burrau_defaults",
    EuclidPlaneDefaults euclid_plane_defaults "stage_branch_read_euclid_plane_defaults",
    MixedMu1AlphaDefaults mixed_mu1_alpha_defaults "stage_branch_read_mixed_mu1_alpha_defaults",
    BurrauThetaKDefaults burrau_theta_k_defaults "stage_branch_read_burrau_theta_k_defaults",
    BurrauThetaLzDefaults burrau_theta_lz_defaults "stage_branch_read_burrau_theta_lz_defaults",
    BurrauThetaDeltaMDefaults burrau_theta_delta_m_defaults "stage_branch_read_burrau_theta_delta_m_defaults",
    ShapeSphereDefaults shape_sphere_defaults "stage_branch_read_shape_sphere_defaults",
);
