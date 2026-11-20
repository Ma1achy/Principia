//! qa's tests for TASK-M2-37: the sign of the mass ratio (chart_reference §2.1, "A pixel whose ratio is at or below 0,
//! or is not realised, is not a system"; `MixedMassRatio::realised`, "`q` positive and finite"; REQ-CHART-068,
//! REQ-CHART-070). Written from those statements. A relative miss divides by the mass, so at a mass below 0 it comes out
//! negative and "within `ε_μ`" holds of anything: the round trip alone cannot refuse such a ratio when the link returns
//! the other masses in tolerance, which it can just below `q = 0`.
//!
//! - `qa_ratio_not_positive_or_not_finite_is_not_realised`: under either link, for every ordered pair, across held
//!   masses and `μ_max` over its registered range, no ratio at or below 0 and none infinite or NaN is realised.
//!   Control: the bare round trip, each mass within `ε_μ` of the relative miss, which takes such a ratio.
//! - `qa_ratio_not_positive_is_no_pixel`: the same through the chart: a linear row at such a ratio is not a system, and
//!   the warped range of an ask that reaches below 0 starts above 0. Control: a positive row, which passes.
//!
//! **Values.** The ratios, slices and `μ_max` of 1, 5 and 10 are test data, the last two the default and the registered
//! top; the one tolerance read is `ε_μ`, in the control.

use kernel::chart::energy::Mapping;
use kernel::chart::mixed::ZBeta;
use kernel::chart::physical::{ratio_masses, MixedMassRatio};
use kernel::chart::{Chart, ValidationResult};
use kernel::generated::links::{
    Defaults, Link, LinkParams, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ,
    StickBreaking,
};
use validation::negative_control;

/// The selection with `stick_breaking` on the mass block.
type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

type Ratio<S> = MixedMassRatio<ZBeta, S, f64>;

/// A realisation test, the chart's own or a control's.
type Realised<'a, S> = &'a dyn Fn(&Ratio<S>, f64, &LinkParams<f64>) -> bool;

const PAIRS: [[u32; 2]; 6] = [[0, 1], [1, 0], [0, 2], [2, 0], [1, 2], [2, 1]];
const HELD: [f64; 7] = [1.0e-4, 1.0e-3, 0.1, 1.0 / 3.0, 0.9, 0.999, 0.9999];
const MU_MAX: [f64; 3] = [1.0, 5.0, 10.0];
const ASKED: [f64; 2] = [0.5, 2.0];

/// Finite ratios at or below 0, from either zero out past `q = −1`, where `m_a` turns negative.
const NOT_POSITIVE: [f64; 15] = [
    -0.0,
    0.0,
    -5.0e-324,
    -f64::MIN_POSITIVE,
    -1.0e-300,
    -1.0e-12,
    -1.0e-9,
    -1.0e-7,
    -1.0e-3,
    -0.5,
    -1.0,
    -2.0,
    -1.0e9,
    -1.0e300,
    -f64::MAX,
];

const NOT_FINITE: [f64; 3] = [f64::INFINITY, f64::NEG_INFINITY, f64::NAN];

/// `z₀` holding the mass `held` on the third body of `pair`, the other two equal.
fn frozen<S: Selection>(pair: [u32; 2], held: f64, p: &LinkParams<f64>) -> [f64; 8] {
    let [z6, z7] = <S::MassLink as Link<2, 3>>::inverse_at(ratio_masses(pair, held, 1.0), p);
    [0.2, -0.3, 0.1, 0.15, -0.05, 0.25, z6, z7]
}

fn chart<S: Selection>(
    pair: [u32; 2],
    held: f64,
    asked: [f64; 2],
    mapping: Mapping,
    p: &LinkParams<f64>,
) -> Option<Ratio<S>> {
    Ratio::<S>::new(
        frozen::<S>(pair, held, p),
        [-1.0, 1.0],
        asked,
        pair,
        mapping,
        p,
    )
}

