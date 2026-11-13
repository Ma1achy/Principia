//! The Benettin measurements and their gates (`validation::measure::benettin`; TASK-M3-14):
//! - `lagrange_known_exponent`: Routh's exponent of Lagrange's orbit, `½·√(√(27β) − 1)·ω` where `27β > 1` and 0 where
//!   not, at the equal-mass, `(0.5, 0.3, 0.2)` and `(0.98, 0.01, 0.01)` masses, against the roots of the rotating
//!   frame's characteristic equation (computed independently, to f64 rounding); the orbit's canonical `ω`.
//! - `measure_validity`: the measurement reads NaN, as `ftle_read` does, with no completed renormalisation:
//!   `n_renorm` 0 or one beyond the run's macro-steps, in both forms; at `n_renorm` equal to them it is finite.
//! - `test10_change`: dd test 10's change is the largest of the three variants' differences over `max(|λ_T|, 1/T)`,
//!   infinite where a read is not finite.
//! - `calibration_rule`: the rule picks the smallest worst change, ties at two significant digits going to the larger
//!   `n_renorm`, then the larger `‖δ₀‖`; the tolerance is the smallest one-digit value at or above twice it.
//! - `calibration_gate`: the gate's `test10` inputs pass at the proposed values on Lagrange's orbit and fail its control,
//!   and refuse an unknown orbit or precision; `sweep` passes only where the rule picks the proposal.
//! - `orbit_fixtures`: the orbits are what they are named: the Kepler-embedded pair's eccentricity 1/2 and its light
//!   third body's circular orbit, Burrau's (3, 4, 5) triangle at rest, Lagrange's equilateral triangle in rigid
//!   rotation at `ω² = M/a³`, all in the sim's units (`M = 1`, `I = 1`, CoM at rest at the origin).
//! - `shadow_form_gate` (REQ-VAL-038; R-974): each displacement form's f32 `λ_T` differs bitwise from the absolute
//!   form's on some orbit, and the absolute form against itself does not; the measurement covers the periodic set at
//!   both `‖δ₀‖`s and makes the choice; a row's reads, gaps (f32 less f64, in one form) and worst gap (a gap that is not
//!   finite counting as infinite); unknown forms refused.
//! - `shadow_form_complete`: the `periodic-set` verdict is structural alone: a table at each of its `‖δ₀‖`s in order,
//!   each with a row per orbit of the set in order, and two Burrau reads in each form, in order; a NaN read counts as a
//!   read (R-649); each part missing, extra or out of order fails it.
//! - `shadow_form_criterion` (R-974; TASK-M3-72's pre-registered criterion): Burrau alone decides where it separates
//!   (a1) from (a2); else (a2) exactly where `W(a1) − W(a2) ≥ 1/T`, the boundary included; else (a1).
//! - `shadow_form_burrau`: the criterion's Burrau read is the form's f32 `λ_T` on Burrau at each of its `‖δ₀‖`s, bit for
//!   bit, and finite exactly where both are.
//! - `eight_orbit_set`: the eight-orbit report's chaotic orbits are the Pythagorean triangles at rest their names give,
//!   Burrau first, and its eight starting conditions are the calibration orbits then the five others.
//! - `eight_orbit_rules`: the bias against the reference, the reference read, the spread, the pick with an orbit left
//!   out, the report's lines and its completeness, on synthetic candidates.
//! - `eight_orbit_provisional`: the values in force, provisional (R-976), are read from the candidate at both their
//!   `‖δ₀‖` and their `n_renorm`, never one alone, and are absent where the sweep did not run them.
//! - `eight_orbit_gate`: the report's candidates leave out the orbit a control names and refuse an unknown one before
//!   the sweep runs; the report passes complete, its regions the chaotic orbits, gives the rule's pick as the
//!   re-run's proposal beside the values in force, provisional (R-976), with their worst change (finite, not finite, or
//!   not in the sweep), and R-976's proposed `min_regions` ≥ 6 with its evidence, and fails with an orbit left out;
//!   an unknown shadow form is refused before the sweep; on synthetic candidates.

use std::path::Path;

use kernel::decode::Ic;
use validation::gate::Gate;
use validation::gate::RegionCount;
use validation::gate::{self, GateSpec, MeasurementGate, Verdict};
use validation::measure::benettin::{
    bias, burrau, burrau_finite, calibration_orbit, calibration_orbits, chaotic_orbits,
    chaotic_spread, choose, eight_orbit_candidates, eight_orbit_report, eight_orbits, figure_eight,
    finite_time_gaps, kepler_embedded, lagrange, lagrange_exponent, lambda, lambda_in,
    leave_one_out_lines, periodic_complete, periodic_set, provisional_worst, pythagorean,
    reference, report_complete, routh_beta, run_lines, select, select_without, shadow_forms,
    spread, spread_lines, sweep_confirms, tolerance, worst_gap, BenettinCalibrationGate, Candidate,
    Form, FormRow, GapRow, Orbit, PeriodicReads, Precision, ShadowFormGate, Test10, BURRAU_DELTA_0,
    CRITERION_DELTA_0, FORM_DELTA_0, GAP_HORIZONS, LAGRANGE_HORIZON, PROPOSED_DELTA_0,
    PROPOSED_MIN_REGIONS, PROPOSED_N_RENORM, PROPOSED_TOLERANCE, REFERENCE_DELTA_0,
};
use validation::negative_control;

use kernel::driver::benettin::{Absolute, Displacement};
use kernel::occupant::yoshida4::Yoshida4;
use validation::measure::benettin::measure;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

// ── lagrange_known_exponent ─────────────────────────────────────────────────────────────────────────────────────

/// The largest real part of the roots `s` of `s⁴ + s² + (27/4)β = 0`, the exponent in units of `ω`, from the
/// quadratic in `s²` solved in complex arithmetic by hand.
fn routh_roots(beta: f64) -> f64 {
    let c = 27.0 / 4.0 * beta;
    let disc = 1.0 - 4.0 * c;
    if disc >= 0.0 {
        // s² real and negative (both roots of the quadratic are negative): s imaginary.
        0.0
    } else {
        // s² = (−1 ± i·√(−disc))/2; Re √z = √((|z| + Re z)/2).
        let (re, im) = (-0.5, (-disc).sqrt() / 2.0);
        let modulus = (re * re + im * im).sqrt();
        ((modulus + re) / 2.0).sqrt()
    }
}

fn check_exponent(exponent: fn([f64; 3]) -> f64) {
    for m in [
        [1.0 / 3.0; 3],
        [0.5, 0.3, 0.2],
        [0.98, 0.01, 0.01],
        [0.9, 0.05, 0.05],
    ] {
        let want = routh_roots(routh_beta(m));
        let got = exponent(m);
        assert!(
            (got - want).abs() <= 1e-15,
            "masses {m:?}: the exponent {got} is not Routh's {want}"
        );
    }
}

