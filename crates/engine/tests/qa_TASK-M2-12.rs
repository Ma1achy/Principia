//! QA tests for TASK-M2-12, written from REQ-CHART-031 (chart_reference §0.6, §5.1, §5.2; dd_decoder §3.7;
//! chart_decoder_contract Part 5; inverse_encode_contract § "Chart-aware validation"; R-25, applied by R-124), not from
//! the implementation.
//! - `qa_m212_refuses_every_override`: the CPU validation pass refuses a view combining `(L_z, E)` or `(L_z, K)` with
//!   an energy-normalisation override, for every `Some(E*)`: `Some(0)` (R-25: the override is the `Option` being
//!   `Some`; "off" is `None`), `Some(−0)`, the frozen configuration's own `U` and `U + K` (the energies the chart
//!   itself spans, which a reader might take for harmless), and random finite targets, at several frozen
//!   configurations and warp parameters. The refusal is the flag's, naming the chart. `None` passes.
//! - `qa_m212_allowing_chart_accepts`: the refusal is the chart's flag, not the target: the latent chart, which allows
//!   `η_E`, passes `Some(0)` and the same finite targets. Without this, a pass refusing every `Some` would satisfy the
//!   first test.
//!
//! Each test has a registered negative control naming the panic it expects (R-176, R-212): a pass reading the old
//! docs' "non-zero `E*`" (letting `Some(0)` through), and a pass refusing every target on every chart.

use engine::chart_validate::{validate_view, Refusal};
use kernel::chart::invariant::Invariant;
use kernel::chart::{Chart, InvariantLE, InvariantLK, Latent};
use validation::decode_f32_eps::SplitMix;
use validation::negative_control;

/// A validation pass under check.
type Pass<C> = fn(&C, Option<f64>) -> Result<(), Refusal>;

/// The targets: zero of both signs, the chart's own energies at `U = u_pot`, and 200 random finite ones.
fn targets(u_pot: f64, k_max: f64, seed: u64) -> Vec<f64> {
    let mut rng = SplitMix::new(seed);
    let mut out = vec![0.0, -0.0, u_pot, u_pot + k_max, u_pot + k_max / 4.0, 1e-12];
    for _ in 0..200 {
        let x = f64::from(rng.uniform(-1.0, 1.0));
        let scale = 10f64.powi((rng.next_u64() % 13) as i32 - 6);
        out.push(x * scale);
    }
    out
}

fn charts() -> Vec<Invariant<f64>> {
    vec![
        Invariant {
            z_cfg: [0.0, 0.0],
            z_mass: [0.0, 0.0],
            k_max: 1.0,
            gamma_k: 2.0,
        },
        Invariant {
            z_cfg: [1.8, 0.4],
            z_mass: [-1.0, 1.2],
            k_max: 0.25,
            gamma_k: 1.0,
        },
        Invariant {
            z_cfg: [-2.5, -1.75],
            z_mass: [1.5, 1.5],
            k_max: 7.0,
            gamma_k: 3.5,
        },
    ]
}

fn check_refuses<C: Chart>(pass: Pass<C>, chart: &C, seed: u64) {
    let name = chart.name().to_owned();
    assert_eq!(
        pass(chart, None),
        Ok(()),
        "{name}: E* off (None) is refused"
    );
    for e in targets(-0.192, 1.0, seed) {
        match pass(chart, Some(e)) {
            Err(Refusal::ForbidsEnergyNormalisation { chart: c, e_star }) => {
                assert_eq!(c, name, "the refusal names {c}, not {name}");
                assert_eq!(
                    e_star.to_bits(),
                    e.to_bits(),
                    "the refusal names E* = {e_star}, not {e}"
                );
            }
            other => panic!(
                "{name} with the override Some({e:e}) is not refused for the flag: {other:?}"
            ),
        }
    }
}

fn check_both(le: Pass<InvariantLE<f64>>, lk: Pass<InvariantLK<f64>>) {
    for (i, c) in charts().into_iter().enumerate() {
        check_refuses(le, &InvariantLE(c), 0x212 + i as u64);
        check_refuses(lk, &InvariantLK(c), 0x2120 + i as u64);
    }
}

#[test]
fn qa_m212_refuses_every_override() {
    check_both(validate_view, validate_view);
}

/// The old docs' reading: only a non-zero `E*` is an override.
#[cfg(feature = "controls")]
fn non_zero_only<C: Chart>(chart: &C, e: Option<f64>) -> Result<(), Refusal> {
    validate_view(chart, e.filter(|x| *x != 0.0))
}

negative_control!(
    qa_m212_refuses_every_override,
    "A pass that lets Some(0) through, the docs' old 'non-zero E*', breaks R-25",
    expected = "with the override Some(0e0) is not refused",
    check_both(non_zero_only, non_zero_only)
);

fn latent() -> Latent<f64> {
    Latent {
        z0: [0.2, -0.1, 0.0, 0.3, 0.1, 0.0, 0.4, -0.2],
        q1: [0.0; 8],
        q2: [0.0; 8],
    }
}

fn check_accepts(pass: Pass<Latent<f64>>) {
    let chart = latent();
    assert!(
        !chart.forbids_energy_normalisation(),
        "the latent chart forbids η_E"
    );
    assert_eq!(
        pass(&chart, None),
        Ok(()),
        "the latent chart with E* off is refused"
    );
    for e in targets(-0.192, 1.0, 0x2121) {
        assert_eq!(
            pass(&chart, Some(e)),
            Ok(()),
            "the latent chart, which allows η_E, refuses the finite target Some({e:e})"
        );
    }
}

#[test]
fn qa_m212_allowing_chart_accepts() {
    check_accepts(validate_view);
}

/// A pass refusing every target on every chart.
#[cfg(feature = "controls")]
fn refuse_all<C: Chart>(chart: &C, e: Option<f64>) -> Result<(), Refusal> {
    match e {
        None => Ok(()),
        Some(e_star) => Err(Refusal::ForbidsEnergyNormalisation {
            chart: chart.name().to_owned(),
            e_star,
        }),
    }
}

negative_control!(
    qa_m212_allowing_chart_accepts,
    "A pass refusing every Some on every chart refuses the latent chart's targets",
    expected = "which allows η_E, refuses the finite target",
    check_accepts(refuse_all)
);
