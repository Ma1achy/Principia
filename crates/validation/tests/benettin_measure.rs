//! The Benettin measurements and their gates (`validation::measure::benettin`; TASK-M3-14):
//! - `lagrange_known_exponent`: Routh's exponent of Lagrange's orbit, `½·√(√(27β) − 1)·ω` where `27β > 1` and 0 where
//!   not, at the equal-mass, `(0.5, 0.3, 0.2)` and `(0.98, 0.01, 0.01)` masses, against the roots of the rotating
//!   frame's characteristic equation (computed independently, to f64 rounding); the orbit's canonical `ω`.
//! - `test10_change`: dd test 10's change is the largest of the three variants' differences over `max(|λ_T|, 1/T)`,
//!   infinite where a read is not finite.
//! - `calibration_rule`: the rule picks the smallest worst change, ties at two significant digits going to the larger
//!   `n_renorm`, then the larger `‖δ₀‖`; the tolerance is the smallest one-digit value at or above twice it.
//! - `calibration_gate`: the gate's `test10` inputs pass at the proposed values on Lagrange's orbit and fail its control,
//!   and refuse an unknown orbit or precision; `sweep` passes only where the rule picks the proposal.
//! - `orbit_fixtures`: the orbits are what they are named: the Kepler-embedded pair's eccentricity 1/2 and its light
//!   third body's circular orbit, Burrau's (3, 4, 5) triangle at rest, Lagrange's equilateral triangle in rigid
//!   rotation at `ω² = M/a³`, all in the sim's units (`M = 1`, `I = 1`, CoM at rest at the origin).
//! - `shadow_form_gate`: its controls give their expected outcomes, and its measurement covers the periodic set at both
//!   `‖δ₀‖`s with every value finite.
//! - `eight_orbit_set`: the eight-orbit report's chaotic orbits are the Pythagorean triangles at rest their names give,
//!   Burrau first, and its eight starting conditions are the calibration orbits then the five others.
//! - `eight_orbit_rules`: the bias against the reference, the reference read, the spread, the pick with an orbit left
//!   out, the report's lines and its completeness, on synthetic candidates.
//! - `eight_orbit_gate`: the report's candidates leave out the orbit a control names and refuse an unknown one before
//!   the sweep runs; the report passes complete, its regions the chaotic orbits, gives the rule's pick beside the
//!   unchanged proposal, and fails with an orbit left out; on synthetic candidates.

use std::path::Path;

use kernel::decode::Ic;
use validation::gate::Gate;
use validation::gate::RegionCount;
use validation::gate::{self, GateSpec, MeasurementGate, Verdict};
use validation::measure::benettin::{
    bias, burrau, calibration_orbit, calibration_orbits, chaotic_orbits, chaotic_spread,
    eight_orbit_candidates, eight_orbit_report, eight_orbits, figure_eight, finite_time_gaps,
    kepler_embedded, lagrange, lagrange_exponent, lambda, leave_one_out_lines, periodic_set,
    pythagorean, reference, report_complete, routh_beta, run_lines, select, select_without, spread,
    spread_lines, sweep_confirms, tolerance, BenettinCalibrationGate, Candidate, FormRow, GapRow,
    Orbit, Precision, ShadowFormGate, Test10, GAP_HORIZONS, LAGRANGE_HORIZON, PROPOSED_DELTA_0,
    PROPOSED_N_RENORM, PROPOSED_TOLERANCE, REFERENCE_DELTA_0,
};
use validation::negative_control;

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

fn check_form_controls(doubled: f64) {
    let GateSpec::Measurement(config) = gate::spec(root(), "shadow-form").expect("gate.json")
    else {
        panic!("shadow-form is not a measurement");
    };
    let control = |v: serde_json::Value| ShadowFormGate.control(&config, "t", &v);
    let same = control(serde_json::json!({ "kind": "compare", "description": "s", "scale": 1.0 }))
        .expect("runs");
    assert_eq!(same.verdict, Verdict::Pass);
    let other =
        control(serde_json::json!({ "kind": "compare", "description": "d", "scale": doubled }))
            .expect("runs");
    assert_eq!(
        other.verdict,
        Verdict::Fail,
        "the comparison cannot tell two reads apart"
    );
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
    assert!(ShadowFormGate.flags(root(), &["--measure"]).is_err());
    assert!(control(serde_json::json!({ "kind": "compare", "description": "x" })).is_err());
}

#[test]
fn shadow_form_gate() {
    check_form_controls(2.0);
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
    // A row with any value not finite reads not finite.
    let row = FormRow {
        orbit: "r",
        known: 0.0,
        f64_absolute: 0.1,
        f32_absolute: 0.2,
        f32_displacement: 0.3,
    };
    assert!(row.finite());
    for bad in [
        FormRow {
            known: f64::NAN,
            ..row
        },
        FormRow {
            f64_absolute: f64::INFINITY,
            ..row
        },
        FormRow {
            f32_absolute: f64::NAN,
            ..row
        },
        FormRow {
            f32_displacement: f64::NEG_INFINITY,
            ..row
        },
    ] {
        assert!(!bad.finite(), "{bad:?} reads finite");
    }
}

negative_control!(
    shadow_form_gate,
    "a control at the same ‖δ₀‖ must not be told apart",
    expected = "the comparison cannot tell two reads apart",
    check_form_controls(1.0)
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
             conditions the rule picks ‖δ₀‖ = 1e-6, n_renorm = 1 (worst change 0.0000), tolerance 0; the proposal \
             stays ‖δ₀‖ = 3e-3, n_renorm = 300, tolerance {PROPOSED_TOLERANCE}, for the owner at the M3 gate \
             (REQ-INT-076). Bias is against f64 at ‖δ₀‖ = 1e-6 at the same n_renorm"
        )
    );
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
