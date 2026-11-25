//! The CPU validation pass's view-level check (chart_decoder_contract Part 5, `forbids_energy_normalisation`;
//! inverse_encode_contract § "Chart-aware validation"; dd_decoder §3.7; chart_reference §0.6; R-25, R-124;
//! REQ-CHART-031): a view, a chart and its energy target `E*`, is refused before anything decodes when
//! - the chart forbids energy normalisation (`(L_z, E)` and `(L_z, K)`, `kernel::chart::invariant`) and the view sets
//!   any target: every `Some(E*)`, `Some(0)` included, since `E* = 0` is a real target and "off" is `None` (R-25), and
//!   a non-finite one as well; or
//! - the target is not finite, on a chart that allows it: dd_decoder §3.7 reads `E*` as "the view's finite value", so
//!   a NaN or infinite target never reaches the decode (applied per R-369, physics review 5535371186 of PR #216).
//!
//! Hold `L_z` is refused the same way ([`validate_holds`]): a chart with `L_z` as an axis (`forbids_lz_hold`: `(L_z, E)`,
//! `(L_z, K)`, `Φ_θ,L_z`, an `L_z`-axis `MixedEnergy`) with any `Some(L*)`, `Some(0)` included, and a non-finite
//! `L*` on a chart that allows it. A chart that holds `L_z` by its construction is not refused: the hold there is its
//! held value, and the decode runs no rotation after it.
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
    /// The chart `chart` has `L_z` as an axis and the view holds `L_z` at `lz_star`.
    ForbidsLzHold { chart: String, lz_star: f64 },
    /// The view's `L_z` target `lz_star` on the chart `chart` is not finite.
    NonFiniteLzTarget { chart: String, lz_star: f64 },
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
            Refusal::ForbidsLzHold { chart, lz_star } => write!(
                f,
                "refused: the chart {chart} has L_z as an axis, and the view holds L_z at L* = {lz_star} \
                 (every Some(L*), Some(0) included)"
            ),
            Refusal::NonFiniteLzTarget { chart, lz_star } => write!(
                f,
                "refused: the view's L_z target L* = {lz_star} on the chart {chart} is not finite"
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

/// The validation pass on the view `(chart, e_star, lz_star)`: [`validate_view`] on `E*` first, then `L*`: a chart with
/// `L_z` as an axis refuses every `Some(L*)`, naming the flag whatever the target, and a non-finite `L*` is refused
/// on a chart that allows the hold. Both holds off is [`validate_view`] at `None`.
pub fn validate_holds<C: Chart>(
    chart: &C,
    e_star: Option<f64>,
    lz_star: Option<f64>,
) -> Result<(), Refusal> {
    validate_view(chart, e_star)?;
    match lz_star {
        None => Ok(()),
        Some(l) if chart.forbids_lz_hold() => Err(Refusal::ForbidsLzHold {
            chart: chart.name().to_owned(),
            lz_star: l,
        }),
        Some(l) if !l.is_finite() => Err(Refusal::NonFiniteLzTarget {
            chart: chart.name().to_owned(),
            lz_star: l,
        }),
        Some(_) => Ok(()),
    }
}
