//! The mock's placeholder content, pinned (R-390): its fake samples, hover path, quads and bounds are deterministic in
//! their inputs, so each is pinned here by a fingerprint of its values at fixed points, one per outcome class. The
//! values are placeholder content, not corpus values; the pins keep them, and every capture of them, from drifting
//! unseen. Each check also runs on an input it must reject (R-176, through `rejects`).

use engine::contract::sim_config::Plane;

use super::support::rejects;
use crate::mock::quads;
use crate::mock::sample::{hash, hover_path, sample};
use crate::mock::{MOCK_Q1, MOCK_Q2, MOCK_Z0};

fn plane() -> Plane {
    Plane {
        z0: MOCK_Z0,
        q1: MOCK_Q1,
        q2: MOCK_Q2,
    }
}

/// `a` within a relative 1e-12 of `b`.
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

/// A sample's values at `at`, the playhead at the horizon 50: its fate, `t_end`, FTLE, `E₀`, F₂ word, substeps,
/// minimum separation, `|ΔE/E|`, path span, and fingerprints of its shape path and its three bodies' paths.
type Pin = (
    [f64; 2],
    &'static str,
    f64,
    f64,
    f64,
    &'static str,
    u64,
    f64,
    f64,
    f64,
    f64,
    [f64; 3],
);

const PINS: [Pin; 9] = [
    (
        [0.0125, 0.37],
        "body 0 escape",
        13.065500184893608,
        0.10762578232053785,
        -0.6289034717516188,
        "a⁻¹ b a⁻¹ a⁻¹ a⁻¹",
        13215,
        0.0005882992686136277,
        3.6109020823641913e-10,
        13.065500184893608,
        1635.5341278437757,
        [86.22456288948777, 11.087705826265246, -8.287769025662847],
    ),
    (
        [0.02, 0.37],
        "body 1 escape",
        11.577927723701578,
        0.05882863585822634,
        -0.26553645481362764,
        "b⁻¹ a a⁻¹ b b⁻¹",
        1897,
        0.0005902133928029798,
        3.2776315387727665e-10,
        11.577927723701578,
        1601.7919063280583,
        [-24.546410069277268, 57.66558547150801, -18.179308539568325],
    ),
    (
        [0.01, 0.37],
        "body 2 escape",
        19.45344758743886,
        0.043415001376270086,
        -0.798409702812205,
        "b a⁻¹ b⁻¹ a a",
        25527,
        0.0009123883350905089,
        3.095884160353308e-10,
        19.45344758743886,
        1526.5043068226696,
        [-7.538005660708346, 1.7110780456801056, -49.97169912620519],
    ),
    (
        [0.03, 0.37],
        "bounded",
        50.0,
        0.057852986941288695,
        -0.791212596099649,
        "a⁻¹ a b a⁻¹ b⁻¹",
        10802,
        0.0009291081755029155,
        4.576595815984092e-10,
        50.0,
        1496.0201211935048,
        [-8.092774567336868, -7.905377978725281, 15.66355770062433],
    ),
    (
        [0.0075, 0.37],
        "collision 0–1 (pair 2)",
        19.552833864290736,
        0.08153871882197564,
        -0.6040762487929896,
        "b⁻¹ b⁻¹ b⁻¹ a b⁻¹",
        18553,
        0.0007579114671963907,
        9.95553510677928e-10,
        19.552833864290736,
        1447.5973921358336,
        [-14.651271335784086, 3.886276414104742, -9.641701039291826],
    ),
    (
        [0.0, 0.37],
        "collision 0–2 (pair 1)",
        14.504321306303607,
        0.09751699979853584,
        -0.3777323120011715,
        "a b a a a",
        31948,
        0.0002992313988397655,
        3.300672114135523e-10,
        14.504321306303607,
        1534.2510855275123,
        [-4.4451637633214665, 2.598746782978196, -9.398500598561531],
    ),
    (
        [0.06, 0.37],
        "collision 1–2 (pair 0)",
        6.403374704977978,
        0.11662986348666891,
        -0.7921301703390782,
        "b a⁻¹ b a⁻¹ b",
        3589,
        0.0002806751255386189,
        7.145669389610703e-10,
        6.403374704977978,
        1813.4571462098402,
        [-117.60829567515154, 106.34577245350664, -0.2592247399638321],
    ),
    (
        [0.0325, 0.37],
        "collision @ t=0",
        0.0,
        0.11977219904249069,
        -0.398639256635397,
        "b a⁻¹ a b a",
        17794,
        0.0007289659091031354,
        9.274791291980364e-10,
        5.0,
        1276.1227389545754,
        [-248.16168207063623, 102.86975263110415, 123.73840085403825],
    ),
    (
        [0.0025, 0.37],
        "degenerate",
        12.21898250078084,
        0.1022782568880939,
        -0.2779624066213728,
        "a⁻¹ b a⁻¹ a⁻¹ b⁻¹",
        29802,
        0.0006821431223135733,
        6.968955476142583e-10,
        12.21898250078084,
        1598.424909886902,
        [2.955203254203274, 0.6764505691364318, -13.57793273783865],
    ),
];