#[test]
fn lagrange_known_exponent() {
    check_exponent(lagrange_exponent);
    assert!((routh_beta([1.0 / 3.0; 3]) - 1.0 / 3.0).abs() < 1e-16);
    // β is a ratio: masses not summing to 1 give the same β as the same masses normalised.
    assert!((routh_beta([1.0; 3]) - 1.0 / 3.0).abs() < 1e-16);
    assert!((routh_beta([5.0, 3.0, 2.0]) - routh_beta([0.5, 0.3, 0.2])).abs() < 1e-16);
    assert!((lagrange_exponent([1.0 / 3.0; 3]) - 0.5f64.sqrt()).abs() < 1e-15);
    // The equal-mass orbit in canonical units: side √3, ω = 3^(−3/4), its exponent ω/√2.
    let known = lagrange("equal", [1.0 / 3.0; 3], 30.0)
        .known
        .expect("known");
    assert!(
        (known - 3f64.powf(-0.75) / 2f64.sqrt()).abs() < 1e-12,
        "{known}"
    );
    assert_eq!(
        lagrange("stable", [0.98, 0.01, 0.01], 30.0).known,
        Some(0.0)
    );
    assert_eq!(figure_eight(30.0).known, Some(0.0));
    assert_eq!(periodic_set().len(), 4);
}

negative_control!(
    lagrange_known_exponent,
    "the other root's real part must fail",
    expected = "is not Routh's",
    check_exponent(|m| {
        let b = 27.0 * routh_beta(m);
        if b > 1.0 {
            0.5 * (b.sqrt() + 1.0).sqrt()
        } else {
            0.0
        }
    })
);

// ── measure_validity ────────────────────────────────────────────────────────────────────────────────────────────

/// A read of `λ_T` on `orbit` at `n_renorm`: the form's measurement, or a stand-in.
type Read = fn(&Orbit, u32) -> f64;

fn check_validity(reads: [Read; 2]) {
    // A short run: a few macro-steps, so that `n_renorm` can reach past them.
    let orbit = figure_eight(0.01);
    let steps = measure::<Yoshida4, f64, Absolute>(&orbit, 1e-3, 1).steps;
    assert!(steps > 1, "the run completes {steps} macro-steps");
    for read in reads {
        for n in [0, steps + 1] {
            let l = read(&orbit, n);
            assert!(
                l.is_nan(),
                "n_renorm = {n} past {steps} steps reads {l}, not NaN"
            );
        }
        for n in [1, steps] {
            let l = read(&orbit, n);
            assert!(l.is_finite(), "n_renorm = {n} of {steps} steps reads {l}");
        }
    }
}

fn absolute(o: &Orbit, n: u32) -> f64 {
    measure::<Yoshida4, f64, Absolute>(o, 1e-3, n).lambda
}

fn displacement(o: &Orbit, n: u32) -> f64 {
    measure::<Yoshida4, f32, Displacement>(o, 1e-3, n).lambda
}

#[test]
fn measure_validity() {
    check_validity([absolute, displacement]);
}

negative_control!(
    measure_validity,
    "a read finite with no completed renormalisation must fail",
    expected = "not NaN",
    check_validity([|o, n| absolute(o, n.max(1)), displacement])
);

// ── test10_change ───────────────────────────────────────────────────────────────────────────────────────────────

fn t10(lambdas: [f64; 4], horizon: f64) -> Test10 {
    Test10 {
        delta_0: 1e-3,
        n_renorm: 10,
        horizon,
        lambdas,
    }
}

fn check_change(change: fn(&Test10) -> f64) {
    let cases = [
        (t10([0.5, 0.55, 0.45, 0.5], 50.0), 0.1),
        (t10([0.5, 0.5, 0.5, 0.4], 50.0), 0.2),
        (t10([-0.5, -0.5, -0.45, -0.5], 50.0), 0.1),
        (t10([0.001, 0.011, 0.001, 0.001], 10.0), 0.1),
        (t10([0.2, 0.2, 0.2, 0.2], 10.0), 0.0),
    ];
    for (t, want) in cases {
        let got = change(&t);
        assert!(
            (got - want).abs() < 1e-12,
            "{:?}: the change {got} is not {want}",
            t.lambdas
        );
    }
    for bad in [f64::NAN, f64::INFINITY] {
        for k in 0..4 {
            let mut l = [0.5; 4];
            l[k] = bad;
            assert_eq!(
                change(&t10(l, 50.0)),
                f64::INFINITY,
                "the change is not {}",
                f64::INFINITY
            );
        }
    }
}

#[test]
fn test10_change() {
    check_change(Test10::change);
    // Its four runs are at the candidate, at half its ‖δ₀‖, at twice its n_renorm, and at both: a short orbit.
    let orbit = lagrange("short", [0.5, 0.3, 0.2], 1.0);
    let (d, n) = (1e-3, 10);
    let at = |d: f64, n: u32| lambda(&orbit, Precision::F64, d, n).to_bits();
    let t = Test10::run(&orbit, Precision::F64, d, n);
    assert_eq!(
        t.lambdas.map(f64::to_bits),
        [at(d, n), at(d / 2.0, n), at(d, 2 * n), at(d / 2.0, 2 * n)],
        "Test10's runs are not at (δ₀, n), (δ₀/2, n), (δ₀, 2n), (δ₀/2, 2n)"
    );
    assert!(
        at(d, n) != at(d / 2.0, n) && at(d, n) != at(d, 2 * n),
        "λ on the short orbit does not tell the variants apart"
    );
}

negative_control!(
    test10_change,
    "a change without the 1/T floor must fail",
    expected = "is not",
    check_change(|t| {
        let [l, rest @ ..] = t.lambdas;
        rest.iter().map(|x| (x - l).abs()).fold(0.0, f64::max) / l.abs()
    })
);

// ── calibration_rule ────────────────────────────────────────────────────────────────────────────────────────────

/// A candidate whose one run's change is `worst`.
fn cand(delta_0: f64, n_renorm: u32, worst: f64) -> Candidate {
    Candidate {
        delta_0,
        n_renorm,
        runs: vec![(
            "synthetic",
            Precision::F64,
            t10([1.0, 1.0 + worst, 1.0, 1.0], 50.0),
        )],
    }
}

fn check_rule(tol: fn(f64) -> f64) {
    for (worst, want) in [
        (0.0944, 0.2),
        (0.1, 0.2),
        (0.05, 0.1),
        (0.04, 0.08),
        (0.46, 1.0),
        (0.5, 1.0),
        (0.051, 0.2),
        (0.0004, 0.0008),
    ] {
        assert_eq!(
            tol(worst),
            want,
            "the tolerance from worst change {worst} is not {want}"
        );
    }
    let picks = |c: Vec<Candidate>| {
        let s = select(&c).expect("a candidate");
        (s.delta_0, s.n_renorm)
    };
    assert_eq!(
        picks(vec![
            cand(1e-3, 10, 0.3),
            cand(1e-4, 100, 0.2),
            cand(1e-2, 1, 0.25)
        ]),
        (1e-4, 100)
    );
    // Tied at two digits: the larger n_renorm, then the larger ‖δ₀‖.
    assert_eq!(
        picks(vec![cand(1e-3, 10, 0.1234), cand(1e-4, 100, 0.1225)]),
        (1e-4, 100)
    );
    assert_eq!(
        picks(vec![cand(1e-4, 100, 0.1225), cand(1e-3, 10, 0.1234)]),
        (1e-4, 100)
    );
    assert_eq!(
        picks(vec![cand(1e-4, 100, 0.12), cand(1e-3, 100, 0.12)]),
        (1e-3, 100)
    );
    assert_eq!(
        picks(vec![cand(1e-3, 100, 0.12), cand(1e-4, 100, 0.12)]),
        (1e-3, 100)
    );
    // Not tied: the smaller worst, whatever n_renorm.
    assert_eq!(
        picks(vec![cand(1e-3, 10, 0.11), cand(1e-4, 100, 0.12)]),
        (1e-3, 10)
    );
    assert!(select(&[]).is_none());
    assert_eq!(cand(1e-3, 10, f64::NAN).worst(), f64::INFINITY);
}

