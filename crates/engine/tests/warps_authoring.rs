//! Warps and pair maps in the authoring layer (design `principia_chart_composition.md` § 4, § 5.1): power and log are
//! refused on an energy-type axis by name, offered on the others; power's exponent must be finite and positive; a
//! warped or paired pair of latent axes leaves the latent chart's uniforms, which carry neither, for the CPU.

use engine::chart_authoring::lowering::{resolve_spec, Lowering};
use engine::chart_authoring::registry::{AxisSpec, ChartSpec, PairMap, Registry, Warp};
use engine::chart_authoring::validate::{validate_spec_under, Refusal, Rules};
use validation::negative_control;

fn axis(name: &str, warp: Warp) -> AxisSpec {
    AxisSpec {
        warp,
        ..AxisSpec::pick(name)
    }
}

fn spec(h: AxisSpec, v: AxisSpec) -> ChartSpec {
    ChartSpec::new(h, v, [0.0; 8])
}

/// The refusals and the lowerings; `warp_check` is the validator's warp rule, off for the control.
fn check_authoring(warp_check: bool) {
    let registry = Registry::standard();
    let rules = Rules {
        warp_check,
        ..Rules::BUILT
    };
    let validate_spec = |r: &Registry, s: &ChartSpec| validate_spec_under(r, s, rules);
    for (name, warp, label) in [
        ("E", Warp::Power(2.0), "power"),
        ("K", Warp::Log, "log"),
        ("L_z", Warp::Log, "log"),
        ("α", Warp::Power(0.0), "power"),
        ("α", Warp::Power(f64::NAN), "power"),
    ] {
        match validate_spec(&registry, &spec(axis(name, warp), AxisSpec::pick("β"))) {
            Err(Refusal::WarpNotOffered { axis, warp }) => {
                assert!(axis == name && warp == label, "refused as {axis}, {warp}")
            }
            r => panic!("{name} warped {label} is not refused: {r:?}"),
        }
    }
    for warp in [Warp::Power(2.5), Warp::Log, Warp::Linear] {
        assert!(
            validate_spec(&registry, &spec(axis("α", warp), AxisSpec::pick("β"))).is_ok(),
            "α refused {warp:?}"
        );
    }
    let latent = |warp, pair_map| {
        let mut s = spec(axis("z_α", warp), AxisSpec::pick("z_β"));
        s.pair_map = pair_map;
        resolve_spec(&validate_spec(&registry, &s).expect("a latent pair")).lowering
    };
    assert!(matches!(
        latent(Warp::Feasibility, PairMap::Independent),
        Lowering::Uniform { .. }
    ));
    for (warp, pair_map) in [
        (Warp::Power(2.0), PairMap::Independent),
        (Warp::Log, PairMap::Independent),
        (
            Warp::Feasibility,
            PairMap::Polar {
                pole_crossing: true,
            },
        ),
        (Warp::Feasibility, PairMap::LogPolar),
    ] {
        assert!(
            matches!(latent(warp, pair_map), Lowering::Cpu { .. }),
            "{warp:?} under {pair_map:?} lowers to the latent chart's uniforms, which drop it"
        );
    }
}

#[test]
fn warps_and_pair_maps_authoring() {
    check_authoring(true);
}

negative_control!(
    warps_and_pair_maps_authoring,
    "the validator's warp rule off",
    expected = "warped power is not refused",
    check_authoring(false)
);
