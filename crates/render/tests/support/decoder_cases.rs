//! The decoder's `DEGENERATE` outcomes and its non-finite decode, each at a chart point the chart accepts
//! (REQ-DEC-038, REQ-COL-102): a `decode_failed` sample of each, which REQ-COL-102's lane leaves unset, for the outcome
//! view's failures (`tests/outcome_view.rs`) and the presentation views' blank (`tests/presentation_no_hatch.rs`;
//! R-511).

use validation::chart::{self, Latent};
use validation::{energy_norm, Defaults, Degenerate, StickBreakingLinks};

/// A decoder outcome a presentation view draws blank, its lane unset (R-511): the decoder raises it after `Φ`, at a
/// chart point the chart accepts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Case {
    /// `DEGENERATE(ENERGY_UNSCALABLE)`: the rest start under an energy target above its potential.
    Unscalable,
    /// `DEGENERATE(ENERGY_INFEASIBLE)`: an energy target below the potential.
    Infeasible,
    /// `DEGENERATE(M01_TINY)`: the inner pair's mass below the mass block's ε.
    M01Tiny,
    /// A non-finite decoder output: `η_E` toward the largest finite target overflows the descriptor's `K_0`.
    NonFinite,
}

pub const CASES: [Case; 4] = [
    Case::Unscalable,
    Case::Infeasible,
    Case::M01Tiny,
    Case::NonFinite,
];

/// The latent chart pinned at `z0`, its plane's directions zero, so every `(u, v)` maps to `z0`.
pub fn pinned(z0: [f64; 8]) -> Latent<f64> {
    Latent {
        z0,
        q1: [0.0; 8],
        q2: [0.0; 8],
    }
}

/// `case`'s latent chart, which the decoder reaches `case` on at `(½, ½)`.
pub fn case_chart(case: Case) -> Latent<f64> {
    let mut z0 = [0.0; 8];
    match case {
        Case::Unscalable | Case::Infeasible => {}
        Case::M01Tiny => {
            z0[6] = 30.0;
            z0[7] = 30.0;
        }
        Case::NonFinite => z0[2] = -30.0,
    }
    pinned(z0)
}

/// Whether the shared decoder raises `case` at `chart`'s `(½, ½)`: the tag in `D ∘ Φ`'s `degenerate`, or, for the
/// non-finite decode, a non-finite descriptor under no tag.
pub fn raised(case: Case, chart: &Latent<f64>) -> bool {
    let energy = |e: f64| energy_norm::decode_chart::<Defaults, f64, _>(chart, 0.5, 0.5, Some(e)).0;
    match case {
        Case::Unscalable => energy(0.0).degenerate == Some(Degenerate::EnergyUnscalable),
        Case::Infeasible => energy(-1.0).degenerate == Some(Degenerate::EnergyInfeasible),
        Case::M01Tiny => {
            chart::decode::<StickBreakingLinks, f64, _>(chart, 0.5, 0.5).degenerate
                == Some(Degenerate::M01Tiny)
        }
        Case::NonFinite => {
            let d = energy(f64::MAX);
            d.degenerate.is_none() && !d.descriptor.K_0.is_finite()
        }
    }
}