#[test]
fn calibration_rule() {
    check_rule(tolerance);
    assert_eq!(tolerance(0.0944), PROPOSED_TOLERANCE);
    // The sweep confirms the gate only when every part holds; each case below breaks one part alone.
    assert!(sweep_confirms(&cand(3e-3, 300, 0.0944), 0.2));
    for (c, tol, part) in [
        (cand(1e-3, 300, 0.0944), 0.2, "‖δ₀‖ is not the proposal"),
        (cand(3e-3, 100, 0.0944), 0.2, "n_renorm is not the proposal"),
        (
            cand(3e-3, 300, 0.3),
            0.2,
            "the rule's tolerance is not the gate's",
        ),
        (
            cand(3e-3, 300, 0.3),
            0.6,
            "the gate's tolerance is not the proposal",
        ),
    ] {
        assert!(!sweep_confirms(&c, tol), "the sweep confirms where {part}");
    }
}

negative_control!(
    calibration_rule,
    "a tolerance without the margin of two must fail",
    expected = "is not",
    check_rule(|w| tolerance(w / 2.0))
);

// ── calibration_gate ────────────────────────────────────────────────────────────────────────────────────────────

fn calibration_config() -> gate::GateConfig {
    gate::config(root(), "benettin-calibration").expect("the gate's gate.json")
}

fn run(input: serde_json::Value) -> Result<Verdict, String> {
    BenettinCalibrationGate
        .run(&calibration_config(), "test", &input)
        .map(|r| {
            r.check().expect("a complete report");
            r.verdict
        })
}

fn check_gate_test10(expect_control: Verdict) {
    let input = |extra: serde_json::Value| {
        let mut v = serde_json::json!({ "kind": "test10", "description": "t", "orbit": "lagrange", "precision": "f64" });
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        v
    };
    assert_eq!(run(input(serde_json::json!({}))), Ok(Verdict::Pass));
    assert_eq!(
        run(input(
            serde_json::json!({ "orbit": "burrau", "precision": "f32", "delta_0": 1e-4, "n_renorm": 10 })
        )),
        Ok(expect_control),
        "the control is not told apart"
    );
    assert!(run(input(serde_json::json!({ "orbit": "pluto" }))).is_err());
    assert!(run(input(serde_json::json!({ "precision": "f16" }))).is_err());
    assert!(run(serde_json::json!({ "kind": "neither", "description": "t" })).is_err());
}

#[test]
fn calibration_gate() {
    assert_eq!(BenettinCalibrationGate.name(), "benettin-calibration");
    assert_eq!(calibration_config().threshold.value, PROPOSED_TOLERANCE);
    check_gate_test10(Verdict::Fail);
    assert!(calibration_orbit("kepler").is_some() && calibration_orbit("burrau").is_some());
    // The calibration orbits are the three their doc names, at their masses.
    for (key, want) in [
        ("kepler", kepler_embedded()),
        ("burrau", burrau()),
        (
            "lagrange",
            lagrange("Lagrange, equal masses", [1.0 / 3.0; 3], LAGRANGE_HORIZON),
        ),
    ] {
        let got = calibration_orbit(key).expect("a calibration orbit");
        assert!(
            got.name == want.name && got.ic == want.ic && got.horizon == want.horizon,
            "the calibration orbit `{key}` is not the orbit its name says"
        );
    }
    // The sweep: the rule picks the proposal, and the gate's tolerance is the rule's.
    assert_eq!(
        run(serde_json::json!({ "kind": "sweep", "description": "t" })),
        Ok(Verdict::Pass)
    );
    assert_eq!((PROPOSED_DELTA_0, PROPOSED_N_RENORM), (3e-3, 300));
}

negative_control!(
    calibration_gate,
    "a gate that cannot tell the control from the proposal must fail",
    expected = "the control is not told apart",
    check_gate_test10(Verdict::Pass)
);

// ── shadow_form_gate ────────────────────────────────────────────────────────────────────────────────────────────

fn check_form_controls(second: &str) -> String {
    let GateSpec::Measurement(config) = gate::spec(root(), "shadow-form").expect("gate.json")
    else {
        panic!("shadow-form is not a measurement");
    };
    let control = |v: serde_json::Value| ShadowFormGate.control(&config, "t", &v);
    let differs = |a: &str, b: &str| {
        control(serde_json::json!({ "kind": "differs", "description": "d", "forms": [a, b] }))
            .expect("runs")
            .verdict
    };
    assert_eq!(
        differs("absolute", second),
        Verdict::Pass,
        "a displacement form reads as the absolute form on every orbit"
    );
    assert_eq!(differs("absolute", "absolute"), Verdict::Fail);
    let set =
        control(serde_json::json!({ "kind": "periodic-set", "description": "p" })).expect("runs");
    assert_eq!(set.verdict, Verdict::Pass);
    for o in periodic_set() {
        // Each orbit in both form tables; an unstable one in both gap tables too.
        let unstable = o.known.is_some_and(|k| k > 0.0);
        assert_eq!(
            set.markdown.matches(o.name).count(),
            if unstable { 4 } else { 2 },
            "{} not measured at each ‖δ₀‖ in each of its tables",
            o.name
        );
    }
    assert!(
        set.markdown.contains("Chosen: "),
        "the report makes no choice"
    );
    assert!(ShadowFormGate.flags(root(), &["--measure"]).is_err());
    assert!(control(serde_json::json!({ "kind": "differs", "description": "x" })).is_err());
    assert!(control(
        serde_json::json!({ "kind": "differs", "description": "x", "forms": ["absolute", "nope"] })
    )
    .is_err());
    set.markdown
}