/// Asserts the sample at `pin`'s point matches it.
fn check_pin(plane: &Plane, pin: &Pin) {
    let (at, fate, t_end, ftle, e0, word, substeps, min_sep, drift, path_t, shape, bodies) = *pin;
    let s = sample(plane, at, 50.0, 50.0);
    assert_eq!(s.fate.as_deref(), Some(fate), "{at:?}");
    assert!(
        same(s.t_end.unwrap(), t_end),
        "t_end {:?} at {at:?}",
        s.t_end
    );
    assert!(
        same(s.ftle, ftle) && same(s.e0, e0),
        "FTLE {} E₀ {} at {at:?}",
        s.ftle,
        s.e0
    );
    assert_eq!((s.f2_word.as_str(), s.substeps), (word, substeps), "{at:?}");
    assert!(
        same(s.min_separation, min_sep) && same(s.energy_drift, drift),
        "{at:?}"
    );
    assert!(same(s.path_t, path_t), "path_t {} at {at:?}", s.path_t);
    let got: f64 = s.shape.iter().map(|q| q[0] + 2.0 * q[1]).sum();
    assert!(
        same(got, shape),
        "the shape path's fingerprint {got} at {at:?}"
    );
    for (k, body) in s.bodies.iter().enumerate() {
        let got: f64 = body.iter().map(|q| q[0] - 3.0 * q[1]).sum();
        assert!(
            same(got, bodies[k]),
            "body {k}'s fingerprint {got} at {at:?}"
        );
    }
}

#[test]
fn mock_hover_content_samples_are_pinned() {
    let p = plane();
    for pin in &PINS {
        check_pin(&p, pin);
    }
    rejects("a pin moved to another point", || {
        let mut moved = PINS[0];
        moved.0 = [0.5, 0.5];
        check_pin(&p, &moved)
    });
    for (x, h) in [
        (0.0, 0.0),
        (1.0, 0.5462073519520345),
        (-2.5, 0.729571438896528),
        (100.0, 0.17615513507917058),
    ] {
        assert!(same(hash(x), h), "hash({x}) = {}", hash(x));
    }
    // A negative sine's fraction is brought into [0, 1).
    assert!((0..200)
        .map(|i| hash(f64::from(i) * 0.37 - 30.0))
        .all(|h| (0.0..1.0).contains(&h)));
}

#[test]
fn mock_hover_content_path_and_quads_are_pinned() {
    let p = plane();
    let path = hover_path(&p, [0.4, 0.6]);
    assert_eq!(path.len(), 160);
    let at: f64 = path.iter().map(|q| q.at[0] + 3.0 * q.at[1]).sum();
    let alpha: f64 = path.iter().map(|q| f64::from(q.alpha)).sum();
    assert!((at - 352.0).abs() < 1e-9, "{at}");
    assert!((alpha - 95.4572403728962).abs() < 1e-4, "{alpha}");
    let leaves = quads::quads(&p);
    assert_eq!(leaves.len(), 682);
    let qs: f64 = leaves
        .iter()
        .map(|c| c[0] + 2.0 * c[1] + 3.0 * c[2] + 5.0 * c[3])
        .sum();
    assert!((qs - 3019.625).abs() < 1e-9, "{qs}");
    let bounds = quads::bounds(&p);
    assert_eq!(bounds.len(), 2728);
    let bs: f64 = bounds
        .iter()
        .map(|s| s[0][0] + 2.0 * s[0][1] + 3.0 * s[1][0] + 5.0 * s[1][1])
        .sum();
    assert!((bs - 15938.364999999812).abs() < 1e-6, "{bs}");
    rejects("another tree", || assert_eq!(leaves.len(), 681));
}
