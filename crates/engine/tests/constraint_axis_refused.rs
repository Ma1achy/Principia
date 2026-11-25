//! A constraint beside an axis it fixes is refused (chart_composition § 6.1; chart_reference §0.6), each row of the
//! refusal table by name:
//! - `AxisAndHold`: an `L_z` axis with hold `L_z`; an `E`, `K`, `KE` or `Q` axis with hold `E`.
//! - `SameQuantity`: `(K, KE)`. `NoJointInverse`: a registered row with no inverse, picked. `SizeOnly`: `I`.
//!   `NoWarpWritten`: `PE`.
//! - `View`, on the kernel chart: `forbids_energy_normalisation` with any `E*`, and `forbids_lz_hold` (`L_z` an axis:
//!   `(L_z, E)`, `(L_z, K)`, `Φ_θ,L_z`, an `L_z`-axis `MixedEnergy`) with any `L*`, `Some(0)` and `Some(−0)` included;
//!   a chart that holds `L_z` by construction, or leaves it free, is not refused, and a non-finite `L*` is.
//! - `HoldAgainstConvention`, retired: a hold on a chart declared at rest is a label, not a refusal.
//!
//! **Pass criterion, written before the runs:** each case gives exactly its row, by name and with its payload; the
//! allowing charts pass every finite target. Controls: the flag cleared on `(L_z, E)` is not refused; `Some(0)` read as
//! off is not refused.

use engine::chart_authoring::registry::{AxisSpec, ChartSpec, Constraints, Registry};
use engine::chart_authoring::validate::{validate_spec, Refusal as Spec};
use engine::chart_validate::{validate_holds, Refusal};
use kernel::chart::energy::Mapping;
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::Alpha;
use kernel::chart::physical::{AxisK, AxisLz, MixedEnergy};
use kernel::chart::{BurrauThetaLz, Chart, InvariantLE, InvariantLK, Latent, LzHold};
use kernel::generated::links::Defaults;
use validation::negative_control;

/// The refusal a spec meets; `holds_read` false drops the constraints, as a validator that ignored them would.
fn refusal(h: &str, v: &str, c: Constraints, holds_read: bool) -> Option<Spec> {
    let mut s = ChartSpec::new(AxisSpec::pick(h), AxisSpec::pick(v), [0.0; 8]);
    s.constraints = if holds_read { c } else { NONE };
    validate_spec(&Registry::standard(), &s).err()
}

const NONE: Constraints = Constraints {
    hold_e: None,
    hold_lz: None,
};

fn check_rows(holds_read: bool) {
    let lz = Constraints {
        hold_lz: Some(0.0),
        ..NONE
    };
    assert_eq!(
        refusal("L_z", "α", lz, holds_read),
        Some(Spec::AxisAndHold {
            axis: "L_z".into(),
            quantity: "L_z"
        })
    );
    for q in ["E", "K", "KE", "Q"] {
        let e = Constraints {
            hold_e: Some(-1.0),
            ..NONE
        };
        assert_eq!(
            refusal(q, "α", e, holds_read),
            Some(Spec::AxisAndHold {
                axis: q.into(),
                quantity: "E"
            }),
            "{q} under hold E"
        );
    }
    assert!(matches!(
        refusal("K", "KE", NONE, holds_read),
        Some(Spec::SameQuantity { .. })
    ));
    assert_eq!(
        refusal("I", "α", NONE, holds_read),
        Some(Spec::SizeOnly { axis: "I".into() })
    );
    assert_eq!(
        refusal("PE", "α", NONE, holds_read),
        Some(Spec::NoWarpWritten { axis: "PE" })
    );
    assert_eq!(
        refusal("min(r_ij)/max(r_ij)", "α", NONE, holds_read),
        Some(Spec::NoJointInverse {
            a: "min(r_ij)/max(r_ij)".into(),
            b: "α".into()
        })
    );
    // Retired: hold L_z on a chart declared at rest is a label, never a refusal; an axis beside it that the hold does
    // not fix is untouched.
    assert_eq!(refusal("α", "β", lz, holds_read), None);
}

#[test]
fn constraint_axis_refused_rows() {
    check_rows(true);
}

negative_control!(
    constraint_axis_refused_rows,
    "constraints ignored by the validator leave an axis beside its hold unrefused",
    expected = "left: None",
    check_rows(false)
);

fn inv() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.3, -0.5],
        z_mass: [0.7, -0.4],
        k_max: 1.0,
        gamma_k: 2.0,
    }
}

const TARGETS: [f64; 8] = [
    0.0,
    -0.0,
    -0.3,
    0.5,
    1e300,
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
];

/// A pass under check: the chart and `L*`.
type Pass<C> = fn(&C, Option<f64>) -> Result<(), Refusal>;

fn pass<C: Chart>(c: &C, l: Option<f64>) -> Result<(), Refusal> {
    validate_holds(c, None, l)
}

/// A chart with `L_z` as an axis: every `Some(L*)` refused, naming the chart and the flag; `None` passes.
fn check_axis<C: Chart>(p: Pass<C>, c: &C) {
    let name = c.name();
    assert!(c.forbids_lz_hold(), "{name} allows hold L_z");
    assert_eq!(p(c, None), Ok(()), "{name} with L* off is refused");
    for l in TARGETS {
        match p(c, Some(l)) {
            Err(Refusal::ForbidsLzHold { chart, lz_star }) => {
                assert_eq!(chart, name);
                assert_eq!(lz_star.to_bits(), l.to_bits());
            }
            other => panic!("{name} with Some({l:e}) is not refused for the flag: {other:?}"),
        }
    }
}

/// A chart that allows it: every finite `L*` passes, a non-finite one is refused as such.
fn check_allowing<C: Chart>(c: &C) {
    let name = c.name();
    assert!(!c.forbids_lz_hold(), "{name} forbids hold L_z");
    for l in TARGETS {
        let got = pass(c, Some(l));
        if l.is_finite() {
            assert_eq!(got, Ok(()), "{name} with Some({l:e})");
        } else {
            assert!(
                matches!(got, Err(Refusal::NonFiniteLzTarget { .. })),
                "{name} {l:e}: {got:?}"
            );
        }
    }
}

#[test]
fn constraint_axis_refused_view() {
    check_axis(pass, &InvariantLE(inv()));
    check_axis(pass, &InvariantLK(inv()));
    check_axis(
        pass,
        &BurrauThetaLz::<f64> {
            theta_range: [0.2, 0.7],
            k: 1.0,
            lz_fraction: 0.5,
        },
    );
    let z0 = [0.3, -0.5, 0.2, -0.1, 0.15, 0.05, 0.7, -0.4];
    let lone_lz: MixedEnergy<Alpha, AxisLz, Defaults, f64> =
        MixedEnergy::new(z0, [0.2, 1.3], 1.0, 1.0, 0.5, Mapping::Warped);
    check_axis(pass, &lone_lz);
    let lone_k: MixedEnergy<Alpha, AxisK, Defaults, f64> =
        MixedEnergy::new(z0, [0.2, 1.3], 1.0, 1.0, 0.5, Mapping::Warped);
    assert_eq!(lone_k.lz_hold(), LzHold::Construction);
    check_allowing(&lone_k);
    check_allowing(&Latent::<f64> {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    });
}

negative_control!(
    constraint_axis_refused_view,
    "a chart whose flag is clear, standing for (L_z, E) with it cleared, is not refused",
    expected = "allows hold L_z",
    check_axis(
        pass,
        &Latent::<f64> {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        }
    )
);