#[test]
fn shadow_form_gate() {
    let report = check_form_controls("plain");
    check_form_controls("cancellation-free");
    // The criterion reads its worst gaps at ‖δ₀‖ = 1e-5, not at the other ‖δ₀‖ the tables give.
    let rows = shadow_forms(CRITERION_DELTA_0, PROPOSED_N_RENORM);
    let w = format!(
        "W(a1) = {:.5}, W(a2) = {:.5}",
        worst_gap(&rows, Form::Plain),
        worst_gap(&rows, Form::CancellationFree)
    );
    assert!(
        report.contains(&w),
        "the criterion's line does not read `{w}`, the worst gaps at ‖δ₀‖ = {CRITERION_DELTA_0:e}"
    );
    assert_eq!(ShadowFormGate.name(), "shadow-form");
    // The finite-time gap (physics N3): the unstable orbits alone, each measured at both horizons.
    let gaps = finite_time_gaps(PROPOSED_DELTA_0, PROPOSED_N_RENORM);
    assert_eq!(
        gaps.iter().map(|g| g.orbit).collect::<Vec<_>>(),
        ["Lagrange, equal masses", "Lagrange, (0.5, 0.3, 0.2)"]
    );
    let equal = periodic_set()[0];
    for (k, horizon) in GAP_HORIZONS.into_iter().enumerate() {
        let at = Orbit { horizon, ..equal };
        assert_eq!(
            gaps[0].lambdas[k].to_bits(),
            lambda(&at, Precision::F64, PROPOSED_DELTA_0, PROPOSED_N_RENORM).to_bits(),
            "the gap row's λ_T at T = {horizon} is not the f64 measurement there"
        );
    }
    assert_eq!(GAP_HORIZONS, [LAGRANGE_HORIZON, 2.0 * LAGRANGE_HORIZON]);
    let row = GapRow {
        orbit: "g",
        known: 0.5,
        lambdas: [0.25, 0.375],
    };
    assert_eq!(row.gaps(), [0.25, 0.125]);
    assert_eq!(
        row.line(),
        "| g | 0.50000 | 0.25000 (+0.25000, ×T +7.500) | 0.37500 (+0.12500, ×T +7.500) | 0.500 |"
    );
    // The set is the four orbits its doc names, at their masses.
    let named = [
        lagrange("Lagrange, equal masses", [1.0 / 3.0; 3], LAGRANGE_HORIZON),
        lagrange(
            "Lagrange, (0.5, 0.3, 0.2)",
            [0.5, 0.3, 0.2],
            LAGRANGE_HORIZON,
        ),
        lagrange(
            "Lagrange, (0.98, 0.01, 0.01)",
            [0.98, 0.01, 0.01],
            LAGRANGE_HORIZON,
        ),
        figure_eight(LAGRANGE_HORIZON),
    ];
    for (o, want) in periodic_set().iter().zip(&named) {
        assert!(
            o.name == want.name && o.ic == want.ic && o.horizon == want.horizon,
            "{} is not the orbit its name says",
            o.name
        );
    }
    // A row's reads, gaps and line.
    let row = FormRow {
        orbit: "r",
        known: 0.5,
        lambdas: [[0.75, 0.5], [0.625, 0.5], [0.5, 0.25]],
    };
    assert_eq!(row.lambda(Form::Plain, Precision::F32), 0.625);
    assert_eq!(row.lambda(Form::CancellationFree, Precision::F64), 0.25);
    assert_eq!(
        Form::ALL.map(|f| row.gap(f)),
        [0.25, 0.125, 0.25],
        "the gap is λ_T(f32) − λ_T(f64) in the same form"
    );
    assert_eq!(
        row.line(),
        "| r | 0.50000 | 0.75000 (+0.25000, +0.25000) | 0.62500 (+0.12500, +0.12500) | 0.50000 (+0.00000, +0.25000) |"
    );
    let bad = FormRow {
        lambdas: [[f64::NAN, 0.5], [0.5, 0.5], [-1.0, 0.5]],
        ..row
    };
    assert_eq!(worst_gap(&[row, bad], Form::Absolute), f64::INFINITY);
    assert_eq!(worst_gap(&[row, bad], Form::Plain), 0.125);
    assert_eq!(worst_gap(&[row, bad], Form::CancellationFree), 1.5);
    assert_eq!(worst_gap(&[], Form::Plain), 0.0);
    for f in Form::ALL {
        let word = match f {
            Form::Absolute => "absolute",
            Form::Plain => "plain",
            Form::CancellationFree => "cancellation-free",
        };
        assert_eq!(Form::parse(word), Ok(f));
    }
    assert!(Form::parse("displacement").is_err());
    // Each form's name in the tables, (a1) and (a2) as R-974 names them.
    assert_eq!(
        Form::ALL.map(Form::word),
        ["absolute", "(a1) plain", "(a2) cancellation-free"]
    );
    assert_eq!(CRITERION_DELTA_0, 1e-5);
    assert_eq!(BURRAU_DELTA_0, [1e-5, 1e-6]);
}

negative_control!(
    shadow_form_gate,
    "the absolute form held against itself as a displacement form must fail",
    expected = "a displacement form reads as the absolute form on every orbit",
    {
        check_form_controls("absolute");
    }
);

// ── shadow_form_complete ────────────────────────────────────────────────────────────────────────────────────────

/// A structurally complete `periodic-set` measurement, its values synthetic, a NaN among them as the absolute form's
/// on Burrau is.
fn complete_reads() -> PeriodicReads {
    let row = |orbit| FormRow {
        orbit,
        known: 0.5,
        lambdas: [[f64::NAN, 0.5], [0.5, 0.5], [0.5, 0.5]],
    };
    let rows = || periodic_set().map(|o| row(o.name)).to_vec();
    PeriodicReads {
        tables: FORM_DELTA_0.iter().map(|&d| (d, rows())).collect(),
        burrau: Form::ALL
            .iter()
            .map(|&f| (f, vec![f64::NAN; BURRAU_DELTA_0.len()]))
            .collect(),
    }
}

fn check_complete(complete: fn(&PeriodicReads) -> bool) {
    let full = complete_reads();
    assert!(
        complete(&full),
        "a complete measurement, a NaN among its reads, reads as incomplete"
    );
    let mut cases: Vec<(&str, PeriodicReads)> = Vec::new();
    let mut r = full.clone();
    r.tables.remove(0);
    cases.push(("the table at the first ‖δ₀‖ missing", r));
    let mut r = full.clone();
    r.tables.pop();
    cases.push(("the criterion's table missing", r));
    let mut r = full.clone();
    r.tables.reverse();
    cases.push(("the tables out of order", r));
    let mut r = full.clone();
    r.tables[1].0 = 1e-6;
    cases.push(("a table at another ‖δ₀‖", r));
    let mut r = full.clone();
    r.tables.push(full.tables[1].clone());
    cases.push(("a third table", r));
    let mut r = full.clone();
    r.tables[0].1.pop();
    cases.push(("an orbit's row missing at the first ‖δ₀‖", r));
    let mut r = full.clone();
    r.tables[1].1.remove(0);
    cases.push(("an orbit's row missing at the criterion's ‖δ₀‖", r));
    let mut r = full.clone();
    let extra = r.tables[1].1[0];
    r.tables[1].1.push(extra);
    cases.push(("an extra row", r));
    let mut r = full.clone();
    r.tables[0].1[2].orbit = "elsewhere";
    cases.push(("a row of another orbit", r));
    let mut r = full.clone();
    r.burrau.remove(0);
    cases.push(("the absolute form's Burrau reads missing", r));
    let mut r = full.clone();
    r.burrau.pop();
    cases.push(("(a2)'s Burrau reads missing", r));
    let mut r = full.clone();
    r.burrau.swap(1, 2);
    cases.push(("the Burrau forms out of order", r));
    let mut r = full.clone();
    r.burrau[1].1.pop();
    cases.push(("one Burrau read of (a1)'s two", r));
    let mut r = full.clone();
    r.burrau[2].1.push(0.5);
    cases.push(("three Burrau reads of (a2)'s", r));
    let mut r = full.clone();
    r.burrau[0].1.clear();
    cases.push(("no Burrau read of the absolute form's", r));
    for (what, reads) in cases {
        assert!(
            !complete(&reads),
            "an incomplete measurement reads as complete: {what}"
        );
    }
}

