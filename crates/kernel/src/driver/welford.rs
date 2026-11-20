//! The Welford streaming diffusion (dd_simstate_payload §4; dd_integrator §3.5; R-17): the per-macro-step update of
//! the per-sample co-moments `mean_y` and `C_ty`, the time-only moments derived in closed form, and the per-sample `y`
//! the regression fits.
//!
//! **The time-only moments are derived, never a mutable shared global** (payload §4; REQ-PAY-054): a GPU dispatch has
//! no dispatch-wide barrier to publish one mid-dispatch, and none is needed. The samples are taken at `h, 2h, …, nh`,
//! `h = dt_macro`, `n` the completed macro-steps, so `mean_t = (n + 1)·h/2` ([`mean_t`]) and
//! `C_tt(n) = h²·n(n² − 1)/12` ([`c_tt`]) are functions of `(n, h)` alone, which every sample computes for itself.
//!
//! **The per-sample update** ([`Welford::update`]; REQ-INT-041), after the macro-step that makes the count `n`: with
//! the old-mean time deviation `δ_t = t_n − mean_t(n − 1) = 0.5·n·h` ([`delta_t`]), never the post-insertion one,
//! `mean_y += (y − mean_y)/n`, then `C_ty += δ_t·(y − mean_y)` with the new `mean_y`. Two accumulators of the sample's
//! own float type, f32 on the GPU, and nothing else: no stored `(t, y)` point and no raw sum (REQ-PAY-057). The slope
//! `C_ty / C_tt(n)` is derived at read by the ledger's read side, NaN for `n < 2` (R-245; `crate::payload`'s
//! `diffusion_slope`), so nothing here divides by `C_tt`.
//!
//! **`y`** ([`y`]; payload §4, the definition REQ-PAY-073 writes, R-72): the squared shape-sphere chord between the
//! sample's shape now and its IC's, `‖n̂(r_k, m) − n̂(r_0, m)‖²`, in `[0, 4]`. The wrapper takes it after each completed
//! macro-step, on the post-projection state, with `n̂(r_0, m)` derived once from the sample's own IC
//! (`super::wrapper`).
//!
//! Generic over the kernel's `Real` (R-265); the step count is converted by [`Count`], so no float constant is written.

use spirv_std::num_traits::Float;

use super::advance::Count;
use crate::shape::shape;
use crate::Real;

/// `mean_t = (n + 1)·h/2`, the mean of the sample times `h, 2h, …, nh` (payload §4). At `n = 0`, with no sample, it
/// reads `h/2`, which nothing reads.
#[inline]
pub fn mean_t<R: Real + Count>(n: u32, h: R) -> R {
    (R::count(n) + R::count(1)) * h / R::count(2)
}

/// `C_tt(n) = h²·n(n² − 1)/12 = Σᵢ (tᵢ − mean_t)²` over `tᵢ = h, 2h, …, nh` (payload §4), 0 for `n < 2`. Written in
/// the read side's operation order, `h·h·n·(n − 1)·(n + 1)/12` (`crate::payload::diffusion_c_tt`), so that at f32 the
/// two agree to the bit; `n − 1` and `n + 1` are exact for every u16 `n`.
#[inline]
pub fn c_tt<R: Real + Count>(n: u32, h: R) -> R {
    let m = R::count(n);
    let one = R::count(1);
    h * h * m * (m - one) * (m + one) / R::count(12)
}

/// `δ_t = 0.5·n·h`, the time deviation of the `n`-th sample `t_n = nh` from the OLD mean `mean_t(n − 1) = nh/2`, the
/// Welford cross-term's time factor (payload §4: not `t_n − mean_t(n)`, which gives a wrong `C_ty`).
#[inline]
pub fn delta_t<R: Real + Count>(n: u32, h: R) -> R {
    R::count(1) / R::count(2) * R::count(n) * h
}

/// The per-sample `y` (payload §4; REQ-PAY-073): `‖n − n₀‖²`, the squared chord between two shape-sphere points, here
/// the sample's shape now and its IC's. In `[0, 4]` for unit vectors.
#[inline]
pub fn chord2<R: Real>(n: [R; 3], n_0: [R; 3]) -> R {
    let d = [n[0] - n_0[0], n[1] - n_0[1], n[2] - n_0[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

/// The per-sample `y` of the configuration `r` with masses `m` against the IC's shape `n_0 = n̂(r_0, m)` (payload §4):
/// [`chord2`] of [`shape`]`(r, m)` and `n_0`.
#[inline]
pub fn y<R: Real + Float>(r: [[R; 2]; 3], m: [R; 3], n_0: [R; 3]) -> R {
    chord2(shape(r, m), n_0)
}

/// The per-sample Welford co-moments (payload §1, §4): `SimState`'s `mean_y` and `C_ty`, fixed in size. A run starts
/// from [`Welford::default`], both 0, the state before any sample.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Welford<R> {
    /// The running mean of `y` over the samples taken.
    pub mean_y: R,
    /// The centred co-moment `Σᵢ (tᵢ − mean_t)(yᵢ − mean_y)`.
    pub c_ty: R,
}

impl<R: Real + Count> Welford<R> {
    /// The co-moments after the sample `y` taken at the completed macro-step that makes the count `n`, `n ≥ 1`
    /// (payload §4): `mean_y += (y − mean_y)/n`, then `C_ty += δ_t·(y − mean_y)`, the new `mean_y`'s deviation times
    /// the old-mean time deviation [`delta_t`]. A pure function of the sample's own state and `(y, n, h)`: no shared
    /// state is read or written.
    #[inline]
    #[must_use]
    pub fn update(self, y: R, n: u32, h: R) -> Self {
        let mean_y = self.mean_y + (y - self.mean_y) / R::count(n);
        Self {
            mean_y,
            c_ty: self.c_ty + delta_t(n, h) * (y - mean_y),
        }
    }
}
