//! QA tests for TASK-M7-11's fix round: the custom-stop ramp's OKLab `a` and `b` bound (colour_composition §9.4,
//! "Stops"; REQ-COL-140, R-72), from the section's text, not the implementation.
//!
//! - `qa_ramp_ab_bound_on_reading`: the bound holds where a ramp is read from text too (`RenderState` serialises the
//!   ramp; §9.4 "serialised with the node"): `a` or `b` past `[−1, 1]` is refused, at `±1` read.
//! - `qa_ramp_ab_bound_admits_srgb_gamut`: §9.4 says the bound "refuses no colour of the sRGB gamut": every corner and
//!   edge sample of the linear sRGB cube, in OKLab, is a stop `Ramp::new` accepts.
//! - `qa_ramp_ab_bound_keeps_rgb_finite`: §9.4 says within the bound the ramp's linear sRGB is finite, each component
//!   at most 103 in magnitude: a property over stops anywhere in the bound and any `t`, with its control over the same
//!   inputs.
//! - `qa_ramp_ab_bound_corners_within_figure`: the same figure at each of the bound's corners, with its control.
//!
//! The colours and `t` values are test inputs, not register constants (REQ-SYS-082).

use engine::contract::stain::ramp::{Ramp, Stop};
use proptest::prelude::*;
use serde_json::json;
use validation::negative_control;

// ── reading from text ───────────────────────────────────────────────────────────────────────────────────────────────

/// A ramp's text, two stops, the second's OKLab `a` and `b` as given.
fn ramp_text(a: f64, b: f64) -> serde_json::Value {
    json!({
        "stops": [
            { "position": 0.0, "oklab": [0.5, 0.0, 0.0] },
            { "position": 1.0, "oklab": [0.5, a, b] },
        ],
        "cyclic": false,
    })
}

/// Checks that each `(a, b)` is refused on reading.
fn check_read_refused(cases: &[(f64, f64)]) {
    for &(a, b) in cases {
        assert!(
            serde_json::from_value::<Ramp>(ramp_text(a, b)).is_err(),
            "a ramp with a = {a}, b = {b} was read"
        );
    }
}

#[test]
fn qa_ramp_ab_bound_on_reading() {
    // Past the bound, on a or on b, either sign, and far past it (an a whose cube overflows f64): refused.
    check_read_refused(&[
        (1.5, 0.0),
        (-1.5, 0.0),
        (0.0, 1.5),
        (0.0, -1.5),
        (1e200, 0.0),
        (0.0, -1e200),
    ]);
    // At the bound, each sign: read, and the value kept.
    for (a, b) in [(1.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        let r: Ramp = serde_json::from_value(ramp_text(a, b))
            .unwrap_or_else(|e| panic!("a ramp at a = {a}, b = {b} was refused: {e}"));
        assert_eq!(r.oklab_at(1.0), Some([0.5, a, b]));
    }
}

negative_control!(
    qa_ramp_ab_bound_on_reading,
    "a stop at the bound, a = -1, b = 1, is read, so the refusal check must fail on it",
    expected = "was read",
    check_read_refused(&[(-1.0, 1.0)])
);

// ── the sRGB gamut lies inside the bound ────────────────────────────────────────────────────────────────────────────

/// Ottosson (2020): linear sRGB → OKLab.
fn ottosson_forward([r, g, b]: [f64; 3]) -> [f64; 3] {
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    ]
}

/// The linear sRGB cube sampled on an `n`-step grid, each sample's OKLab clamped to `L ∈ [0, 1]` (white's `L` is 1
/// up to rounding in the published coefficients; the bound under test is on `a` and `b`).
fn gamut_samples(n: u32) -> Vec<[f64; 3]> {
    let step = |k: u32| f64::from(k) / f64::from(n);
    let mut out = Vec::new();
    for i in 0..=n {
        for j in 0..=n {
            for k in 0..=n {
                let [l, a, b] = ottosson_forward([step(i), step(j), step(k)]);
                out.push([l.clamp(0.0, 1.0), a, b]);
            }
        }
    }
    out
}

/// Checks that each colour is a stop `Ramp::new` accepts.
fn check_accepted(colours: &[[f64; 3]]) {
    for &c in colours {
        let stops = vec![
            Stop {
                position: 0.0,
                oklab: c,
            },
            Stop {
                position: 1.0,
                oklab: [0.5, 0.0, 0.0],
            },
        ];
        assert!(
            Ramp::new(stops, false).is_ok(),
            "the sRGB colour at OKLab {c:?} was refused"
        );
    }
}

#[test]
fn qa_ramp_ab_bound_admits_srgb_gamut() {
    check_accepted(&gamut_samples(16));
}

