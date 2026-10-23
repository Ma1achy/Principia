//! Substep control (dd_integrator §3.3; integrator_contract Part 3, Part 4; R-502): which `N_sub` a macro-step takes,
//! and the record of it the payload keeps.
//!
//! **The law.** `N_sub` specifies as `clamp(⌈(r_sub/r_min)^{γ_sub}⌉, 1, N_max)` and is computed, never by a runtime
//! `pow`, `sqrt` or `div`, as `min(bucket(d²), N_max)`: the bucket is `1 + k`, `k` the number of the sim key's thresholds
//! `THR[n] = round_f32((r_sub·n^{−1/γ_sub})²)` that `d²` is strictly below, since `d² < (r_sub·n^{−1/γ_sub})²` is
//! `(r_sub/r_min)^{γ_sub} > n`. The thresholds are generated per sim key on the host (`super::nsub_table`) and reach
//! the kernel as exact f32 bit patterns in [`NsubThresholds`]; the comparison tree that reads them is generated once
//! over the table's maximum length, `L_max` ([`crate::generated::nsub_thresholds::bucket_tree`]), each threshold read
//! by a constant index, with no loop. So the branch compares one float, `d²`, against values every backend receives
//! bit for bit, and the transcendental never runs on the device.
//!
//! **Clamp before cast** (gpu_determinism_note § "The discipline", rule 3). The bucket is a leaf of the tree, an
//! integer constant, and `N_sub` its minimum with the integer `N_max`: no float is cast to an integer on the path, so
//! there is no out-of-range `OpConvertFToU` to clamp against. `d²` is compared only, `±∞` and NaN included: `+∞` and
//! NaN are below no threshold (bucket 1), and `0` is below every positive one.
//!
//! **The one float the branch reads** ([`d2_quantised`]). The positions are cast to f32 first, so the f64 and the f32
//! kernels decide on the same input; each separation is one subtraction; each square is [`square`], `IeeeBits::product`'s
//! split form, so a backend that fuses or contracts rounds it alike, with a floor below which a square is 0, so that no
//! partial product is subnormal (R-547; gpu_determinism_note rules 2 and 6); the two squares are added once, and the
//! least of the three pairs' sums is taken by comparison. The proof of this form on every backend, Metal and lavapipe
//! with their flushing and fusing, is REQ-INT-066's (TASK-M4-03, which writes and proves the form `d²` keeps); here it
//! is the CPU's, at f64 and at f32. *Was* (the task's earlier deliverable): "lone-sub, lone-mul, explicit
//! `fma(dy, dy, dx*dx)`", the form R-547 retired.
//!
//! **The record** ([`SubstepState`]). Per macro-step the live `N_sub` (render Part 5's animated effort view), the exact
//! u32 `total_substeps = Σ N_sub` and the sticky `saturated`, set when `N_sub == N_max` occurs and never cleared
//! (payload §2, descriptor bit 5; R-86). With `N_max ≤ L_max` and `horizon_steps ≤ 65535`, `Σ N_sub ≤ 65535·L_max`
//! fits a u32 (REQ-INT-094).
//!
//! **Domain** (pitfalls §10): each decision is identical given its macro-step's inputs, the quantised positions and
//! the sim key (R-84); along a chaotic trajectory the inputs differ by precision, and so may the decisions.

use spirv_std::glam::UVec4;

use crate::generated::nsub_thresholds::{bucket_tree, NSUB_TABLE_ROWS};
use crate::payload::set_saturated;
use crate::real::IeeeBits;
use crate::Real;

/// The sim key's `N_sub` thresholds as `SimUniforms` holds them (R-502): `THR[1] … THR[L_max]`, f32 bit patterns,
/// four to a 16-byte row, `THR[n]` at row `(n − 1)/4`, lane `(n − 1) mod 4` (`x`, `y`, `z`, `w`), so a uniform array's
/// 16-byte stride takes them with no padding. The host generator fills it (`super::nsub_table::thresholds`); dispatch
/// binds it (TASK-M3-06).
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct NsubThresholds {
    /// The rows of four thresholds' bits.
    pub rows: [UVec4; NSUB_TABLE_ROWS],
}

impl NsubThresholds {
    /// The bucket of `d2`: `1 + k`, `k` the number of thresholds `d2` is strictly below, from the generated comparison
    /// tree.
    #[inline]
    pub fn bucket(&self, d2: f32) -> u32 {
        bucket_tree(d2, &self.rows)
    }
}

/// `N_sub` for the macro-step whose `d²` is `d2`: `min(bucket(d2), n_max)` (dd_integrator §3.3). `n_max` is the sim
/// key's, at least 1 and at most `L_max`, which the host generator holds to (REQ-INT-091).
#[inline]
pub fn n_sub(d2: f32, table: &NsubThresholds, n_max: u32) -> u32 {
    table.bucket(d2).min(n_max)
}