#[test]
fn shadow_form_complete() {
    check_complete(periodic_complete);
}

negative_control!(
    shadow_form_complete,
    "the vacuous check, a table at the criterion's ‖δ₀‖ with a row per orbit and nothing else, must fail",
    expected = "an incomplete measurement reads as complete",
    check_complete(|r| {
        r.tables
            .iter()
            .any(|t| t.0 == CRITERION_DELTA_0 && t.1.len() == periodic_set().len())
    })
);

// ── shadow_form_criterion ───────────────────────────────────────────────────────────────────────────────────────

/// The pre-registered (a1)/(a2) choice, `choose`, on its boundaries.
fn check_criterion(choose: fn([f64; 2], [bool; 2], f64) -> Form) {
    let t = 30.0;
    let step = 1.0 / t;
    // Burrau alone separates them: the finite one, whatever the gaps.
    assert_eq!(
        choose([0.0, 1.0], [false, true], t),
        Form::CancellationFree,
        "Burrau decides for (a2)"
    );
    assert_eq!(
        choose([1.0, 0.0], [true, false], t),
        Form::Plain,
        "Burrau decides for (a1)"
    );
    for burrau in [[true, true], [false, false]] {
        // At the boundary, W(a1) − W(a2) = 1/T exactly: (a2).
        assert_eq!(
            choose([2.0 * step, step], burrau, t),
            Form::CancellationFree,
            "at 1/T exactly"
        );
        // Just below it: (a1).
        let below = f64::from_bits((2.0 * step).to_bits() - 1);
        assert_eq!(
            choose([below, step], burrau, t),
            Form::Plain,
            "just below 1/T"
        );
        // (a2) worse: (a1).
        assert_eq!(choose([0.0, 1.0], burrau, t), Form::Plain, "(a2) worse");
        // (a1) not finite, (a2) finite: (a2); both not finite: (a1).
        assert_eq!(
            choose([f64::INFINITY, 0.5], burrau, t),
            Form::CancellationFree,
            "(a1) infinite"
        );
        assert_eq!(
            choose([f64::INFINITY; 2], burrau, t),
            Form::Plain,
            "both infinite"
        );
    }
}

#[test]
fn shadow_form_criterion() {
    check_criterion(choose);
}

// ── shadow_form_burrau ──────────────────────────────────────────────────────────────────────────────────────────

/// `read`, the criterion's Burrau read, against `lambda_in` at each of [`BURRAU_DELTA_0`], (a1) at the proposed
/// `n_renorm`.
fn check_burrau(read: fn(Form, u32) -> (bool, [f64; 2])) {
    let (finite, l) = read(Form::Plain, PROPOSED_N_RENORM);
    let o = burrau();
    for (k, d) in BURRAU_DELTA_0.into_iter().enumerate() {
        assert_eq!(
            l[k].to_bits(),
            lambda_in(&o, Form::Plain, Precision::F32, d, PROPOSED_N_RENORM).to_bits(),
            "Burrau's read at ‖δ₀‖ = {d:e} is not the f32 λ_T there"
        );
    }
    assert_eq!(
        finite,
        l.iter().all(|x| x.is_finite()),
        "Burrau's flag is not whether both reads are finite"
    );
}

#[test]
fn shadow_form_burrau() {
    check_burrau(burrau_finite);
}

negative_control!(
    shadow_form_burrau,
    "a Burrau read that is not the measurement must fail",
    expected = "is not the f32 λ_T there",
    check_burrau(|_, _| (true, [0.0; 2]))
);

negative_control!(
    shadow_form_criterion,
    "a choice that reads the boundary as strict must fail",
    expected = "at 1/T exactly",
    check_criterion(|w, b, t| match b {
        [true, false] => Form::Plain,
        [false, true] => Form::CancellationFree,
        _ if w[0] - w[1] > 1.0 / t => Form::CancellationFree,
        _ => Form::Plain,
    })
);

// ── orbit_fixtures ──────────────────────────────────────────────────────────────────────────────────────────────

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn norm(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// `M = 1`, `I = 1`, the centre of mass at the origin and at rest.
fn check_units(ic: &Ic<f64>, what: &str) {
    let m = ic.m;
    let total = m[0] + m[1] + m[2];
    let i: f64 = (0..3).map(|k| m[k] * dot(ic.r[k], ic.r[k])).sum();
    let com = [0, 1].map(|c| (0..3).map(|k| m[k] * ic.r[k][c]).sum::<f64>());
    let mom = [0, 1].map(|c| (0..3).map(|k| ic.p[k][c]).sum::<f64>());
    assert!((total - 1.0).abs() < 1e-12, "{what}: M = {total}");
    assert!((i - 1.0).abs() < 1e-12, "{what}: I = {i}");
    assert!(
        norm(com) < 1e-12 && norm(mom) < 1e-12,
        "{what}: CoM {com:?}, P {mom:?}"
    );
}

#[test]
fn orbit_fixtures() {
    // The Kepler-embedded orbit: the pair's eccentricity from its relative orbit, and body 2 circular about the pair.
    let k = kepler_embedded().ic;
    check_units(&k, "Kepler-embedded");
    assert!((k.m[2] - 1e-9).abs() < 1e-21 && (k.m[0] - k.m[1]).abs() < 1e-15);
    let (mp, mu) = (k.m[0] + k.m[1], k.m[0] * k.m[1] / (k.m[0] + k.m[1]));
    let rel = sub(k.r[1], k.r[0]);
    let vrel = sub(
        [k.p[1][0] / k.m[1], k.p[1][1] / k.m[1]],
        [k.p[0][0] / k.m[0], k.p[0][1] / k.m[0]],
    );
    let energy = mu * dot(vrel, vrel) / 2.0 - k.m[0] * k.m[1] / norm(rel);
    let a = -k.m[0] * k.m[1] / (2.0 * energy);
    let h = cross(rel, vrel);
    let e = (1.0 - h * h / (mp * a)).sqrt();
    assert!((e - 0.5).abs() < 1e-9, "the pair's eccentricity is {e}");
    assert!(
        dot(rel, vrel).abs() < 1e-12,
        "the pair does not start at an apsis"
    );
    assert!(norm(rel) > a, "the pair does not start at apocentre");
    let c = [0, 1].map(|j| (k.m[0] * k.r[0][j] + k.m[1] * k.r[1][j]) / mp);
    let vc = [0, 1].map(|j| (k.p[0][j] + k.p[1][j]) / mp);
    let (r2, v2) = (
        sub(k.r[2], c),
        sub([k.p[2][0] / k.m[2], k.p[2][1] / k.m[2]], vc),
    );
    assert!(
        dot(r2, v2).abs() < 1e-9 * norm(r2) * norm(v2),
        "body 2 moves radially"
    );
    assert!(
        (dot(v2, v2) * norm(r2) - 1.0).abs() < 1e-6,
        "body 2 is not circular"
    );
    assert!(
        (norm(r2) / a - 20.0).abs() < 1e-9,
        "body 2 is not at 20 semi-major axes"
    );
    // Burrau: the (3, 4, 5) triangle at rest, masses (5, 4, 3)/12 opposite the sides.
    let b = burrau().ic;
    check_units(&b, "Burrau");
    assert!(b.p.iter().flatten().all(|&x| x == 0.0));
    let side = |i: usize, j: usize| norm(sub(b.r[i], b.r[j]));
    let (s01, s02, s12) = (side(0, 1), side(0, 2), side(1, 2));
    assert!(
        (s01 / s12 - 0.6).abs() < 1e-12 && (s02 / s12 - 0.8).abs() < 1e-12,
        "{s01} {s02} {s12}"
    );
    for (got, want) in b.m.iter().zip([5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0]) {
        assert!((got - want).abs() < 1e-15);
    }
    // Lagrange: equilateral, rigid rotation, ω² = M/a³.
    for m in [[1.0 / 3.0; 3], [0.5, 0.3, 0.2], [0.98, 0.01, 0.01]] {
        let l = lagrange("L", m, 30.0);
        check_units(&l.ic, "Lagrange");
        let r = l.ic.r;
        let a = norm(sub(r[0], r[1]));
        assert!(
            (norm(sub(r[0], r[2])) - a).abs() < 1e-12 && (norm(sub(r[1], r[2])) - a).abs() < 1e-12
        );
        let omega = (1.0 / (a * a * a)).sqrt();
        for i in 0..3 {
            let v = [l.ic.p[i][0] / m[i], l.ic.p[i][1] / m[i]];
            let rot = [-omega * r[i][1], omega * r[i][0]];
            let anti = [omega * r[i][1], -omega * r[i][0]];
            assert!(
                norm(sub(v, rot)) < 1e-12 || norm(sub(v, anti)) < 1e-12,
                "body {i} of {m:?} is not in rigid rotation at ω = {omega}"
            );
        }
        let want = lagrange_exponent(m) * omega;
        assert!((l.known.expect("known") - want).abs() < 1e-12);
    }
    assert_eq!(
        (Precision::F32.word(), Precision::F64.word()),
        ("f32", "f64")
    );
}

negative_control!(
    orbit_fixtures,
    "a fixture outside the sim's units must fail",
    expected = "I = ",
    check_units(
        &Ic {
            m: [1.0 / 3.0; 3],
            r: [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]],
            p: [[0.0; 2]; 3]
        },
        "a unit-I check"
    )
);