negative_control!(
    qa_ramp_ab_bound_admits_srgb_gamut,
    "red with its a doubled past the bound, a = 1.2, is refused, so the acceptance check must fail on it",
    expected = "was refused",
    check_accepted(&[[0.627_955, 1.2, 0.125_846]])
);

// ── within the bound the linear sRGB is finite ──────────────────────────────────────────────────────────────────────

/// §9.4's figure: within the bound each linear sRGB component is at most 103 in magnitude.
const RGB_MAX: f64 = 103.0;

/// Checks that the ramp of `stops` gives, at `t`, a finite linear sRGB colour, each component within `max`.
fn check_finite(stops: Vec<Stop>, cyclic: bool, t: f64, max: f64) {
    let r = Ramp::new(stops.clone(), cyclic).expect("a ramp within the bound");
    let rgb = r.linear_at(t).expect("a finite t");
    assert!(
        rgb.iter().all(|c| c.is_finite() && c.abs() <= max),
        "stops {stops:?}, cyclic {cyclic}: at t = {t} the ramp is {rgb:?}, past {max}"
    );
}

fn in_bound_stop() -> impl Strategy<Value = Stop> {
    (0.0..=1.0f64, 0.0..=1.0f64, -1.0..=1.0f64, -1.0..=1.0f64).prop_map(|(p, l, a, b)| Stop {
        position: p,
        oklab: [l, a, b],
    })
}

/// The property's domain: two to five stops anywhere in the bound, some pushed to its corners, where the cubes are
/// largest; either kind of ramp; any `t`, in and out of `[0, 1]`.
fn in_bound_ramp() -> impl Strategy<Value = (Vec<Stop>, bool, f64)> {
    let stops = (
        prop::collection::vec(in_bound_stop(), 2..6),
        prop::collection::vec((any::<bool>(), any::<bool>(), any::<bool>()), 6),
    )
        .prop_map(|(mut stops, corners)| {
            for (s, &(push, sa, sb)) in stops.iter_mut().zip(&corners) {
                if push {
                    s.oklab = [
                        s.oklab[0],
                        if sa { 1.0 } else { -1.0 },
                        if sb { 1.0 } else { -1.0 },
                    ];
                }
            }
            stops.sort_by(|x, y| x.position.total_cmp(&y.position));
            stops
        });
    (stops, any::<bool>(), -2.0..=3.0f64)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn qa_ramp_ab_bound_keeps_rgb_finite((stops, cyclic, t) in in_bound_ramp()) {
        check_finite(stops, cyclic, t, RGB_MAX);
    }
}

negative_control!(
    qa_ramp_ab_bound_keeps_rgb_finite,
    "the property's own inputs reach large components (L = 1, a = b = -1 gives blue near 22), so with its limit at 9 \
     the property must fail on them; 9 and not 10, whose message would also match the real limit's, past 103",
    expected = "past 9",
    {
        // A runner of its own, seeded, so the control cannot pass or fail by luck, and persisting nothing: a failure
        // here is the point, and no regression file may be written for it.
        let config = ProptestConfig {
            cases: 512,
            max_shrink_iters: 0,
            failure_persistence: None,
            ..ProptestConfig::default()
        };
        let rng = proptest::test_runner::TestRng::deterministic_rng(proptest::test_runner::RngAlgorithm::ChaCha);
        let outcome = proptest::test_runner::TestRunner::new_with_rng(config, rng).run(
            &in_bound_ramp(),
            |(stops, cyclic, t)| {
                check_finite(stops, cyclic, t, 9.0);
                Ok(())
            },
        );
        if let Err(e) = outcome {
            panic!("{e}");
        }
    }
);

#[test]
fn qa_ramp_ab_bound_corners_within_figure() {
    // Every corner of the bound, L at 0 and at 1: within §9.4's figure.
    for l in [0.0, 1.0] {
        for a in [-1.0, 1.0] {
            for b in [-1.0, 1.0] {
                let c = [l, a, b];
                check_finite(
                    vec![
                        Stop {
                            position: 0.0,
                            oklab: c,
                        },
                        Stop {
                            position: 1.0,
                            oklab: c,
                        },
                    ],
                    false,
                    0.5,
                    RGB_MAX,
                );
            }
        }
    }
}

negative_control!(
    qa_ramp_ab_bound_corners_within_figure,
    "at the corner L = 1, a = 1, b = -1 a component exceeds 1, so the check with its limit at 1 must fail",
    expected = "past 1",
    {
        let c = [1.0, 1.0, -1.0];
        check_finite(
            vec![Stop { position: 0.0, oklab: c }, Stop { position: 1.0, oklab: c }],
            false,
            0.5,
            1.0,
        )
    }
);