/// A kernel float type's position as the bucket's `d²` reads it: rounded to f32, once.
pub trait Quantise: Real {
    /// `self` rounded to f32: the identity at f32, the IEEE conversion at f64.
    fn quantise(self) -> f32;
}

impl Quantise for f32 {
    #[inline]
    fn quantise(self) -> f32 {
        self
    }
}

impl Quantise for f64 {
    #[inline]
    fn quantise(self) -> f32 {
        self as f32
    }
}

/// `x²` as the bucket's `d²` forms it: 0 where `|x| < 2⁻⁴⁰`, else [`IeeeBits::product`]`(x, x)`.
///
/// The floor keeps every intermediate normal. With `2^e ≤ |x| < 2^(e+1)`, the product's high part holds `x`'s 12
/// leading significant bits and its low part is a multiple of `2^(e−23)`, so each partial product is exact and the
/// least nonzero one, low times low, is at least `2^(2e−46)`; at `e ≥ −40` that is at least `2⁻¹²⁶`, f32's least
/// normal, and the partial products, all of `x`'s sign squared, sum to no less than the largest. A subnormal `x` (two
/// positions closer than `2⁻¹²⁶`, which a flushing backend reads as 0) is under the floor on every backend, so it is 0
/// on every one. The test is on `x`'s exponent field, read from its bits. The floor moves a `d²` by less than `2⁻⁷⁹`,
/// half an ulp of `2⁻⁵⁵`; the host generator refuses a threshold that decides `N_sub` below `2⁻⁵⁵`
/// (`super::nsub_table`), so the floor moves a decision only within `2⁻²⁴` relative of an edge.
#[inline]
pub fn square(x: f32) -> f32 {
    if x.exponent() < -40 {
        0.0
    } else {
        x.product(x)
    }
}

/// The quantised `d²` of the pair `a`, `b`: one subtraction per axis, each square by [`square`], one addition.
#[inline]
fn pair_d2(a: [f32; 2], b: [f32; 2]) -> f32 {
    square(b[0] - a[0]) + square(b[1] - a[1])
}

/// The lesser of `a` and `b` by comparison: `b` where `b < a`, else `a` (a NaN `a` is kept).
#[inline]
fn least(a: f32, b: f32) -> f32 {
    if b < a {
        b
    } else {
        a
    }
}

/// The position-quantised `d²` of the closest pair of the positions `r`, the one float the bucket reads (the module's
/// note): each position cast to f32 ([`Quantise`]), then each R-22 pair's (`pair_d2`; pair `k` the side opposite
/// body `k`), the least of the three. The f64 and the f32 kernels give the same value for positions whose f32 casts
/// agree.
#[inline]
pub fn d2_quantised<R: Quantise>(r: &[[R; 2]; 3]) -> f32 {
    let q = [
        [r[0][0].quantise(), r[0][1].quantise()],
        [r[1][0].quantise(), r[1][1].quantise()],
        [r[2][0].quantise(), r[2][1].quantise()],
    ];
    least(
        least(pair_d2(q[1], q[2]), pair_d2(q[2], q[0])),
        pair_d2(q[0], q[1]),
    )
}

/// The substepper's record of a run (integrator_contract Part 3; payload §2): the live `N_sub` of the current
/// macro-step, the exact cumulative `total_substeps` and the sticky `saturated`. A run starts from
/// [`SubstepState::default`], all zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubstepState {
    /// The `N_sub` the current macro-step takes, the value the bucket gave it.
    pub live_n_sub: u32,
    /// `Σ N_sub` over the macro-steps so far, exact.
    pub total_substeps: u32,
    /// Whether `N_sub == N_max` has occurred at some macro-step; never cleared.
    pub saturated: bool,
}

impl SubstepState {
    /// Chooses the next macro-step's `N_sub` from its quantised `d2` ([`n_sub`]), records it, and returns it: the live
    /// `N_sub` is set to it, `total_substeps` grows by it, and `saturated` is set if it is `n_max`.
    #[inline]
    pub fn macro_step(&mut self, d2: f32, table: &NsubThresholds, n_max: u32) -> u32 {
        let n = n_sub(d2, table, n_max);
        self.live_n_sub = n;
        self.total_substeps += n;
        self.saturated |= n == n_max;
        n
    }

    /// The descriptor word `packed_a` with `saturated`, bit 5, set from this record (payload §2; R-86), every other
    /// bit kept.
    #[inline]
    pub fn packed_a(&self, packed_a: u32) -> u32 {
        set_saturated(packed_a, self.saturated)
    }
}
