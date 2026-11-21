//! Check 3 (pitfalls §2.4): check 2's ground truth and score on `deep interior` ([`super::deep_interior`]), where the
//! transient population actually lives: the config chart's base rates are too high to discriminate (pitfalls §2.4),
//! and prin-rs's 895 transients were measured here.
//!
//! **The region, horizon and criterion** (REQ-VAL-124, proposed): the region as `fixtures/slices.toml` gives it, body 0
//! over the box of half-width 0.05 about `(0, 0)` with the other two at Burrau's positions, at rest, `t_max = 13` in
//! Burrau's units, each pixel's key in its own canonical units; the harness horizon [`super::TRUTH_FACTOR`] `t_max`,
//! the firings judged by half of it; the same precision criterion as check 2, [`super::check2::PRECISION_PROPOSED`]
//! over at least [`super::check2::MIN_DECIDED_FIRINGS`] decided firings.
//!
//! **`r_coll = 0`** ([`R_COLL`]), no collision terminal, Heggie integrating every encounter through. The plan is unclear
//! here (the region's `r_coll = 0.001` is the slice's, and check 3's fixture is this task's to propose); chose 0 because
//! at the slice's `0.001` the region is censored: measured on a 16 × 16 grid, 255 of 256 marches end in a collision
//! between `t ≈ 3.5` and `10` canonical, before the detector can fire on anything, and a check of the transients that
//! live in close encounters cannot be read on a fixture that ends every one. `r_coll` is a recorded parameter, not a
//! physical constant (prin-rs FINDINGS §4), and the ground-truth suite's benchmarks take 0 for the same reason
//! (`ground_truth::BENCHMARK_R_COLL`). The gate's report gives the slice's own `r_coll` run beside it.

use super::check2::{self, Scored, PRECISION_PROPOSED};
use super::march::PostEscape;
use super::Population;

/// The check's pixels per side, proposed.
pub const N: usize = 32;

/// The check's collision radius, Burrau's units (the module's note).
pub const R_COLL: f64 = 0.0;

/// The population scored by check 2's ground truth.
pub fn score(pop: &Population, runs: &[PostEscape]) -> Vec<Scored> {
    check2::score(pop, runs)
}

/// The verdict on the scores, at the precision `precision`.
pub fn judge(scores: &[Scored], precision: f64) -> Result<String, String> {
    check2::judge(&check2::tally(scores, false), precision)
}

/// The proposed precision.
pub const PRECISION: f64 = PRECISION_PROPOSED;
