//! The body plane against the Python reference's recorded fixture (TASK-M3-36; chart_reference §5.1; REQ-VAL-028,
//! REQ-VAL-119, REQ-VAL-146), and the chart bitwise equal to today's slice. The criteria are `validation::bodyplane`'s,
//! fixed before the first run: placement bitwise, `Φ`'s position within 6 ulp (derived), the gauge-free algebra within
//! `1e-13`.
//!
//! Controls: the shape vector compared without the sign convention fails; a `Φ` shifted by a hundredth of a cell fails
//! the position bound; a `Φ` written `c + h·(2u − 1)` is not bitwise today's slice.

use std::path::{Path, PathBuf};

use kernel::chart::BodyPlane;
use kernel::occupant::State;
use validation::bodyplane::{compare, Convention, Fixture, Placement};
use validation::gate::ablation::Raster;
use validation::gate::reregistration::{centred, ics, Case, Form, BURRAU_M};
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn check(convention: Convention, placement: Placement) {
    let fx = Fixture::load(&root()).expect("the fixture loads");
    assert_eq!(fx.cases.len(), 6, "six windows");
    assert_eq!(
        fx.rows.len(),
        5 * 81 + 9,
        "the five regions at 9² and the cross-check's 3²"
    );
    let r = compare(&fx, convention, placement);
    println!("{}", r.text());
    assert!(
        r.pass(),
        "the body plane disagrees with the Python reference:\n{}",
        r.text()
    );
}

#[test]
fn bodyplane_matches_the_python_reference() {
    check(Convention::Negated, Placement::Chart);
}

negative_control!(
    bodyplane_matches_the_python_reference,
    "the shape vector compared without the reference's n₂ sign convention must fail",
    expected = "disagrees with the Python reference",
    check(Convention::AsIs, Placement::Chart)
);

/// `Φ`'s pixel positions alone, under `placement`, against the derived 6-ulp bound.
fn check_position(placement: Placement) {
    let fx = Fixture::load(&root()).expect("the fixture loads");
    let r = compare(&fx, Convention::Negated, placement);
    let pos = r
        .columns
        .iter()
        .find(|c| c.name.starts_with("Φ position"))
        .expect("the position column");
    println!(
        "Φ position: worst {:.3} ulp at row {}, {} past {}",
        pos.worst, pos.at, pos.failures, pos.bound
    );
    assert_eq!(
        pos.failures, 0,
        "Φ's position is past its derived bound: worst {} ulp",
        pos.worst
    );
}

#[test]
fn bodyplane_position_within_its_derived_bound() {
    check_position(Placement::Chart);
}

negative_control!(
    bodyplane_position_within_its_derived_bound,
    "Φ shifted by a hundredth of a cell must fail the derived position bound",
    expected = "past its derived bound",
    check_position(Placement::Shifted)
);

/// `Φ` of `chart`, or of the control's form `c + h·(2u − 1)`, against the re-registration gate's `ics` (today's slice)
/// on every pixel of each Burrau region at a 64² raster. `ics` returns each state in its centre-of-mass frame, so the
/// chart's system goes through the same `centred` at the masses `ics` uses, `(3, 4, 5)`, and the positions are compared
/// bitwise.
fn check_today(control: bool) {
    let fx = Fixture::load(&root()).expect("the fixture loads");
    let mut differ = 0;
    let mut pixels = 0;
    for c in fx.cases.iter().filter(|c| c.n == 9) {
        let chart: BodyPlane<f64> = validation::bodyplane::chart(c);
        let case = Case {
            name: c.name.clone(),
            form: Form::BodyPlane {
                body: c.body as usize,
                centre: c.centre,
                half: c.half,
            },
            horizon: 13.0,
        };
        let raster = Raster::full(64);
        let today = ics(&case, raster);
        for (k, s) in today.iter().enumerate() {
            let s = s.as_ref().expect("today's slice builds every pixel");
            let (u, v) = raster.uv(k);
            let p = if control {
                let h = c.half;
                [
                    c.centre[0] + h * (2.0 * u - 1.0),
                    c.centre[1] + h * (2.0 * v - 1.0),
                ]
            } else {
                chart.position(u, v)
            };
            let (_, r) = chart.system_at(p);
            let ours = centred(&State {
                m: BURRAU_M,
                r,
                p: [[0.0; 2]; 3],
            });
            pixels += 1;
            if ours
                .r
                .iter()
                .flatten()
                .zip(s.r.iter().flatten())
                .any(|(a, b)| a.to_bits() != b.to_bits())
            {
                differ += 1;
            }
        }
    }
    println!("Φ against today's slice: {differ} of {pixels} pixels differ");
    assert_eq!(
        differ, 0,
        "Φ is not bitwise today's slice: {differ} of {pixels} pixels differ"
    );
}

#[test]
fn bodyplane_is_bitwise_todays_slice() {
    check_today(false);
}

negative_control!(
    bodyplane_is_bitwise_todays_slice,
    "Φ written c + h·(2u − 1) rounds differently from today's slice",
    expected = "is not bitwise today's slice",
    check_today(true)
);
