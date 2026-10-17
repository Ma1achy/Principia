//! The CPU validation pass's view-level check (`engine::chart_validate`; chart_reference §0.6, §5.2; dd_decoder §3.7;
//! R-25, R-124; REQ-CHART-031):
//! - `forbids_energy_normalisation_lz_e`: a view combining `(L_z, E)` with an `E*` override is refused, for `Some(0)`
//!   and `Some(−0)` as for any other `Some(E*)`, finite or not, and the refusal names the flag; `None` passes.
//! - `forbids_energy_normalisation_lz_k`: the same for `(L_z, K)`.
//! - `forbids_energy_normalisation_allowing_chart`: a chart that allows `η_E`, the latent chart, passes every finite
//!   `E*` and `None`, and a non-finite `E*` is refused as such (dd_decoder §3.7, "the view's finite value").

use engine::chart_validate::{validate_view, Refusal};
use kernel::chart::invariant::Invariant;
use kernel::chart::{Chart, InvariantLE, InvariantLK, Latent};
use validation::negative_control;

/// The targets every check runs: `0`, `−0`, ordinary bound and unbound energies, the extremes, and the non-finite.
const TARGETS: [f64; 12] = [
    0.0,
    -0.0,
    -0.192,
    0.5,
    -1e-300,
    f64::MIN_POSITIVE,
    1e300,
    -1e300,
    f64::MAX,
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
];

/// A validation pass under check.
type Pass<C> = fn(&C, Option<f64>) -> Result<(), Refusal>;

/// Every `Some(E*)` with a forbidding chart is refused, naming the chart and the flag; `None` passes.
fn check_forbidding<C: Chart>(pass: Pass<C>, chart: &C) {
    let name = chart.name();
    assert!(chart.forbids_energy_normalisation(), "{name} allows η_E");
    assert_eq!(pass(chart, None), Ok(()), "{name} with E* off is refused");
    for e in TARGETS {
        match pass(chart, Some(e)) {
            Err(Refusal::ForbidsEnergyNormalisation { chart: c, e_star }) => {
                assert_eq!(c, name, "the refusal names another chart");
                assert_eq!(
                    e_star.to_bits(),
                    e.to_bits(),
                    "the refusal names another E*"
                );
                let text = Refusal::ForbidsEnergyNormalisation { chart: c, e_star }.to_string();
                assert!(
                    text.contains(name) && text.contains("forbids energy normalisation"),
                    "the refusal's message: {text}"
                );
            }
            other => panic!("{name} with Some({e:e}) is not refused for the flag: {other:?}"),
        }
    }
}

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.3, -0.5],
        z_mass: [0.7, -0.4],
        k_max: 1.0,
        gamma_k: 2.0,
    }
}

#[test]
fn forbids_energy_normalisation_lz_e() {
    check_forbidding(validate_view, &InvariantLE(invariant()));
}

/// A pass that reads `Some(0)` as "off", the R-25 hazard.
#[cfg(feature = "controls")]
fn zero_is_off<C: Chart>(chart: &C, e: Option<f64>) -> Result<(), Refusal> {
    match e {
        Some(0.0) => Ok(()),
        other => validate_view(chart, other),
    }
}

negative_control!(
    forbids_energy_normalisation_lz_e,
    "a pass reading Some(0) as off lets (L_z, E) with E* = 0 through",
    expected = "with Some(0e0) is not refused",
    check_forbidding(zero_is_off, &InvariantLE(invariant()))
);

#[test]
fn forbids_energy_normalisation_lz_k() {
    check_forbidding(validate_view, &InvariantLK(invariant()));
}

/// A pass that refuses only a non-finite target, ignoring the flag.
#[cfg(feature = "controls")]
fn flag_ignored<C: Chart>(chart: &C, e: Option<f64>) -> Result<(), Refusal> {
    match e {
        Some(x) if !x.is_finite() => validate_view(chart, e),
        _ => Ok(()),
    }
}

negative_control!(
    forbids_energy_normalisation_lz_k,
    "a pass ignoring the flag lets (L_z, K) with a finite E* through",
    expected = "is not refused for the flag",
    check_forbidding(flag_ignored, &InvariantLK(invariant()))
);

/// A chart that allows `η_E` passes `None` and every finite `E*`, and refuses each non-finite one as such.
fn check_allowing<C: Chart>(pass: Pass<C>, chart: &C) {
    let name = chart.name();
    assert!(!chart.forbids_energy_normalisation(), "{name} forbids η_E");
    assert_eq!(pass(chart, None), Ok(()), "{name} with E* off is refused");
    for e in TARGETS {
        let got = pass(chart, Some(e));
        if e.is_finite() {
            assert_eq!(got, Ok(()), "{name} with the finite E* = {e:e} is refused");
        } else {
            assert!(
                matches!(&got, Err(Refusal::NonFiniteTarget { chart: c, e_star })
                    if c == name && e_star.to_bits() == e.to_bits()),
                "{name} with the non-finite E* = {e} is not refused as such: {got:?}"
            );
        }
    }
}

fn latent() -> Latent<f64> {
    Latent {
        z0: [0.1; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    }
}

#[test]
fn forbids_energy_normalisation_allowing_chart() {
    check_allowing(validate_view, &latent());
}

/// A pass that refuses every target, as if every chart forbade `η_E`.
#[cfg(feature = "controls")]
fn every_target_refused<C: Chart>(chart: &C, e: Option<f64>) -> Result<(), Refusal> {
    match e {
        None => Ok(()),
        Some(e_star) => Err(Refusal::ForbidsEnergyNormalisation {
            chart: chart.name().to_owned(),
            e_star,
        }),
    }
}

negative_control!(
    forbids_energy_normalisation_allowing_chart,
    "a pass refusing every target refuses the latent chart's finite E*",
    expected = "with the finite E* = 0e0 is refused",
    check_allowing(every_target_refused, &latent())
);