// ── eight_orbit_set ─────────────────────────────────────────────────────────────────────────────────────────────

/// Checks `orbits` against Euclid's pairs `pairs`: each the triangle of sides `(a, b, c) = (m² − n², 2mn, m² + n²)` at
/// rest, `a` and `b` its legs over the hypotenuse, masses `(c, b, a)/(a + b + c)`, named by its sides (Burrau's by its
/// own name), in the sim's units.
fn check_chaotic(orbits: &[Orbit], pairs: &[(u32, u32)]) {
    assert_eq!(orbits.len(), pairs.len());
    for (o, &(m, n)) in orbits.iter().zip(pairs) {
        let (a, b, c) = (
            f64::from(m * m - n * n),
            f64::from(2 * m * n),
            f64::from(m * m + n * n),
        );
        let want = if (m, n) == (2, 1) {
            "Burrau".to_owned()
        } else {
            format!("({a}, {b}, {c}) at rest")
        };
        assert_eq!(o.name, want, "{} is not the triangle at ({m}, {n})", o.name);
        let ic = o.ic;
        check_units(&ic, o.name);
        assert!(ic.p.iter().flatten().all(|&x| x == 0.0), "{} moves", o.name);
        let side = |i: usize, j: usize| norm(sub(ic.r[i], ic.r[j]));
        let (s01, s02, s12) = (side(0, 1), side(0, 2), side(1, 2));
        assert!(
            (s01 / s12 - a / c).abs() < 1e-12 && (s02 / s12 - b / c).abs() < 1e-12,
            "{} is not the triangle at ({m}, {n}): sides {s01} {s02} {s12}",
            o.name
        );
        for (got, want) in ic.m.iter().zip([c, b, a].map(|x| x / (a + b + c))) {
            assert!(
                (got - want).abs() < 1e-15,
                "{} is not the triangle at ({m}, {n}): mass {got}, not {want}",
                o.name
            );
        }
        assert_eq!(o.horizon, burrau().horizon);
        assert_eq!(o.known, None);
    }
}

const PAIRS: [(u32, u32); 6] = [(2, 1), (3, 2), (4, 1), (4, 3), (5, 2), (5, 4)];

#[test]
fn eight_orbit_set() {
    check_chaotic(&chaotic_orbits(), &PAIRS);
    let same = |a: &Orbit, b: &Orbit| a.name == b.name && a.ic == b.ic && a.horizon == b.horizon;
    assert!(same(&pythagorean("Burrau", 2, 1), &burrau()));
    assert!(same(&chaotic_orbits()[0], &burrau()));
    // The eight: the three calibration orbits, then the five triangles after Burrau.
    let eight = eight_orbits();
    let want: Vec<Orbit> = calibration_orbits()
        .into_iter()
        .chain(chaotic_orbits().into_iter().skip(1))
        .collect();
    assert_eq!(eight.len(), 8);
    for (o, w) in eight.iter().zip(&want) {
        assert!(same(o, w), "{} is not {}", o.name, w.name);
    }
    // A pair that is not Euclid's is refused.
    assert!(std::panic::catch_unwind(|| pythagorean("x", 3, 1)).is_err());
}

negative_control!(
    eight_orbit_set,
    "two triangles' pairs swapped must fail",
    expected = "is not the triangle",
    check_chaotic(
        &chaotic_orbits(),
        &[(2, 1), (4, 1), (3, 2), (4, 3), (5, 2), (5, 4)]
    )
);

// ── eight_orbit_rules ───────────────────────────────────────────────────────────────────────────────────────────