/// Each `(μ_max, pair, held)` of the grid.
fn each_slice(mut visit: impl FnMut(&LinkParams<f64>, [u32; 2], f64)) {
    for mu_max in MU_MAX {
        let p = LinkParams {
            mu_max,
            ..LinkParams::DEFAULT
        };
        for pair in PAIRS {
            for held in HELD {
                visit(&p, pair, held);
            }
        }
    }
}

fn realised_arm<S: Selection>(link: &str, realised: Realised<S>) {
    each_slice(|p, pair, held| {
        let built = chart::<S>(pair, held, ASKED, Mapping::Linear, p).expect("the linear chart");
        for q in NOT_POSITIVE.into_iter().chain(NOT_FINITE) {
            assert!(
                !realised(&built, q, p),
                "{link}, μ_max {}, bodies {pair:?}, held {held}: q = {q:?} is realised",
                p.mu_max
            );
        }
    });
    let p = LinkParams::DEFAULT;
    let centre = chart::<S>([0, 1], 1.0 / 3.0, ASKED, Mapping::Linear, &p).expect("the chart");
    assert!(
        realised(&centre, 1.0, &p),
        "{link}: the centre's q = 1 is not realised"
    );
}

/// The round trip with no test of the ratio's own sign: each of the masses, and the ratio, within `ε_μ` of its relative
/// miss, written as `(x − m).abs() / m`.
#[cfg(feature = "controls")]
fn round_trip_alone<S: Selection>(chart: &Ratio<S>, q: f64, p: &LinkParams<f64>) -> bool {
    let m = ratio_masses(chart.bodies, chart.held, q);
    let z = <S::MassLink as Link<2, 3>>::inverse_at(m, p);
    let back = <S::MassLink as Link<2, 3>>::forward_at(z, p);
    let within = |x: f64, m: f64| {
        (x - m).abs() / m <= <f64 as kernel::generated::constants::ChartConstants>::EPS_MU
    };
    within(kernel::chart::physical::ratio_of(chart.bodies, back), q)
        && back.iter().zip(m).all(|(&x, m)| within(x, m))
}

#[test]
fn qa_ratio_not_positive_or_not_finite_is_not_realised() {
    realised_arm::<Defaults>("softmax_tanh", &|c, q, p| c.realised(q, p));
    realised_arm::<Stick>("stick_breaking", &|c, q, p| c.realised(q, p));
}

negative_control!(
    qa_ratio_not_positive_or_not_finite_is_not_realised,
    "the bare round trip reads a negative miss at a mass at or below 0 as in tolerance, so it takes such a ratio",
    expected = "is realised",
    {
        realised_arm::<Stick>("stick_breaking", &|c, q, p| round_trip_alone(c, q, p));
    }
);

fn pixel_arm<S: Selection>(link: &str, ratios: &[f64]) {
    each_slice(|p, pair, held| {
        for &q in ratios {
            let asked = [q, ASKED[1]];
            let built =
                chart::<S>(pair, held, asked, Mapping::Linear, p).expect("the linear chart");
            assert!(
                matches!(
                    built.validate_params(0.5, 0.0, p),
                    ValidationResult::Reject { .. }
                ),
                "{link}, μ_max {}, bodies {pair:?}, held {held}: the row of q = {q:?} is passed",
                p.mu_max
            );
            if let Some(warped) = chart::<S>(pair, held, asked, Mapping::Warped, p) {
                assert!(
                    warped.ratio[0] > 0.0,
                    "{link}, μ_max {}, bodies {pair:?}, held {held}: the warped range of {asked:?} \
                     starts at {:?}",
                    p.mu_max,
                    warped.ratio[0]
                );
            }
        }
    });
}

fn check_pixels(ratios: &[f64]) {
    pixel_arm::<Defaults>("softmax_tanh", ratios);
    pixel_arm::<Stick>("stick_breaking", ratios);
}

#[test]
fn qa_ratio_not_positive_is_no_pixel() {
    check_pixels(&NOT_POSITIVE);
}

negative_control!(
    qa_ratio_not_positive_is_no_pixel,
    "a positive row the link realises, so asking that it be refused must fail",
    expected = "is passed",
    check_pixels(&[1.0])
);
