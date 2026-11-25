//! The f32 band (chart_reference § 2.1, "The f32 band at the floor"; R-876, REQ-CHART-169's CPU arm): where an f32
//! chart refuses a pixel because f32 rounding, not the chart, decides, the refusal is labelled as rounding, never drawn
//! as the chart's edge, and no pixel that is not a system is accepted.
//!
//! **The quantity and the width (a definition, R-72; under physics review).** The band is measured in the pixel's own
//! coordinates `(u, v)` at the chart's precision, the row coordinate `t = v` among them, and it is one float wide: an
//! f32 refusal at `(u, v)` is a rounding refusal iff
//! 1. the chart as authored at f64, whose narrowing to f32 the f32 chart is, accepts the same pixel (its f32
//!    coordinates widened exactly), so the chart's own maths has a system there; a parameter's rounding, such as
//!    `θ_max = π/4` narrowed, is f32 rounding too; and
//! 2. a pixel one f32 float away, `(u ± 1 ulp, v)` or `(u, v ± 1 ulp)`, is accepted at f32, so the refusal sits on the
//!    edge of what f32 accepts and not inside a refused region.
//!
//! The width is one float **as measured on these charts** (below), not derived for every chart: a check set by a
//! parameter narrowed to f32, or an operand whose map from `(u, v)` is flat (a power warp near `t = 0`), can put a
//! rounding refusal many floats of `u` or `v` from f32-accepted ground, where it would read [`Band::Unexplained`], a
//! false alarm, never a wrong picture. The GPU's edge walk (TASK-M4-06) repeats the measurement.
//! A refusal the f64 twin shares is the chart's edge ([`Band::Edge`]); one the f64 twin does not share and with no
//! f32-accepted pixel one float away is no rounding effect ([`Band::Unexplained`]), R-506's decoder-bug warning.
//!
//! **Measured (TASK-M2-66, `f32_band_label`):** on `HeldLz` over the `(ν, K)` and `(θ, K)` charts and on `HeldK`,
//! warped and linear, at held values over every magnitude, 2,121,000 pixels: no rounding refusal at the warp's floor,
//! which `invariant::k_floor` keeps feasible at the chart's own precision. The 24,134 rounding refusals are all on the
//! acute charts' `θ` axis at `u = 1`, where `θ_max = π/4` narrowed to f32 lies above `π/4`; against the f32 chart's own
//! parameters widened there are none, so the per-pixel arithmetic never decides alone.
//!
//! CPU-side, off the SPIR-V build: the GPU's edge walk is TASK-M4-06's.

use super::{Chart, Constraint, ValidationResult};

/// What a pixel of an f32 chart is (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    /// Accepted at f32.
    System,
    /// Refused at f32 inside the band: a rounding refusal, labelled as rounding.
    Rounding(Constraint),
    /// Refused at f32 and at f64: the chart's edge.
    Edge(Constraint),
    /// Refused at f32 only, with no f32-accepted pixel one float away: no rounding effect.
    Unexplained(Constraint),
}

impl Band {
    /// The label the pixel carries: `rounding` for a rounding refusal, `edge` for the chart's edge.
    pub fn label(self) -> &'static str {
        match self {
            Band::System => "system",
            Band::Rounding(_) => "rounding",
            Band::Edge(_) => "edge",
            Band::Unexplained(_) => "unexplained",
        }
    }
}

/// The constraint a result refuses, or `None` where it accepts.
fn refused(r: ValidationResult) -> Option<Constraint> {
    match r {
        ValidationResult::Valid => None,
        ValidationResult::Project { failed, .. }
        | ValidationResult::Clamp { failed, .. }
        | ValidationResult::Reject { failed } => Some(failed),
    }
}

/// `x` one f32 float up (`up`) or down.
fn step(x: f32, up: bool) -> f32 {
    if x.is_nan() || x.is_infinite() {
        return x;
    }
    let b = x.to_bits();
    let next = match (x == 0.0, up, x > 0.0) {
        (true, true, _) => 1,
        (true, false, _) => 0x8000_0001,
        (false, true, true) | (false, false, false) => b + 1,
        (false, true, false) | (false, false, true) => b - 1,
    };
    f32::from_bits(next)
}

/// The f32 pixel `(u, v)` of the f32 chart `narrow`, against its f64 twin `wide` (the module's note).
pub fn band<N: Chart, W: Chart>(narrow: &N, wide: &W, u: f32, v: f32) -> Band {
    let at = |c: &N, a: f32, b: f32| refused(c.validate(f64::from(a), f64::from(b)));
    let Some(failed) = at(narrow, u, v) else {
        return Band::System;
    };
    if refused(wide.validate(f64::from(u), f64::from(v))).is_some() {
        return Band::Edge(failed);
    }
    let near = [
        (step(u, true), v),
        (step(u, false), v),
        (u, step(v, true)),
        (u, step(v, false)),
    ];
    if near.iter().any(|&(a, b)| at(narrow, a, b).is_none()) {
        Band::Rounding(failed)
    } else {
        Band::Unexplained(failed)
    }
}