/// A synthetic candidate: each run `(orbit, precision, λ_T at the candidate, its change)` over `T = 50`.
fn syn(delta_0: f64, n_renorm: u32, runs: &[(&'static str, Precision, f64, f64)]) -> Candidate {
    Candidate {
        delta_0,
        n_renorm,
        runs: runs
            .iter()
            .map(|&(o, p, l, change)| {
                // A change of `change` relative to `max(|λ|, 1/50)`.
                let d = change * l.abs().max(1.0 / 50.0);
                (o, p, t10([l, l + d, l, l], 50.0))
            })
            .collect(),
    }
}

fn synthetic_orbit(name: &'static str) -> Orbit {
    Orbit { name, ..burrau() }
}

fn check_eight_orbit_rules(bias: fn(f64, f64, f64) -> f64) {
    use Precision::{F32, F64};
    // The bias, on dd test 10's scale, its floor 1/T.
    for (l, r, t, want) in [
        (0.6, 0.5, 50.0, 0.2),
        (-0.4, -0.5, 50.0, 0.2),
        (0.01, 0.0, 50.0, 0.5),
        (0.0, 0.01, 50.0, -0.5),
    ] {
        let got = bias(l, r, t);
        assert!(
            (got - want).abs() < 1e-12,
            "the bias of {l} against {r} over {t} is {got}, not {want}"
        );
    }
    // The reference: f64 at REFERENCE_DELTA_0 at the same n_renorm, never f32's.
    let cands = vec![
        syn(1e-3, 10, &[("A", F64, 0.9, 0.0), ("B", F64, 0.3, 0.0)]),
        syn(
            REFERENCE_DELTA_0,
            10,
            &[("A", F32, 9.0, 0.0), ("A", F64, 0.5, 0.0)],
        ),
        syn(REFERENCE_DELTA_0, 20, &[("A", F64, 0.7, 0.0)]),
    ];
    assert_eq!(reference(&cands, "A", 10), Some(0.5));
    assert_eq!(reference(&cands, "A", 20), Some(0.7));
    assert_eq!(reference(&cands, "A", 30), None);
    assert_eq!(reference(&cands, "B", 10), None);
    assert_eq!(cands[1].lambda("A", F32), Some(9.0));
    assert_eq!(cands[1].lambda("B", F32), None);
    assert_eq!(
        cands[0]
            .without("A")
            .runs
            .iter()
            .map(|r| r.0)
            .collect::<Vec<_>>(),
        ["B"]
    );
    // The spread: least, upper median, largest; a non-finite value sorts as infinite; NaN for none.
    assert_eq!(spread(&[3.0, 1.0, 2.0]), [1.0, 2.0, 3.0]);
    assert_eq!(spread(&[4.0, 1.0, 3.0, 2.0]), [1.0, 3.0, 4.0]);
    assert_eq!(
        spread(&[1.0, f64::NAN]),
        [1.0, f64::INFINITY, f64::INFINITY]
    );
    assert!(spread(&[]).iter().all(|x| x.is_nan()));
    // The pick with an orbit left out: A's change decides between X and Y.
    let x = syn(1e-2, 1, &[("A", F64, 0.5, 0.9), ("B", F64, 0.5, 0.1)]);
    let y = syn(1e-3, 1, &[("A", F64, 0.5, 0.2), ("B", F64, 0.5, 0.3)]);
    let grid = vec![x.clone(), y.clone()];
    assert_eq!(select(&grid).map(|c| c.delta_0), Some(1e-3));
    assert_eq!(select_without(&grid, "A"), Some((1e-2, 1)));
    assert_eq!(select_without(&grid, "B"), Some((1e-3, 1)));
    assert_eq!(select_without(&[], "A"), None);
    assert_eq!(
        leave_one_out_lines(&grid, &y, &["A", "B"]),
        [
            "without A: the rule picks ‖δ₀‖ = 1e-2, n_renorm = 1, which is not the full set's pick",
            "without B: the rule picks ‖δ₀‖ = 1e-3, n_renorm = 1, which is the full set's pick",
        ]
    );
    assert_eq!(
        leave_one_out_lines(&[], &y, &["A"]),
        ["without A: no candidate"]
    );
    // The spread over the chaotic orbits alone, at one precision, the bias against the reference.
    let set = vec![
        syn(
            REFERENCE_DELTA_0,
            1,
            &[
                ("A", F32, 0.4, 0.1),
                ("A", F64, 0.5, 0.2),
                ("B", F64, 0.25, 0.4),
                ("K", F64, 0.1, 9.0),
            ],
        ),
        syn(
            1e-3,
            1,
            &[
                ("A", F32, 0.6, 0.3),
                ("A", F64, 0.55, 0.0),
                ("B", F64, 0.3, 0.6),
                ("K", F64, 0.2, 7.0),
            ],
        ),
    ];
    let [ch, bi] = chaotic_spread(&set, &set[1], F64, &["A", "B"]);
    assert!(
        (ch[0] - 0.0).abs() < 1e-12 && (ch[2] - 0.6).abs() < 1e-12 && ch[1] == ch[2],
        "{ch:?}"
    );
    assert!(
        (bi[0] - 0.1).abs() < 1e-12 && (bi[2] - 0.2).abs() < 1e-12,
        "{bi:?}"
    );
    let [ch32, bi32] = chaotic_spread(&set, &set[1], F32, &["A", "B"]);
    assert!(
        (ch32[0] - 0.3).abs() < 1e-12 && ch32[0] == ch32[2],
        "{ch32:?}"
    );
    // A's f32 run is read against A's f64 reference: (0.6 − 0.5)/0.5.
    assert!((bi32[0] - 0.2).abs() < 1e-12, "{bi32:?}");
    // No reference: NaN, which sorts as infinite.
    let lone = syn(1e-3, 7, &[("A", F64, 0.5, 0.0)]);
    let [_, nob] = chaotic_spread(std::slice::from_ref(&lone), &lone, F64, &["A"]);
    assert_eq!(nob, [f64::INFINITY; 3]);
    assert_eq!(
        spread_lines(&set[1..], &["A", "B"]),
        ["‖δ₀‖ = 1e-3, n_renorm = 1: worst change 7.0000; over the chaotic orbits (least / median / largest) \
          f32 change +0.300 / +0.300 / +0.300, bias +inf / +inf / +inf; f64 change +0.000 / +0.600 / +0.600, \
          bias +inf / +inf / +inf"]
    );
    assert_eq!(
        run_lines(&set[..1])[0],
        "‖δ₀‖ = 1e-6, n_renorm = 1: worst change 9.0000; A f32: 0.100 (λ_T 0.4000, bias -0.200); \
         A f64: 0.200 (λ_T 0.5000, bias +0.000); B f64: 0.400 (λ_T 0.2500, bias +0.000); \
         K f64: 9.000 (λ_T 0.1000, bias +0.000)"
    );
    assert_eq!(
        run_lines(&[lone])[0],
        "‖δ₀‖ = 1e-3, n_renorm = 7: worst change 0.0000; A f64: 0.000 (λ_T 0.5000, bias NaN)"
    );
    // Completeness: every orbit once at each precision, no other run, at least one candidate.
    let orbits = [synthetic_orbit("A"), synthetic_orbit("B")];
    let full = |extra: &[(&'static str, Precision, f64, f64)]| {
        let mut runs = vec![
            ("A", F32, 0.5, 0.0),
            ("A", F64, 0.5, 0.0),
            ("B", F32, 0.5, 0.0),
            ("B", F64, 0.5, 0.0),
        ];
        runs.extend_from_slice(extra);
        syn(1e-3, 1, &runs)
    };
    assert!(report_complete(&[full(&[])], &orbits));
    assert!(
        !report_complete(&[], &orbits),
        "no candidate reads complete"
    );
    assert!(
        !report_complete(&[full(&[]), full(&[("Z", F32, 0.5, 0.0)])], &orbits),
        "an extra run reads complete"
    );
    assert!(
        !report_complete(&[full(&[]).without("B")], &orbits),
        "an orbit left out reads complete"
    );
    let mut twice = full(&[]);
    twice.runs[1].1 = F32;
    assert!(
        !report_complete(&[twice], &orbits),
        "a run twice, another missing, reads complete"
    );
}

#[test]
fn eight_orbit_rules() {
    check_eight_orbit_rules(bias);
}

negative_control!(
    eight_orbit_rules,
    "a bias without the 1/T floor must fail",
    expected = "the bias of",
    check_eight_orbit_rules(|l, r, _| (l - r) / r.abs())
);

// ── eight_orbit_provisional ─────────────────────────────────────────────────────────────────────────────────────

fn check_provisional(worst: fn(&[Candidate]) -> Option<f64>) {
    use Precision::F64;
    let at = |delta_0, n_renorm, change| syn(delta_0, n_renorm, &[("A", F64, 0.5, change)]);
    let near = [
        at(1e-3, PROPOSED_N_RENORM, 0.125),
        at(PROPOSED_DELTA_0, 100, 0.25),
        at(PROPOSED_DELTA_0, 1000, 0.375),
    ];
    assert_eq!(
        worst(&near),
        None,
        "a candidate that is the provisional values in one coordinate alone is read as them"
    );
    let mut with = near.to_vec();
    with.push(at(PROPOSED_DELTA_0, PROPOSED_N_RENORM, 0.5));
    let got = worst(&with).expect("the provisional values are in the sweep");
    assert!(
        (got - 0.5).abs() < 1e-12,
        "the provisional values' worst change is {got}, not 0.5"
    );
    assert_eq!(worst(&[]), None);
}

#[test]
fn eight_orbit_provisional() {
    check_provisional(provisional_worst);
}

negative_control!(
    eight_orbit_provisional,
    "the provisional values matched on ‖δ₀‖ alone must fail",
    expected = "in one coordinate alone",
    check_provisional(|c| {
        c.iter()
            .find(|c| c.delta_0 == PROPOSED_DELTA_0)
            .map(Candidate::worst)
    })
);

// ── eight_orbit_gate ────────────────────────────────────────────────────────────────────────────────────────────

fn check_eight_orbit_gate(expect_control: Verdict) {
    use Precision::{F32, F64};
    let config = calibration_config();
    let orbits = [
        synthetic_orbit("A"),
        synthetic_orbit("B"),
        synthetic_orbit("K"),
    ];
    // Every orbit at both precisions, each run's four λ_T `lambdas`.
    let every = |delta_0: f64, lambdas: [f64; 4]| Candidate {
        delta_0,
        n_renorm: 1,
        runs: ["A", "B", "K"]
            .iter()
            .flat_map(|&o| [F32, F64].map(|p| (o, p, t10(lambdas, 50.0))))
            .collect(),
    };
    let grid = || {
        vec![
            every(REFERENCE_DELTA_0, [0.5; 4]),
            every(1e-3, [0.5, 0.625, 0.5, 0.5]),
            every(1e-2, [f64::INFINITY; 4]),
        ]
    };
    // The candidates: the sweep's, with a named orbit's runs left out; an unknown name refused before the sweep runs.
    let ran = std::cell::Cell::new(0);
    let sweep = |o: &[Orbit]| {
        ran.set(ran.get() + 1);
        assert_eq!(o.len(), 3);
        grid()
    };
    assert_eq!(
        eight_orbit_candidates(&orbits, None, sweep).expect("runs"),
        grid()
    );
    let fewer = eight_orbit_candidates(&orbits, Some("B"), sweep).expect("runs");
    assert_eq!(
        fewer,
        grid().iter().map(|c| c.without("B")).collect::<Vec<_>>()
    );
    assert_eq!(ran.get(), 2);
    assert!(eight_orbit_candidates(&orbits, Some("pluto"), sweep).is_err());
    assert_eq!(ran.get(), 2, "the sweep ran for an unknown orbit");
    // The report: complete, its regions the chaotic orbits, a spread line and a run line per candidate, the non-finite
    // count and a leave-one-out line per chaotic orbit; the rule's pick beside the unchanged proposal.
    let report =
        eight_orbit_report(&config, "t", "d", &grid(), &orbits, &["A", "B"]).expect("runs");
    let regions = report.check().expect("a complete report");
    assert_eq!(report.verdict, Verdict::Pass);
    assert_eq!(regions.count, RegionCount::Recorded(2));
    assert_eq!(report.scatter.len(), 2 * 3);
    assert_eq!(
        report.negative_results,
        [
            "1 of 3 candidates gave a non-finite λ_T somewhere (the f32 shadow's δ lost to rounding)",
            "without A: the rule picks ‖δ₀‖ = 1e-6, n_renorm = 1, which is the full set's pick",
            "without B: the rule picks ‖δ₀‖ = 1e-6, n_renorm = 1, which is the full set's pick",
        ]
    );
    assert_eq!(report.scatter[..3], spread_lines(&grid(), &["A", "B"])[..]);
    assert_eq!(report.scatter[3..], run_lines(&grid())[..]);
    assert_eq!(
        report.description,
        format!(
            "d: reported, not gated (R-649; the verdict is the report's completeness alone). Over the 3 starting \
             conditions the rule picks ‖δ₀‖ = 1e-6, n_renorm = 1 (worst change 0.0000), tolerance 0: the re-run's \
             proposal from its evidence, in this shadow form. In force, provisional (R-976): ‖δ₀‖ = 3e-3, n_renorm = \
             300, tolerance {PROPOSED_TOLERANCE}, not in this sweep; the owner confirms REQ-INT-076's values at the M3 \
             gate. Bias is against f64 at ‖δ₀‖ = 1e-6 at the same n_renorm. Proposed (R-976): min_regions ≥ 6, the \
             chaotic orbits, for the owner at the M3 gate; its evidence, this report's 2 regions and the pick with \
             each left out (its negative results)"
        )
    );
    assert_eq!(PROPOSED_MIN_REGIONS, 6);
    // The values in force, provisional (R-976), measured where the sweep ran them: finite, and not finite.
    for (lambdas, said) in [
        ([0.5, 0.75, 0.5, 0.5], "worst change here 0.5000;"),
        ([f64::INFINITY; 4], "worst change here not finite;"),
    ] {
        let mut ran = grid();
        ran.push(Candidate {
            delta_0: PROPOSED_DELTA_0,
            n_renorm: PROPOSED_N_RENORM,
            ..every(PROPOSED_DELTA_0, lambdas)
        });
        let with = eight_orbit_report(&config, "t", "d", &ran, &orbits, &["A", "B"]).expect("runs");
        assert!(
            with.description
                .contains(&format!("tolerance {PROPOSED_TOLERANCE}, {said}")),
            "the provisional values' result is not `{said}`: {}",
            with.description
        );
    }
    // Its control: an orbit's runs left out, which the completeness check tells apart.
    let control =
        eight_orbit_report(&config, "t", "d", &fewer, &orbits, &["A", "B"]).expect("runs");
    assert_eq!(
        control.verdict, expect_control,
        "the control is not told apart"
    );
    assert!(eight_orbit_report(&config, "t", "d", &[], &orbits, &["A"]).is_err());
    // The gate refuses an unknown orbit to leave out, before it sweeps.
    let refused = BenettinCalibrationGate.run(
        &config,
        "t",
        &serde_json::json!({ "kind": "eight-orbits", "description": "t", "drop": "pluto" }),
    );
    assert!(refused.is_err());
    // And an unknown shadow form, before it sweeps.
    let refused = BenettinCalibrationGate.run(
        &config,
        "t",
        &serde_json::json!({ "kind": "eight-orbits", "description": "t", "form": "displacement" }),
    );
    assert!(refused.is_err_and(|e| e.contains("form `displacement`")));
}

#[test]
fn eight_orbit_gate() {
    check_eight_orbit_gate(Verdict::Fail);
}

negative_control!(
    eight_orbit_gate,
    "a report that cannot tell an orbit left out must fail",
    expected = "the control is not told apart",
    check_eight_orbit_gate(Verdict::Pass)
);
