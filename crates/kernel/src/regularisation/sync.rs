//! The sync grid: the regularised occupants' fixed quantum of physical time (integrator_contract Part 2a § "The sync
//! grid is a fixed quantum of physical time").
//!
//! **One grid.** The boundaries are `t = k·Δ_sync`, `Δ_sync` the register's `delta_sync` ([`delta_sync`]), the same for
//! every run, independent of the horizon `T` and of frame timing; never `t_max / n_sync`. A boundary's time is one
//! product, `k·Δ_sync` ([`boundary_time`]), never a running sum.
//!
//! **`ADVANCE` stops only on the grid.** A seam call's `t_now` and `t_target` are macro-step indices, physical time
//! `index·dt_macro`; each is mapped to the latest boundary at or before it ([`boundary_of_step`],
//! [`boundary_at_or_before`]), and the march runs from the one to the other. An off-grid target returns the boundary
//! before it, so live march, catch-up and a re-boot from `t = 0` reach the same boundary state bitwise, whatever the
//! targets asked for and whatever `dt_macro` the horizon gives. A terminal still latches at the step it fires on,
//! between boundaries or not (Part 7).
//!
//! **Steps are sized from `Δ_sync`**, never from a call's target: each march's interval is `dt_left = Δ_sync`, its
//! entry size and step limit read it, and Aarseth–Zare's `dt_left` is `Δ_sync` (`az::march`, `heggie::march`,
//! `logh::march`, each taking it from its configuration's `delta_sync`).
//!
//! The unregularised occupants' macro-step grid, `dt_macro = max(10⁻³, T/65535)`, is not this grid.

use super::az::AzReal;
use crate::driver::advance::Width;
use crate::generated::constants::DELTA_SYNC;

/// The register's `Δ_sync` at the width `R`: 1/8, exact at both widths.
#[inline]
pub fn delta_sync<R: Width>() -> R {
    R::at_width(DELTA_SYNC as f32, DELTA_SYNC)
}

/// The physical time of boundary `k`, `k·Δ_sync`: one product, so boundary `k` is the same time however it was reached.
#[inline]
pub fn boundary_time<R: AzReal>(k: u32, delta_sync: R) -> R {
    R::count(k) * delta_sync
}

/// The latest sync boundary at or before the physical time `t`: `k` with `k·Δ_sync ≤ t < (k + 1)·Δ_sync`, decided by
/// those products, comparison-only (Part 2c rule 2); `0` for a `t` before the first boundary after `0`.
#[inline]
pub fn boundary_at_or_before<R: AzReal>(t: R, delta_sync: R) -> u32 {
    let mut k = (t / delta_sync).whole();
    if k > 0 && R::count(k) * delta_sync > t {
        k -= 1;
    }
    if k < u32::MAX && R::count(k + 1) * delta_sync <= t {
        k += 1;
    }
    k
}

/// The boundary a seam call's macro-step index `index` maps to: the latest at or before its physical time
/// `index·dt_macro` ([`boundary_at_or_before`]).
#[inline]
pub fn boundary_of_step<R: AzReal>(index: u32, dt_macro: R, delta_sync: R) -> u32 {
    boundary_at_or_before(R::count(index) * dt_macro, delta_sync)
}
