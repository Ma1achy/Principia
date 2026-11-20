//! The CPU validation pass's view-level check (chart_decoder_contract Part 5, `forbids_energy_normalisation`;
//! inverse_encode_contract § "Chart-aware validation"; dd_decoder §3.7; chart_reference §0.6; R-25, R-124;
//! REQ-CHART-031): a view, a chart and its energy target `E*`, is refused before anything decodes when
//! - the chart forbids energy normalisation (`(L_z, E)` and `(L_z, K)`, `kernel::chart::invariant`) and the view sets
//!   any target: every `Some(E*)`, `Some(0)` included, since `E* = 0` is a real target and "off" is `None` (R-25), and
//!   a non-finite one as well; or
//! - the target is not finite, on a chart that allows it: dd_decoder §3.7 reads `E*` as "the view's finite value", so
//!   a NaN or infinite target never reaches the decode (applied per R-369, physics review 5535371186 of PR #216).
//!
//! The forbidding chart is checked first, so its refusal names the flag whatever the target. A view that passes is
//! decoded as dd_decoder §3.7 says: `None` leaves the momenta untouched, and on a chart that allows `η_E` the outcome at
//! `K₀ = 0` and on `E* < U` is a label on the decode, never a refusal (`kernel::decode::energy_norm`).

use std::fmt;

use kernel::chart::Chart;

/// Why the validation pass refuses a view.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// The chart `chart` forbids energy normalisation and the view sets the target `e_star` (R-25).
    ForbidsEnergyNormalisation { chart: String, e_star: f64 },
    /// The view's target `e_star` on the chart `chart` is not finite (dd_decoder §3.7).
    NonFiniteTarget { chart: String, e_star: f64 },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::ForbidsEnergyNormalisation { chart, e_star } => write!(
                f,
                "refused: the chart {chart} forbids energy normalisation, and the view sets E* = {e_star} \
                 (chart_reference §0.6; R-25: every Some(E*), Some(0) included)"
            ),
            Refusal::NonFiniteTarget { chart, e_star } => write!(
                f,
                "refused: the view's energy target E* = {e_star} on the chart {chart} is not finite (dd_decoder §3.7)"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// The validation pass on the view `(chart, e_star)`: `Ok` where it may decode, else the [`Refusal`] (the module's
/// note).
pub fn validate_view<C: Chart>(chart: &C, e_star: Option<f64>) -> Result<(), Refusal> {
    match e_star {
        None => Ok(()),
        Some(e) if chart.forbids_energy_normalisation() => {
            Err(Refusal::ForbidsEnergyNormalisation {
                chart: chart.name().to_owned(),
                e_star: e,
            })
        }
        Some(e) if !e.is_finite() => Err(Refusal::NonFiniteTarget {
            chart: chart.name().to_owned(),
            e_star: e,
        }),
        Some(_) => Ok(()),
    }
}
