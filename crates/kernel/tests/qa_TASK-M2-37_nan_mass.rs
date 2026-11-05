//! qa's tests for TASK-M2-37, fix round 6: a NaN mass on the mass-ratio chart, at the pixel and over every body pair
//! (chart_reference §2.1: "every pixel's masses are positive and sum to 1"; "A pixel whose ratio is at or below 0, or
//! is not realised, is not a system"; REQ-CHART-067, REQ-CHART-068). Written from that paragraph. A NaN compares false
//! with everything, so a test of "positive" or "within `ε_μ`" that drops it would pass one.
//!
//! - `qa_nan_mass_no_warped_chart`: a NaN in either mass control, or in both, offers no warped chart, under either link
//!   and for every ordered pair, whichever masses the NaN reaches: the third body's held mass, or only the other two
//!   (`stick_breaking`'s second control leaves `m₀` a number). Control: finite controls, which are offered.
//! - `qa_nan_mass_linear_rows_not_a_system`: under the linear option the chart keeps its fixed range and a NaN held
//!   mass makes every row, inside the square and past it, not a system: refused, never moved onto a row that passes.
//!   Control: a held mass that is a number, whose rows pass.
//!
//! **Values.** The ask `[1e-3, 1e3]` holds `q = 1` and the realised part of each finite slice used; the slices and rows
//! are test data; no tolerance is read here.

use kernel::chart::energy::Mapping;
use kernel::chart::mixed::ZBeta;
use kernel::chart::physical::MixedMassRatio;
use kernel::chart::{Chart, ValidationResult};
use kernel::generated::links::{
    Defaults, LinkParams, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ, StickBreaking,
};
use validation::negative_control;

/// The selection with `stick_breaking` on the mass block.
type Stick =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

type Ratio<S> = MixedMassRatio<ZBeta, S, f64>;

const PAIRS: [[u32; 2]; 6] = [[0, 1], [1, 0], [0, 2], [2, 0], [1, 2], [2, 1]];
const ASKED: [f64; 2] = [1.0e-3, 1.0e3];

fn slice(z: [f64; 2]) -> [f64; 8] {
    [0.2, -0.3, 0.1, 0.15, -0.05, 0.25, z[0], z[1]]
}

fn chart<S: Selection>(z: [f64; 2], pair: [u32; 2], mapping: Mapping) -> Option<Ratio<S>> {
    Ratio::<S>::new(
        slice(z),
        [-1.0, 1.0],
        ASKED,
        pair,
        mapping,
        &LinkParams::DEFAULT,
    )
}

fn no_warped_arm<S: Selection>(link: &str, controls: &[[f64; 2]]) {
    for &z in controls {
        for pair in PAIRS {
            assert!(
                chart::<S>(z, pair, Mapping::Warped).is_none(),
                "{link} offers a warped chart at z {z:?}, bodies {pair:?}"
            );
        }
    }
}

fn check_no_warped(controls: &[[f64; 2]]) {
    no_warped_arm::<Defaults>("softmax_tanh", controls);
    no_warped_arm::<Stick>("stick_breaking", controls);
}

#[test]
fn qa_nan_mass_no_warped_chart() {
    check_no_warped(&[
        [f64::NAN, 0.3],
        [0.3, f64::NAN],
        [f64::NAN, f64::NAN],
        [-f64::NAN, 0.3],
    ]);
}

negative_control!(
    qa_nan_mass_no_warped_chart,
    "the same refusal asked of finite controls must fail, so the check can see an offered chart",
    expected = "offers a warped chart",
    check_no_warped(&[[0.0, 0.0]])
);

/// Each row of a linear chart at `z`, inside the square and past both of its edges, refused, for each of `pairs`.
fn linear_arm<S: Selection>(link: &str, z: [f64; 2], pairs: &[[u32; 2]]) {
    for &pair in pairs {
        let built =
            chart::<S>(z, pair, Mapping::Linear).expect("the linear option keeps its chart");
        for j in -8..=72 {
            let v = f64::from(j) / 64.0;
            assert!(
                matches!(
                    built.validate_params(0.5, v, &LinkParams::DEFAULT),
                    ValidationResult::Reject { .. }
                ),
                "{link}, z {z:?}, bodies {pair:?}: the row {v} is passed"
            );
        }
    }
}

/// A NaN held mass under each link: `softmax_tanh`'s NaN reaches every mass, `stick_breaking`'s first control too, and
/// its second control reaches the held mass of every pair but `(1, 2)` and `(2, 1)`, which hold `m₀`.
fn check_linear(z: [f64; 2]) {
    linear_arm::<Defaults>("softmax_tanh", z, &PAIRS);
    linear_arm::<Stick>("stick_breaking", z, &PAIRS);
}

#[test]
fn qa_nan_mass_linear_rows_not_a_system() {
    check_linear([f64::NAN, 0.3]);
    check_linear([f64::NAN, f64::NAN]);
    linear_arm::<Defaults>("softmax_tanh", [0.3, f64::NAN], &PAIRS);
    linear_arm::<Stick>(
        "stick_breaking",
        [0.3, f64::NAN],
        &[[0, 1], [1, 0], [0, 2], [2, 0]],
    );
}

negative_control!(
    qa_nan_mass_linear_rows_not_a_system,
    "a held mass that is a number has rows the link realises, so refusing them all must fail",
    expected = "is passed",
    check_linear([0.0, 0.0])
);
